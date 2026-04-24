use crate::config;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tauri::{Emitter, Runtime, WebviewWindow};
use walkdir::{DirEntry, WalkDir};

// Biến toàn cục để theo dõi Task ID, giúp hủy các luồng đếm cũ nếu user chuyển qua file/folder khác
static CURRENT_TASK: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Serialize)]
pub struct FinderState {
    pub ctx: String,
    pub data: Option<CtxData>,
}

#[derive(Clone, Serialize)]
pub struct CtxData {
    pub name: String,
    pub ext: String,
    pub meta: String,
    pub color: String,
    pub actions: Vec<String>,
    pub path: String,
    pub folder_summary: String,
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub fn spawn_analyze_and_emit<R: Runtime>(window: WebviewWindow<R>, paths: Vec<String>) {
    if paths.is_empty() {
        let _ = window.emit(
            "finder-state",
            FinderState {
                ctx: "none".into(),
                data: None,
            },
        );
        return;
    }

    // Tăng Task ID lên 1 mỗi khi có yêu cầu phân tích mới
    let task_id = CURRENT_TASK.fetch_add(1, Ordering::SeqCst) + 1;

    if paths.len() == 1 {
        let path_str = paths[0].clone();
        let path = Path::new(&path_str);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let is_dir = path.is_dir();
        let cfg = config::get_config(&path_str, is_dir);

        // 1. NẾU LÀ FILE -> Tính toán nhanh và trả về luôn
        if !is_dir {
            let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let _ = window.emit(
                "finder-state",
                FinderState {
                    ctx: "file".into(),
                    data: Some(CtxData {
                        name,
                        ext: cfg.ext_label,
                        meta: format_size(size),
                        color: cfg.color,
                        actions: cfg.actions,
                        path: path_str,
                        folder_summary: "".into(),
                    }),
                },
            );
            return;
        }

        // 2. NẾU LÀ FOLDER -> Bắn State báo hiệu trước, sau đó chạy ngầm
        let thread_path = path_str.clone();
        let thread_name = name.clone();
        let thread_cfg_ext = cfg.ext_label.clone();
        let thread_cfg_color = cfg.color.clone();
        let thread_cfg_actions = cfg.actions.clone();

        // Bắn tín hiệu "đang chuẩn bị" để UI ko bị đơ
        let _ = window.emit(
            "finder-state",
            FinderState {
                ctx: "folder".into(),
                data: Some(CtxData {
                    name: thread_name.clone(),
                    ext: thread_cfg_ext.clone(),
                    meta: "Scanning items...".into(),
                    color: thread_cfg_color.clone(),
                    actions: thread_cfg_actions.clone(),
                    path: thread_path.clone(),
                    folder_summary: "".into(),
                }),
            },
        );

        std::thread::spawn(move || {
            let mut file_count = 0;
            let mut dir_count = 0;
            let mut total_size: u64 = 0;
            let mut ext_map: HashMap<String, usize> = HashMap::new();

            let walker = WalkDir::new(&thread_path)
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| !is_hidden(e))
                .filter_map(|e| e.ok());

            let mut last_emit = Instant::now();

            for entry in walker {
                // Kiểm tra xem User có đang chọn qua mục khác chưa, nếu có -> Hủy Task này
                if CURRENT_TASK.load(Ordering::Relaxed) != task_id {
                    return;
                }

                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total_size += meta.len();
                        if entry.depth() == 1 {
                            file_count += 1; // Chỉ đếm file ở ngoài cùng
                        }
                        let ext = entry
                            .path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("no-ext")
                            .to_lowercase();
                        *ext_map.entry(ext).or_insert(0) += 1;
                    } else if meta.is_dir() && entry.depth() == 1 {
                        dir_count += 1; // Chỉ đếm folder ở ngoài cùng
                    }
                }

                // LIVE UPDATE CỰC MƯỢT: Bắn kết quả lên UI mỗi 100ms trong lúc quét
                if last_emit.elapsed() > Duration::from_millis(100) {
                    let _ = window.emit(
                        "finder-state",
                        FinderState {
                            ctx: "folder".into(),
                            data: Some(CtxData {
                                name: thread_name.clone(),
                                ext: thread_cfg_ext.clone(),
                                meta: format!(
                                    "Scanning... {} folders, {} files ({}...)",
                                    dir_count,
                                    file_count,
                                    format_size(total_size)
                                ),
                                color: thread_cfg_color.clone(),
                                actions: thread_cfg_actions.clone(),
                                path: thread_path.clone(),
                                folder_summary: String::new(), // Chưa xong nên chưa tóm tắt ext
                            }),
                        },
                    );
                    last_emit = Instant::now();
                }
            }

            // HOÀN THÀNH: Build summary và bắn final state lên UI
            let mut summary = String::new();
            for (ext, count) in ext_map {
                summary.push_str(&format!("{} .{} files, ", count, ext));
            }

            if CURRENT_TASK.load(Ordering::SeqCst) == task_id {
                let _ = window.emit(
                    "finder-state",
                    FinderState {
                        ctx: "folder".into(),
                        data: Some(CtxData {
                            name: thread_name,
                            ext: thread_cfg_ext,
                            meta: format!(
                                "{} · {} folders, {} files",
                                format_size(total_size),
                                dir_count,
                                file_count
                            ),
                            color: thread_cfg_color,
                            actions: thread_cfg_actions,
                            path: thread_path,
                            folder_summary: summary,
                        }),
                    },
                );
            }
        });
    } else {
        // 3. XỬ LÝ KHI CHỌN NHIỀU FILE (Multi-selection)
        let count = paths.len();
        let cfg = config::get_multi_config();
        
        let mut total_size: u64 = 0;
        for p in &paths {
            if let Ok(meta) = fs::metadata(p) {
                if meta.is_file() {
                    total_size += meta.len();
                }
            }
        }

        let meta_text = if total_size > 0 {
            format!("{} · {} items selected", format_size(total_size), count)
        } else {
            format!("{} items selected", count)
        };

        let _ = window.emit(
            "finder-state",
            FinderState {
                ctx: "multi".into(),
                data: Some(CtxData {
                    name: format!("{} items", count),
                    ext: cfg.ext_label,
                    meta: meta_text,
                    color: cfg.color,
                    actions: cfg.actions,
                    path: paths.join("\n"), 
                    folder_summary: "".into(),
                }),
            },
        );
    }
}

// Bổ sung thêm GB vào format_size cho chuẩn với các thư mục to
fn format_size(bytes: u64) -> String {
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let b = bytes as f64;
    
    if b >= gb {
        format!("{:.2} GB", b / gb)
    } else if b >= mb {
        format!("{:.1} MB", b / mb)
    } else if b >= kb {
        format!("{:.1} KB", b / kb)
    } else {
        format!("{} B", bytes)
    }
}
