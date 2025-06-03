//! Utility functions for quark_bot.

/// Get emoji icon based on file extension
pub fn get_file_icon(filename: &str) -> &'static str {
    let extension = filename.split('.').last().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "pdf" => "📄",
        "doc" | "docx" => "📝",
        "xls" | "xlsx" => "📊",
        "ppt" | "pptx" => "📋",
        "txt" | "md" => "📄",
        "jpg" | "jpeg" | "png" | "gif" | "webp" => "🖼️",
        "mp4" | "avi" | "mov" | "mkv" => "🎥",
        "mp3" | "wav" | "flac" | "aac" => "🎵",
        "zip" | "rar" | "7z" => "📦",
        "json" | "xml" | "csv" => "🗂️",
        "py" | "js" | "ts" | "rs" | "cpp" | "java" => "💻",
        _ => "📎"
    }
}

/// Smart filename cleaning and truncation
pub fn clean_filename(filename: &str) -> String {
    // Remove timestamp prefixes like "1030814179_"
    let cleaned = if let Some(underscore_pos) = filename.find('_') {
        if filename[..underscore_pos].chars().all(|c| c.is_ascii_digit()) {
            &filename[underscore_pos + 1..]
        } else {
            filename
        }
    } else {
        filename
    };
    // Truncate if too long, keeping extension
    if cleaned.len() > 35 {
        if let Some(dot_pos) = cleaned.rfind('.') {
            let name_part = &cleaned[..dot_pos];
            let ext_part = &cleaned[dot_pos..];
            if name_part.len() > 30 {
                format!("{}...{}", &name_part[..27], ext_part)
            } else {
                cleaned.to_string()
            }
        } else {
            format!("{}...", &cleaned[..32])
        }
    } else {
        cleaned.to_string()
    }
} 