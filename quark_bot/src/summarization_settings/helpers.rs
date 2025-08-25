use super::dto::EffectiveSummarizationPrefs;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn build_summarization_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        // Toggle row
        vec![
            InlineKeyboardButton::callback("Enable", "toggle_summarizer:on"),
            InlineKeyboardButton::callback("Disable", "toggle_summarizer:off"),
        ],
        // Threshold rows
        vec![
            InlineKeyboardButton::callback("12k", "set_summarizer_threshold:12000"),
            InlineKeyboardButton::callback("14k", "set_summarizer_threshold:14000"),
            InlineKeyboardButton::callback("16k", "set_summarizer_threshold:16000"),
        ],
        vec![
            InlineKeyboardButton::callback("18k", "set_summarizer_threshold:18000"),
            InlineKeyboardButton::callback("20k", "set_summarizer_threshold:20000"),
        ],
        // Back button
        vec![InlineKeyboardButton::callback(
            "↩️ Back",
            "summarization_back_to_usersettings",
        )],
    ])
}

pub fn format_summarization_status(prefs: &EffectiveSummarizationPrefs) -> String {
    let status = if prefs.enabled { "<b>On</b>" } else { "<b>Off</b>" };
    let threshold = format!("<code>{}</code>", prefs.token_limit);
    
    format!(
        "⚙️ <b>Summarization Settings</b>\n\nStatus: {}\nThreshold: {} tokens\n\n💡 Summarization automatically condenses long conversations when they exceed your chosen token threshold.",
        status, threshold
    )
}
