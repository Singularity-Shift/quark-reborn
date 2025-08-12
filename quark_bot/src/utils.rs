//! Utility functions for quark_bot.

use chrono::{DateTime, Utc};
use open_ai_rust_responses_by_sshift::Model;
use quark_core::helpers::dto::{AITool, PurchaseRequest, ToolUsage};
use std::env;

use crate::services::handler::Services;

/// Helper function to format Unix timestamp into readable date and time
pub fn format_timestamp(timestamp: u64) -> String {
    let datetime = DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_else(|| Utc::now());
    datetime.format("%Y-%m-%d at %H:%M UTC").to_string()
}

/// Helper function to format time duration in a human-readable way
pub fn format_time_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours == 0 {
        // Less than 1 hour, show in minutes
        format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" })
    } else if hours < 24 {
        // 1-23 hours, show in hours
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    } else {
        // 24+ hours, show in days
        let days = hours / 24;
        format!("{} day{}", days, if days == 1 { "" } else { "s" })
    }
}

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
                format!("{}...", &cleaned[..32])
            }
        } else {
            format!("{}...", &cleaned[..32])
        }
    } else {
        cleaned.to_string()
    }
}



// markdown_to_html removed: AI now emits Telegram-compatible HTML directly

pub async fn create_purchase_request(
    file_search_calls: u32,
    web_search_calls: u32,
    image_generation_calls: u32,
    service: Services,
    total_tokens_used: u32,
    model: Model,
    token: &str,
    mut group_id: Option<String>,
) -> Result<(), anyhow::Error> {
    let mut tools_used = Vec::new();
    let account_seed =
        env::var("ACCOUNT_SEED").map_err(|e| anyhow::anyhow!("ACCOUNT_SEED is not set: {}", e))?;

    if file_search_calls > 0 {
        tools_used.push(ToolUsage {
            tool: AITool::FileSearch,
            calls: file_search_calls,
        });
    };
    if web_search_calls > 0 {
        tools_used.push(ToolUsage {
            tool: AITool::WebSearchPreview,
            calls: web_search_calls,
        });
    };
    if image_generation_calls > 0 {
        tools_used.push(ToolUsage {
            tool: AITool::ImageGeneration,
            calls: image_generation_calls,
        });
    };

    if group_id.is_some() {
        let group_id_result = group_id.unwrap();
        let group_id_with_seed = format!("{}-{}", group_id_result, account_seed);
        group_id = Some(group_id_with_seed);
    }

    let purchase_request = PurchaseRequest {
        model,
        tokens_used: total_tokens_used,
        tools_used,
        group_id: group_id.clone(),
    };

    let response = if group_id.is_some() {
        service
            .group_purchase(token.to_string(), purchase_request)
            .await
    } else {
        service.purchase(token.to_string(), purchase_request).await
    };

    match response {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Error purchasing tokens: {}", e);
            Err(e)
        }
    }
}
