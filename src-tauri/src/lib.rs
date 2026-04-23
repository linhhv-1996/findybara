use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

mod mac_finder;
mod stats;
mod config;

use mac_finder::{get_frontmost_window, get_finder_state_paths, FinderBounds};

// State để lưu trạng thái "Công tắc tổng" của ứng dụng
struct AppState {
    is_enabled: AtomicBool,
}

// Cập nhật vị trí cửa sổ dựa trên toạ độ của Finder
fn update_window_pos_with_bounds<R: Runtime>(window: &tauri::WebviewWindow<R>, bounds: &FinderBounds) {
    let ui_height = 42.0;
    let mut target_y = bounds.y + bounds.height;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let sf = monitor.scale_factor();
        let screen_bottom = (monitor.position().y as f64 + monitor.size().height as f64) / sf;
        if target_y + ui_height > screen_bottom { target_y = screen_bottom - ui_height; }
    }

    let window_width = bounds.width.max(380.0);

    // NÂNG CẤP (Tuỳ chọn): Nếu cửa sổ Tauri rộng hơn Finder, hãy căn giữa nó so với Finder
    let mut target_x = bounds.x;
    if window_width > bounds.width {
        target_x = bounds.x - (window_width - bounds.width) / 2.0;
    }

    // LỖI Ở 2 DÒNG NÀY: Thay bounds.width thành window_width, và bounds.x thành target_x
    let _ = window.set_size(tauri::Size::Logical(LogicalSize::new(window_width, ui_height)));
    let _ = window.set_position(tauri::Position::Logical(LogicalPosition::new(target_x, target_y)));
}


#[tauri::command]
fn show_findybara(app: AppHandle) {
    // Bật công tắc hoạt động
    let state = app.state::<AppState>();
    state.is_enabled.store(true, Ordering::Relaxed);

    std::thread::spawn(move || {
        let paths_opt = get_finder_state_paths(); // Lấy danh sách selection hoặc target path
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
    // Tắt công tắc hoạt động (ngừng tracking)
    let state = app.state::<AppState>();
    state.is_enabled.store(false, Ordering::Relaxed);

    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = app_clone.get_webview_window("main") { 
            let _ = window.hide(); 
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, shortcut, event| {
                if shortcut.matches(Modifiers::SHIFT, Code::Space) && event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    let state = app.state::<AppState>();
                    let is_enabled = state.is_enabled.load(Ordering::Relaxed);
                    
                    if let Some(window) = app.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(false);
                        
                        // Toggle logic: Nếu đang bật và hiện -> Tắt, ngược lại -> Bật
                        if is_enabled && is_visible {
                            hide_findybara(app.clone());
                        } else {
                            show_findybara(app.clone());
                        }
                    }
                }
            }).build())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Khởi tạo state: Mặc định bật khi mở app
            app.manage(AppState {
                is_enabled: AtomicBool::new(true),
            });

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
                        let state = app_handle.state::<AppState>();
                        let is_enabled = state.is_enabled.load(Ordering::Relaxed);
                        
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_enabled && is_visible { 
                                hide_findybara(app_handle.clone()); 
                            } else { 
                                show_findybara(app_handle.clone()); 
                            }
                        }
                    }
                }).build(app)?;
                
            let app_handle = app.handle().clone();

            // Luồng kiểm tra Finder ngay khi khởi động
            {
                let app_startup = app_handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(300));
                    if let Some(paths) = get_finder_state_paths() {
                        if let Some(win) = get_frontmost_window() {
                            if win.app_name == "Finder" && win.bounds.width >= 300.0 && win.bounds.height >= 200.0 {
                                let bounds = win.bounds.clone();
                                if let Some(window) = app_startup.get_webview_window("main") {
                                    let _ = app_startup.run_on_main_thread(move || {
                                        update_window_pos_with_bounds(&window, &bounds);
                                        stats::spawn_analyze_and_emit(window.clone(), paths);
                                        let _ = window.show();
                                        let _ = window.set_always_on_top(true);
                                    });
                                }
                            }
                        }
                    }
                });
            }

            // VÒNG LẶP TRACKING CHÍNH (60fps cho toạ độ, 500ms cho Path/Selection)
            std::thread::spawn(move || {
                let mut last_paths: Vec<String> = vec![];
                let mut last_bounds = FinderBounds { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
                let mut last_title = String::new(); 
                let mut is_valid_finder = false;
                let mut last_path_check = Instant::now();
                let my_pid = std::process::id() as u64;

                loop {
                    std::thread::sleep(Duration::from_millis(16)); 

                    if let Some(window) = app_handle.get_webview_window("main") {
                        
                        let is_enabled = app_handle.state::<AppState>().is_enabled.load(Ordering::Relaxed);
                        if !is_enabled {
                            if window.is_visible().unwrap_or(false) {
                                let w = window.clone();
                                let _ = app_handle.run_on_main_thread(move || { let _ = w.hide(); });
                            }
                            last_paths.clear();
                            last_title.clear();
                            last_bounds = FinderBounds { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
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

                        // Cập nhật dữ liệu file khi Title đổi hoặc sau mỗi 500ms
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
                                    let _ = app_handle.run_on_main_thread(move || { let _ = w.show(); });
                                }
                            } else {
                                is_valid_finder = false;
                                let w = window.clone();
                                let _ = app_handle.run_on_main_thread(move || { let _ = w.hide(); });
                            }
                        }

                        // Cập nhật vị trí cửa sổ theo Finder (real-time)
                        if let Some(win) = win_opt {
                            if win.process_id == my_pid { continue; }

                            if win.app_name == "Finder" && is_valid_finder {
                                if win.bounds.width < 300.0 || win.bounds.height < 200.0 { continue; }

                                if win.bounds != last_bounds {
                                    last_bounds = win.bounds.clone();
                                    let w = window.clone();
                                    let b = win.bounds.clone();
                                    let _ = app_handle.run_on_main_thread(move || { update_window_pos_with_bounds(&w, &b); });
                                }
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![show_findybara, hide_findybara])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
