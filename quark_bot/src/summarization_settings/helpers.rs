use super::dto::EffectiveSummarizationPrefs;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn build_summarization_keyboard(prefs: &EffectiveSummarizationPrefs) -> InlineKeyboardMarkup {
    // Create toggle button based on current state
    let toggle_text = if prefs.enabled { "❌ Disable" } else { "✅ Enable" };
    let toggle_callback = if prefs.enabled { "toggle_summarizer:off" } else { "toggle_summarizer:on" };
    
    // Create token threshold buttons with current selection highlighted
    let token_buttons = vec![12000, 14000, 16000, 18000, 20000]
        .into_iter()
        .map(|threshold| {
            let text = if threshold == prefs.token_limit {
                format!("🔘 {}k", threshold / 1000) // Highlight current selection
            } else {
                format!("⚪ {}k", threshold / 1000) // Show as unselected
            };
            InlineKeyboardButton::callback(text, format!("set_summarizer_threshold:{}", threshold))
        })
        .collect::<Vec<_>>();
    
    InlineKeyboardMarkup::new(vec![
        // Single toggle button
        vec![InlineKeyboardButton::callback(toggle_text, toggle_callback)],
        // Token threshold buttons in single column
        vec![token_buttons[0].clone()], // 12k
        vec![token_buttons[1].clone()], // 14k
        vec![token_buttons[2].clone()], // 16k
        vec![token_buttons[3].clone()], // 18k
        vec![token_buttons[4].clone()], // 20k
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
