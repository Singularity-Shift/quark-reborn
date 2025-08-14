//! Utility functions for quark_bot.

use chrono::{DateTime, Utc};
use open_ai_rust_responses_by_sshift::Model;
use quark_core::helpers::dto::{AITool, PurchaseRequest, ToolUsage};
use regex::Regex;
use std::env;
use teloxide::{
    Bot,
    prelude::*,
    types::{ChatId, UserId},
};

use crate::dependencies::BotDependencies;

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

// Minimal markdown to Telegram-HTML converter supporting triple backtick fences
pub fn markdown_to_html(input: &str) -> String {
    // Handle fenced code blocks ```lang\n...\n```
    let mut html = String::new();
    let mut lines = input.lines();
    let mut in_code = false;
    while let Some(line) = lines.next() {
        if line.trim_start().starts_with("```") {
            if !in_code {
                in_code = true;
                html.push_str("<pre>");
            } else {
                in_code = false;
                html.push_str("</pre>\n");
            }
            continue;
        }
        if in_code {
            // Only escape within code blocks
            html.push_str(&teloxide::utils::html::escape(line));
            html.push('\n');
        } else {
            // Preserve non-code content as-is so valid Telegram-HTML (e.g., <a href=...>) remains clickable
            html.push_str(line);
            html.push('\n');
        }
    }
    html
}

/// Sanitize Telegram-HTML outside of <pre> blocks by escaping stray '<' that do not
/// begin an allowed tag. This prevents Telegram parse errors from sequences like
/// "<TOKEN>" or "a < b" in normal prose.
///
/// Allowed tags (case-insensitive): a, b/strong, i/em, u/ins, s/strike/del, code, pre,
/// tg-spoiler, span class="tg-spoiler".
pub fn sanitize_html_outside_pre(input: &str) -> String {
    fn is_allowed_tag(name: &str, inner: &str) -> bool {
        let tag = name.to_ascii_lowercase();
        match tag.as_str() {
            "a" | "b" | "strong" | "i" | "em" | "u" | "ins" | "s" | "strike" | "del"
            | "code" | "pre" | "tg-spoiler" => true,
            "span" => {
                let lower_inner = inner.to_ascii_lowercase();
                // Only allow span when explicitly a Telegram spoiler
                lower_inner.contains("class=\"tg-spoiler\"")
                    || lower_inner.contains("class='tg-spoiler'")
            }
            _ => false,
        }
    }

    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    let mut inside_pre = false;

    while i < input.len() {
        // Safe because i always lands on char boundary
        let ch = input[i..].chars().next().unwrap();
        if ch == '<' {
            // Find closing '>' from next byte onward
            if let Some(rel_end) = input[i + 1..].find('>') {
                let end = i + 1 + rel_end; // index of '>'
                let inner = &input[i + 1..end];
                let trimmed = inner.trim();

                // Extract tag name to possibly toggle <pre> state
                let is_closing = trimmed.starts_with('/')
                    || trimmed.starts_with("/ ")
                    || trimmed.starts_with(" /");
                let name = trimmed
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if inside_pre {
                    // Inside <pre> preserve as-is, but detect </pre>
                    out.push('<');
                    out.push_str(inner);
                    out.push('>');
                    if name == "pre" && is_closing {
                        inside_pre = false;
                    }
                    i = end + 1;
                    continue;
                }

                // Outside <pre>: only allow whitelisted tags. Otherwise escape '<'
                if !name.is_empty() && is_allowed_tag(&name, trimmed) {
                    out.push('<');
                    out.push_str(inner);
                    out.push('>');
                    if name == "pre" && !is_closing {
                        inside_pre = true;
                    }
                } else {
                    // Turn e.g. <TOKEN> into &lt;TOKEN>
                    out.push_str("&lt;");
                    out.push_str(inner);
                    out.push('>');
                }
                i = end + 1;
            } else {
                // No closing '>' — escape '<' to be safe
                out.push_str("&lt;");
                i += ch.len_utf8();
            }
        } else {
            // Fast path for ASCII, but handle UTF-8 correctly
            if inside_pre {
                out.push(ch);
            } else if ch == '>' {
                // Escape stray '>' outside of tags/pre to avoid confusing the parser
                out.push_str("&gt;");
            } else {
                out.push(ch);
            }
            i += ch.len_utf8();
        }
    }

    // If we ended while inside <pre>, we leave as-is; Telegram will error only on malformed tags,
    // not on raw text.
    out
}

/// Ensure the image 'Open image' anchor points to the public Google Cloud Storage URL
/// If a line like `Image URL: <a href="...">Open image</a>` exists and a
/// `https://storage.googleapis.com/...` URL is present anywhere in the text,
/// rewrite that anchor's href to use the GCS URL.
pub fn normalize_image_url_anchor(text: &str) -> String {
    let re_gcs = Regex::new(r#"https://storage\.googleapis\.com/[^\s<>\"]+"#).unwrap();
    let gcs = if let Some(m) = re_gcs.find(text) {
        m.as_str().to_string()
    } else {
        return text.to_string();
    };

    let re_anchor = Regex::new(r#"(?i)(Image URL:\s*)<a\s+href=\"[^\"]+\">([^<]*)</a>"#).unwrap();
    let replacement = format!(r#"$1<a href=\"{}\">$2</a>"#, gcs);
    re_anchor.replace(text, replacement.as_str()).to_string()
}

pub async fn create_purchase_request(
    file_search_calls: u32,
    web_search_calls: u32,
    image_generation_calls: u32,
    total_tokens_used: u32,
    model: Model,
    token: &str,
    mut group_id: Option<String>,
    user_id: String,
    bot_deps: BotDependencies,
) -> Result<(), anyhow::Error> {
    // Resolve currency/version from user or group prefs; fallback to on-chain default
    let (currency, coin_version) = if let Some(gid) = &group_id {
        let key = gid.clone();
        let prefs: Option<crate::payment::dto::PaymentPrefs> =
            bot_deps.payment.get_payment_token_session(key);
        if prefs.is_some() {
            let prefs = prefs.unwrap();
            (prefs.currency, prefs.version)
        } else {
            (
                bot_deps.default_payment_prefs.currency,
                bot_deps.default_payment_prefs.version,
            )
        }
    } else {
        let key = user_id;
        let prefs: Option<crate::payment::dto::PaymentPrefs> =
            bot_deps.payment.get_payment_token_session(key);
        if prefs.is_some() {
            let prefs = prefs.unwrap();
            (prefs.currency, prefs.version)
        } else {
            (
                bot_deps.default_payment_prefs.currency,
                bot_deps.default_payment_prefs.version,
            )
        }
    };
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
        currency,
        coin_version,
        tokens_used: total_tokens_used,
        tools_used,
        group_id: group_id.clone(),
    };

    let response = if group_id.is_some() {
        bot_deps
            .service
            .group_purchase(token.to_string(), purchase_request)
            .await
    } else {
        bot_deps
            .service
            .purchase(token.to_string(), purchase_request)
            .await
    };

    match response {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Error purchasing tokens: {}", e);
            Err(e)
        }
    }
}

pub async fn is_admin(bot: &Bot, chat_id: ChatId, user_id: UserId) -> bool {
    let admins = bot.get_chat_administrators(chat_id).await;

    if admins.is_err() {
        return false;
    }

    let admins = admins.unwrap();
    let is_admin = admins.iter().any(|member| member.user.id == user_id);
    is_admin
}
