use std::fs;
use std::path::Path;
use serde::Serialize;
use walkdir::WalkDir;
use tauri::{Emitter, Runtime, WebviewWindow};
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::config; // Import config mới

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
}

pub fn spawn_analyze_and_emit<R: Runtime>(window: WebviewWindow<R>, path_str: String) {
    if path_str.is_empty() {
        let _ = window.emit("finder-state", FinderState { ctx: "none".into(), data: None });
        return;
    }

    let task_id = CURRENT_TASK.fetch_add(1, Ordering::SeqCst) + 1;
    let path = Path::new(&path_str);
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let is_dir = path.is_dir();

    // 1. Lấy ngay cấu hình chuẩn từ config.rs (Màu và Actions sẽ cố định từ đây)
    let cfg = config::get_config(&path_str, is_dir);

    if !is_dir {
        // TRƯỜNG HỢP LÀ FILE: Tính size phát xong luôn, không cần loop
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let state = FinderState {
            ctx: "file".into(),
            data: Some(CtxData {
                name,
                ext: cfg.ext_label,
                meta: format_size(size),
                color: cfg.color, // Màu chuẩn ngay từ đầu
                actions: cfg.actions, // Action chuẩn ngay từ đầu
            }),
        };
        let _ = window.emit("finder-state", state);
        return;
    }

    // TRƯỜNG HỢP LÀ FOLDER: Hiện Loading nhưng giữ nguyên Màu và Actions
    let loading_state = FinderState {
        ctx: "folder".into(),
        data: Some(CtxData {
            name: name.clone(),
            ext: cfg.ext_label.clone(),
            meta: "Calculating...".into(),
            color: cfg.color.clone(), // KHÔNG dùng màu xám nữa, dùng luôn màu chuẩn
            actions: cfg.actions.clone(), // Hiện luôn actions, không bắt chờ
        }),
    };
    let _ = window.emit("finder-state", loading_state);

    // Chạy thread tính size đệ quy cho folder
    std::thread::spawn(move || {
        let mut file_count = 0;
        let mut dir_count = 0;
        let mut total_size: u64 = 0;

        for entry in WalkDir::new(&path_str).min_depth(1).into_iter().filter_map(|e| e.ok()) {
            if CURRENT_TASK.load(Ordering::Relaxed) != task_id { return; }

            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total_size += meta.len();
                    file_count += 1;
                } else if meta.is_dir() {
                    dir_count += 1;
                }
            }
        }

        if CURRENT_TASK.load(Ordering::SeqCst) == task_id {
            let meta_text = format!("{} · {} folders, {} files", format_size(total_size), dir_count, file_count);
            
            let final_state = FinderState {
                ctx: "folder".into(),
                data: Some(CtxData {
                    name,
                    ext: cfg.ext_label,
                    meta: meta_text,
                    color: cfg.color,
                    actions: cfg.actions,
                }),
            };
            let _ = window.emit("finder-state", final_state);
        }
    });
}

fn format_size(bytes: u64) -> String {
    let kb = 1024.0; let mb = kb * 1024.0; let gb = mb * 1024.0;
    let b = bytes as f64;
    if b >= gb { format!("{:.1} GB", b / gb) }
    else if b >= mb { format!("{:.1} MB", b / mb) }
    else if b >= kb { format!("{:.1} KB", b / kb) }
    else { format!("{} B", bytes) }
}
