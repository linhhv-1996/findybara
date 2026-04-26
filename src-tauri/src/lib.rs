use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, LogicalPosition, LogicalSize, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use std::sync::Arc;

mod ollama;
mod config;
mod mac_finder;
mod stats;

use mac_finder::{get_finder_state_paths, get_frontmost_window, FinderBounds};

struct AppState {
    is_enabled: AtomicBool,
}

struct AppOllamaState(Arc<ollama::OllamaManager>);

#[tauri::command]
fn set_ui_height(app: AppHandle, height: f64) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(scale_factor) = window.scale_factor() {
            if let Ok(current_size) = window.inner_size() {
                let logical_size = current_size.to_logical::<f64>(scale_factor);
                if let Ok(current_pos) = window.outer_position() {
                    let logical_pos = current_pos.to_logical::<f64>(scale_factor);
                    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                        logical_size.width,
                        height,
                    )));
                    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
                        logical_pos.x, logical_pos.y,
                    )));
                }
            }
        }
    }
}

// CẬP NHẬT: Chiều ngang khớp hoàn toàn với Finder
fn update_window_pos_with_bounds<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    bounds: &FinderBounds,
) {
    let window_width = bounds.width; // Giãn bằng Finder
    let target_x = bounds.x;

    let current_logical_height = window
        .inner_size()
        .unwrap_or_default()
        .to_logical::<f64>(window.scale_factor().unwrap_or(1.0))
        .height;

    let mut target_y = bounds.y + bounds.height;
    if let Ok(Some(monitor)) = window.current_monitor() {
        let sf = monitor.scale_factor();
        let screen_bottom = (monitor.position().y as f64 + monitor.size().height as f64) / sf;
        if target_y + current_logical_height > screen_bottom {
            target_y = screen_bottom - current_logical_height - 10.0;
        }
    }

    let _ = window.set_size(tauri::Size::Logical(LogicalSize::new(
        window_width,
        current_logical_height,
    )));
    
    let _ = window.set_position(tauri::Position::Logical(LogicalPosition::new(
        target_x, target_y,
    )));
}

#[tauri::command]
fn show_findybara(app: AppHandle) {
    let state = app.state::<AppState>();
    state.is_enabled.store(true, Ordering::Relaxed);
    std::thread::spawn(move || {
        let paths_opt = get_finder_state_paths();
        let app_clone = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = app_clone.get_webview_window("main") {
                if let Some(win) = get_frontmost_window() {
                    if win.app_name == "Finder" && win.bounds.width >= 300.0 && win.bounds.height >= 200.0 {
                        if let Some(paths) = paths_opt {
                            update_window_pos_with_bounds(&window, &win.bounds);
                            stats::spawn_analyze_and_emit(window.clone(), paths);
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.set_always_on_top(true);
                        }
                    }
                }
            }
        });
    });
}

#[tauri::command]
fn hide_findybara(app: AppHandle) {
    let state = app.state::<AppState>();
    state.is_enabled.store(false, Ordering::Relaxed);

    // 1. Create a clone of the app handle
    let app_clone = app.clone();
    
    let _ = app.run_on_main_thread(move || {
        // 2. Use the clone inside the move closure
        if let Some(window) = app_clone.get_webview_window("main") {
            let _ = window.hide();
        }
    });
}

#[tauri::command]
async fn ask_ai(
    query: String,
    path: String,
    ctx_name: String,       
    folder_summary: String, 
    state: tauri::State<'_, AppOllamaState>
) -> Result<String, String> {
    let full_prompt = format!(
        "Thông tin ngữ cảnh:\n- File/Thư mục: {}\n- Đường dẫn: {}\n- Chi tiết: {}\n\nCâu hỏi từ người dùng: {}",
        ctx_name, path, folder_summary, query
    );
    state.0.generate_text(&full_prompt).await.map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut.matches(Modifiers::SHIFT, Code::Space)
                        && event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                    {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                hide_findybara(app.clone());
                            } else {
                                show_findybara(app.clone());
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.manage(AppState { is_enabled: AtomicBool::new(true) });

            let show_shortcut = Shortcut::new(Some(Modifiers::SHIFT), Code::Space);
            let _ = app.global_shortcut().register(show_shortcut);

            let quit_i = MenuItem::with_id(app, "quit", "Quit Findybara", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| { if event.id == "quit" { app.exit(0); } })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app_handle = tray.app_handle();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) { hide_findybara(app_handle.clone()); }
                            else { show_findybara(app_handle.clone()); }
                        }
                    }
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            // Luồng theo dõi Finder (loop chính)
            let app_tracking = app_handle.clone(); 
            std::thread::spawn(move || {
                let mut last_paths: Vec<String> = vec![];
                let mut last_bounds = FinderBounds { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
                let mut last_title = String::new();
                let mut is_valid_finder = false;
                let mut last_path_check = Instant::now();
                let my_pid = std::process::id() as u64;

                loop {
                    std::thread::sleep(Duration::from_millis(16));
                    if let Some(window) = app_tracking.get_webview_window("main") {
                        let is_enabled = app_tracking.state::<AppState>().is_enabled.load(Ordering::Relaxed);
                        if !is_enabled {
                            if window.is_visible().unwrap_or(false) {
                                let w = window.clone();
                                let _ = app_tracking.run_on_main_thread(move || { let _ = w.hide(); });
                            }
                            continue;
                        }

                        let win_opt = get_frontmost_window();
                        let time_to_check = last_path_check.elapsed() >= Duration::from_millis(500);
                        let mut title_changed = false;

                        if let Some(win) = &win_opt {
                            if win.app_name == "Finder" && win.process_id != my_pid {
                                if win.title != last_title && win.bounds.width >= 300.0 && win.bounds.height >= 200.0 {
                                    title_changed = true;
                                    last_title = win.title.clone();
                                }
                            }
                        }

                        if time_to_check || title_changed {
                            last_path_check = Instant::now();
                            if let Some(paths) = get_finder_state_paths() {
                                is_valid_finder = true;
                                if paths != last_paths {
                                    stats::spawn_analyze_and_emit(window.clone(), paths.clone());
                                    last_paths = paths;
                                }
                                if !window.is_visible().unwrap_or(false) {
                                    let w = window.clone();
                                    let _ = app_tracking.run_on_main_thread(move || { let _ = w.show(); });
                                }
                            } else {
                                is_valid_finder = false;
                                let w = window.clone();
                                let _ = app_tracking.run_on_main_thread(move || { let _ = w.hide(); });
                            }
                        }

                        if let Some(win) = win_opt {
                            if win.process_id != my_pid && win.app_name == "Finder" && is_valid_finder {
                                if win.bounds.width >= 300.0 && win.bounds.height >= 200.0 && win.bounds != last_bounds {
                                    last_bounds = win.bounds.clone();
                                    let w = window.clone();
                                    let b = win.bounds.clone();
                                    let _ = app_tracking.run_on_main_thread(move || { update_window_pos_with_bounds(&w, &b); });
                                }
                            }
                        }
                    }
                }
            });

            let ollama_mgr = Arc::new(ollama::OllamaManager::new(11435, "findybara-model"));
            app.manage(AppOllamaState(ollama_mgr.clone()));

            // Khởi động Ollama Sidecar (giữ nguyên logic của bạn)
            let app_ollama = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let app_data_dir = app_ollama.path().app_data_dir().unwrap();
                let dest_model_path = app_data_dir.join("model.gguf");
                if let Err(e) = ollama_mgr.launch_sidecar(&app_ollama) { eprintln!("{}", e); return; }
                if let Err(e) = ollama_mgr.wait_for_server().await { eprintln!("{}", e); return; }
                let gguf_path = dest_model_path.to_str().unwrap_or_default();
                let _ = ollama_mgr.create_model_if_not_exists(&app_ollama, gguf_path).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![show_findybara, hide_findybara, ask_ai, set_ui_height])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
