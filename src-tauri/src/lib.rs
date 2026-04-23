use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

mod mac_finder;
mod stats;
mod config;

// Import hàm get_frontmost_window mới
use mac_finder::{get_frontmost_window, get_finder_path, FinderBounds};

fn update_window_pos_with_bounds<R: Runtime>(window: &tauri::WebviewWindow<R>, bounds: &FinderBounds) {
    let ui_height = 72.0;
    let mut target_y = bounds.y + bounds.height;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let sf = monitor.scale_factor();
        let screen_bottom = (monitor.position().y as f64 + monitor.size().height as f64) / sf;
        if target_y + ui_height > screen_bottom { target_y = screen_bottom - ui_height; }
    }

    let _ = window.set_size(tauri::Size::Logical(LogicalSize::new(bounds.width, ui_height)));
    let _ = window.set_position(tauri::Position::Logical(LogicalPosition::new(bounds.x, target_y)));
}

#[tauri::command]
fn show_findybara(app: AppHandle) {
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = app_clone.get_webview_window("main") {
            // Chỉ show khi đang ở Finder
            if let Some(win) = get_frontmost_window() {
                if win.app_name == "Finder" {
                    update_window_pos_with_bounds(&window, &win.bounds);
                    
                    if let Some(path) = get_finder_path() {
                        stats::spawn_analyze_and_emit(window.clone(), path);
                    }
                    
                    let _ = window.show();
                    let _ = window.set_focus(); // Khi gọi dòng này, app sẽ lấy focus
                    let _ = window.set_always_on_top(true);
                }
            }
        }
    });
}

#[tauri::command]
fn hide_findybara(app: AppHandle) {
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
                    show_findybara(app.clone());
                }
            }).build())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

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
                        show_findybara(tray.app_handle().clone());
                    }
                }).build(app)?;

            let app_handle = app.handle().clone();
            
            // Lấy ID tiến trình của chính app này
            let my_pid = std::process::id() as u64;

            std::thread::spawn(move || {
                let mut last_path = String::new();
                let mut last_bounds = FinderBounds { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
                let mut tick_counter = 0;

                loop {
                    std::thread::sleep(Duration::from_millis(16));

                    if let Some(window) = app_handle.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            
                            if let Some(win) = get_frontmost_window() {
                                // TRƯỜNG HỢP 1: Đang trỏ vào Finder -> Update vị trí & Data
                                if win.app_name == "Finder" {
                                    if win.bounds != last_bounds {
                                        let w = window.clone();
                                        let b = win.bounds.clone();
                                        let _ = app_handle.run_on_main_thread(move || { update_window_pos_with_bounds(&w, &b); });
                                        last_bounds = win.bounds;
                                    }

                                    tick_counter += 1;
                                    if tick_counter >= 30 {
                                        tick_counter = 0;
                                        if let Some(path) = get_finder_path() {
                                            if path != last_path {
                                                stats::spawn_analyze_and_emit(window.clone(), path.clone());
                                                last_path = path;
                                            }
                                        }
                                    }
                                } 
                                // TRƯỜNG HỢP 2: Đang trỏ vào chính App của mình -> Đứng im, không làm gì cả
                                else if win.process_id == my_pid {
                                    // Không hide, để yên cho user thao tác với UI
                                } 
                                // TRƯỜNG HỢP 3: Bấm sang app khác (Chrome, Zalo...) -> Ẩn app đi
                                else {
                                    let w = window.clone();
                                    let _ = app_handle.run_on_main_thread(move || { let _ = w.hide(); });
                                }
                            } else {
                                let w = window.clone();
                                let _ = app_handle.run_on_main_thread(move || { let _ = w.hide(); });
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
