use std::path::Path;

pub struct FileConfig {
    pub ext_label: String,
    pub color: String,
    pub actions: Vec<String>,
}

// Thêm cấu hình cho Multi-selection
pub fn get_multi_config() -> FileConfig {
    FileConfig {
        ext_label: "MULTI".into(),
        color: "#EAB308".into(), // Màu vàng cho việc chọn nhiều item
        actions: vec!["Copy Paths".into(), "Move items".into(), "Delete".into()],
    }
}

pub fn get_config(path_str: &str, is_dir: bool) -> FileConfig {
    if is_dir {
        return FileConfig {
            ext_label: "DIR".into(),
            color: "#34D399".into(),
            actions: vec![
                "Open VSCode".into(),
                "Clean Up".into(),
                "Compress".into(),
                "Delete".into(),
            ],
        };
    }

    let path = std::path::Path::new(path_str);
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    // TẠO SẴN BIẾN NÀY: Lấy đuôi file thật và viết hoa lên
    let display_ext = if ext.is_empty() {
        "FILE".into()
    } else {
        ext.to_uppercase()
    };

    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" => FileConfig {
            ext_label: display_ext, // <-- Thay chữ "IMAGE" bằng biến này
            color: "#F472B6".into(),
            actions: vec![
                "Preview".into(),
                "Optimize".into(),
                "Copy Path".into(),
                "Delete".into(),
            ],
        },
        "mp4" | "mov" | "mkv" | "avi" => FileConfig {
            ext_label: display_ext, // <-- Thay chữ "VIDEO" bằng biến này
            color: "#818CF8".into(),
            actions: vec![
                "Play".into(),
                "Convert to GIF".into(),
                "Mute Audio".into(),
                "Delete".into(),
            ],
        },
        "pdf" => FileConfig {
            ext_label: display_ext,
            color: "#FB7185".into(),
            actions: vec!["Open PDF".into(), "Merge PDFs".into(), "Compress".into()],
        },
        "rs" | "js" | "ts" | "py" | "html" | "css" => FileConfig {
            ext_label: display_ext, // <-- Thay chữ "CODE" bằng biến này
            color: "#60A5FA".into(),
            actions: vec![
                "Edit in VSCode".into(),
                "Run Script".into(),
                "Delete".into(),
            ],
        },
        _ => FileConfig {
            ext_label: display_ext,
            color: "#9CA3AF".into(),
            actions: vec!["Open with...".into(), "Copy Path".into(), "Delete".into()],
        },
    }
}
