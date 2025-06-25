//! Utility functions for quark_bot.

use regex::Regex;
use std::collections::HashMap;

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
        _ => "📎",
    }
}

/// Smart filename cleaning and truncation
pub fn clean_filename(filename: &str) -> String {
    // Remove timestamp prefixes like "1030814179_"
    let cleaned = if let Some(underscore_pos) = filename.find('_') {
        if filename[..underscore_pos]
            .chars()
            .all(|c| c.is_ascii_digit())
        {
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

/// Convert a limited subset of Markdown (headings, bold, links, horizontal rule, code blocks)
/// into Telegram-compatible HTML so we can send messages with `ParseMode::Html`.
/// This is intentionally simple and avoids escaping edge-cases; it covers the
/// patterns we expect GPT-generated content to use.
pub fn markdown_to_html(md: &str) -> String {
    let mut html = teloxide::utils::html::escape(md);

    // --- Step 1: Isolate code blocks (both inline and multi-line) to prevent nested parsing ---
    // We replace code content with placeholders and process them last to avoid
    // issues where markdown (like links) inside a code block is processed,
    // or a link URL containing special characters is broken.
    
    let mut code_blocks = HashMap::new();
    let mut counter = 0;

    // Handle multi-line code blocks first (```...```)
    let re_code_block = Regex::new(r"```(?:[a-zA-Z0-9_+-]*\n)?((?s).*?)```").unwrap();
    html = re_code_block
        .replace_all(&html, |caps: &regex::Captures| {
            let placeholder = format!("__QUARK_CODE_BLOCK_{}__", counter);
            let code_content = &caps[1];
            // For multi-line code blocks, we use <pre> tags for proper formatting
            code_blocks.insert(placeholder.clone(), format!("<pre>{}</pre>", code_content));
            counter += 1;
            placeholder
        })
        .to_string();

    // Handle inline code (single backticks)
    let re_code = Regex::new(r"`(.*?)`").unwrap();
    html = re_code
        .replace_all(&html, |caps: &regex::Captures| {
            let placeholder = format!("__QUARK_CODE_{}__", counter);
            let code_content = &caps[1];
            // For inline code, we use <code> tags
            code_blocks.insert(placeholder.clone(), format!("<code>{}</code>", code_content));
            counter += 1;
            placeholder
        })
        .to_string();

    // --- Step 2: Process standard Markdown on the remaining text ---

    // Horizontal rule --- → plain em-dash line (HTML <hr> not allowed by Telegram)
    let re_hr = Regex::new(r"(?m)^---+").unwrap();
    html = re_hr.replace_all(&html, "———").to_string();

    // Headings (#, ##, ###) → <b>…</b>
    let re_h1 = Regex::new(r"(?m)^#{1,3}\s+(.*)").unwrap();
    html = re_h1
        .replace_all(&html, |caps: &regex::Captures| {
            format!("<b>{}</b>", &caps[1])
        })
        .to_string();

    // Bold **text** → <b>text</b>
    let re_bold = Regex::new(r"\*\*(.*?)\*\*").unwrap();
    html = re_bold.replace_all(&html, "<b>$1</b>").to_string();

    // Links [text](url) → <a href="url">text</a>
    let re_link = Regex::new(r"\[(.*?)\]\((.*?)\)").unwrap();
    html = re_link
        .replace_all(&html, "<a href=\"$2\">$1</a>")
        .to_string();

    // --- Step 3: Restore code blocks with proper HTML tags ---
    for (placeholder, code_html) in code_blocks {
        html = html.replace(&placeholder, &code_html);
    }

    html
}
