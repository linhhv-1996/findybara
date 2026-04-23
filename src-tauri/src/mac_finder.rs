use active_win_pos_rs::get_active_window;
use objc::{class, msg_send, sel, sel_impl};
use objc::runtime::Object;
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Debug, Clone, PartialEq)]
pub struct FinderBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct ActiveWindow {
    pub app_name: String,
    pub process_id: u64,
    pub title: String,
    pub bounds: FinderBounds,
}

// 1. DÙNG ACTIVE_WIN_POS ĐỂ LẤY TOẠ ĐỘ SIÊU NHANH
#[cfg(target_os = "macos")]
pub fn get_frontmost_window() -> Option<ActiveWindow> {
    if let Ok(window) = get_active_window() {
        return Some(ActiveWindow {
            app_name: window.app_name,
            process_id: window.process_id,
            title: window.title,
            bounds: FinderBounds {
                x: window.position.x,
                y: window.position.y,
                width: window.position.width,
                height: window.position.height,
            }
        });
    }
    None
}

// 2. DÙNG APPLESCRIPT ĐỂ CHECK XEM CÓ PHẢI INFO/TRASH KHÔNG (VÀ LẤY PATH)
#[cfg(target_os = "macos")]
pub fn get_valid_finder_path() -> Option<String> {
    let script_src = "
        tell application \"Finder\"
            if (count of windows) = 0 then return \"\"
            if (class of window 1 is not finder window) then return \"\"
            try
                set p to POSIX path of (target of window 1 as alias)
                if p contains \".Trash\" then return \"\"
                return p
            on error
                return \"\"
            end try
        end tell
    ";

    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), alloc];
        let pool: *mut Object = msg_send![pool, init];

        let ns_string: *mut Object = msg_send![class!(NSString), alloc];
        let script_ns: *mut Object = msg_send![ns_string, initWithBytes:script_src.as_ptr() length:script_src.len() encoding:4];
        let apple_script: *mut Object = msg_send![class!(NSAppleScript), alloc];
        let apple_script: *mut Object = msg_send![apple_script, initWithSource:script_ns];

        let mut error: *mut Object = std::ptr::null_mut();
        let result_descriptor: *mut Object = msg_send![apple_script, executeAndReturnError:&mut error];

        let mut res = None;
        if !result_descriptor.is_null() {
            let res_string: *mut Object = msg_send![result_descriptor, stringValue];
            if !res_string.is_null() {
                let utf8: *const c_char = msg_send![res_string, UTF8String];
                let s = CStr::from_ptr(utf8).to_string_lossy().to_string();
                if !s.is_empty() {
                    res = Some(s);
                }
            }
        }
        let _: () = msg_send![pool, release];
        res
    }
}
