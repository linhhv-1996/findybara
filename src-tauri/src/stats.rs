use std::fs;
use std::path::Path;
use serde::Serialize;
use walkdir::WalkDir;
use tauri::{Emitter, Runtime, WebviewWindow};
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::config;

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

// Nhận Vec<String> thay vì String
pub fn spawn_analyze_and_emit<R: Runtime>(window: WebviewWindow<R>, paths: Vec<String>) {
    if paths.is_empty() {
        let _ = window.emit("finder-state", FinderState { ctx: "none".into(), data: None });
        return;
    }

    let task_id = CURRENT_TASK.fetch_add(1, Ordering::SeqCst) + 1;

    // TRƯỜNG HỢP 1 ITEM ĐƯỢC CHỌN (Hoặc thư mục gốc)
    if paths.len() == 1 {
        let path_str = paths[0].clone();
        let path = Path::new(&path_str);
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let is_dir = path.is_dir();
        let cfg = config::get_config(&path_str, is_dir);

        if !is_dir {
            let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let state = FinderState {
                ctx: "file".into(),
                data: Some(CtxData {
                    name, ext: cfg.ext_label, meta: format_size(size),
                    color: cfg.color, actions: cfg.actions,
                }),
            };
            let _ = window.emit("finder-state", state);
            return;
        }

        let loading_state = FinderState {
            ctx: "folder".into(),
            data: Some(CtxData {
                name: name.clone(), ext: cfg.ext_label.clone(), meta: "Calculating...".into(),
                color: cfg.color.clone(), actions: cfg.actions.clone(),
            }),
        };
        let _ = window.emit("finder-state", loading_state);

        std::thread::spawn(move || {
            let mut file_count = 0; let mut dir_count = 0; let mut total_size: u64 = 0;
            for entry in WalkDir::new(&path_str).min_depth(1).into_iter().filter_map(|e| e.ok()) {
                if CURRENT_TASK.load(Ordering::Relaxed) != task_id { return; }
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() { total_size += meta.len(); file_count += 1; } 
                    else if meta.is_dir() { dir_count += 1; }
                }
            }
            if CURRENT_TASK.load(Ordering::SeqCst) == task_id {
                let meta_text = format!("{} · {} folders, {} files", format_size(total_size), dir_count, file_count);
                let _ = window.emit("finder-state", FinderState {
                    ctx: "folder".into(),
                    data: Some(CtxData { name, ext: cfg.ext_label, meta: meta_text, color: cfg.color, actions: cfg.actions }),
                });
            }
        });
        
    } else {
        // TRƯỜNG HỢP NHIỀU ITEM ĐƯỢC CHỌN (MULTI-SELECTION)
        let cfg = config::get_multi_config();
        let count = paths.len();

        let loading_state = FinderState {
            ctx: "multi".into(),
            data: Some(CtxData {
                name: format!("{} items selected", count),
                ext: cfg.ext_label.clone(),
                meta: "Calculating total size...".into(),
                color: cfg.color.clone(),
                actions: cfg.actions.clone(),
            }),
        };
        let _ = window.emit("finder-state", loading_state);

        std::thread::spawn(move || {
            let mut total_size: u64 = 0;
            
            for path_str in paths {
                if CURRENT_TASK.load(Ordering::Relaxed) != task_id { return; }
                let path = Path::new(&path_str);
                
                if path.is_file() {
                    total_size += fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                } else if path.is_dir() {
                    for entry in WalkDir::new(&path_str).min_depth(1).into_iter().filter_map(|e| e.ok()) {
                        if CURRENT_TASK.load(Ordering::Relaxed) != task_id { return; }
                        if let Ok(meta) = entry.metadata() {
                            if meta.is_file() { total_size += meta.len(); }
                        }
                    }
                }
            }

            if CURRENT_TASK.load(Ordering::SeqCst) == task_id {
                let _ = window.emit("finder-state", FinderState {
                    ctx: "multi".into(),
                    data: Some(CtxData {
                        name: format!("{} items selected", count),
                        ext: cfg.ext_label,
                        meta: format!("Total Size: {}", format_size(total_size)),
                        color: cfg.color,
                        actions: cfg.actions,
                    }),
                });
            }
        });
    }
}

fn format_size(bytes: u64) -> String {
    let kb = 1024.0; let mb = kb * 1024.0; let gb = mb * 1024.0;
    let b = bytes as f64;
    if b >= gb { format!("{:.1} GB", b / gb) }
    else if b >= mb { format!("{:.1} MB", b / mb) }
    else if b >= kb { format!("{:.1} KB", b / kb) }
    else { format!("{} B", bytes) }
}
