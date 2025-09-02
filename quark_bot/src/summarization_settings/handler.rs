use super::helpers::{build_summarization_keyboard, format_summarization_status};
use super::SummarizationSettings;
use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

pub async fn handle_summarization_settings_callback(
    bot: Bot,
    query: CallbackQuery,
    summarization_settings: SummarizationSettings,
) -> Result<()> {
    let data = query.data.as_ref().unwrap();
    let user_id = query.from.id.0 as i64;

    if data == "open_summarization_settings" {
        show_summarization_settings_menu(bot, query, summarization_settings).await?;
    } else if data.starts_with("toggle_summarizer:") {
        let enabled = data == "toggle_summarizer:on";
        if let Err(e) = summarization_settings.set_enabled(user_id, enabled) {
            log::error!("Failed to set summarizer enabled for user {}: {}", user_id, e);
        }
        show_summarization_settings_menu(bot, query, summarization_settings).await?;
    } else if data.starts_with("set_summarizer_threshold:") {
        if let Some(threshold_str) = data.strip_prefix("set_summarizer_threshold:") {
            if let Ok(threshold) = threshold_str.parse::<u32>() {
                // Validate against allowed presets
                if [16000, 18000, 20000, 24000, 26000].contains(&threshold) {
                    if let Err(e) = summarization_settings.set_token_limit(user_id, threshold) {
                        log::error!("Failed to set token limit for user {}: {}", user_id, e);
                    }
                }
            }
        }
        show_summarization_settings_menu(bot, query, summarization_settings).await?;
    } else if data == "summarization_back_to_usersettings" {
        show_user_settings_menu(bot, query).await?;
    }

    Ok(())
}

async fn show_summarization_settings_menu(
    bot: Bot,
    query: CallbackQuery,
    summarization_settings: SummarizationSettings,
) -> Result<()> {
    let user_id = query.from.id.0 as i64;
    let effective_prefs = summarization_settings.get_effective_prefs(user_id);
    
    let status_text = format_summarization_status(&effective_prefs);
    let keyboard = build_summarization_keyboard(&effective_prefs);

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, status_text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(query.id)
        .text("Settings updated!")
        .await?;

    Ok(())
}

async fn show_user_settings_menu(bot: Bot, query: CallbackQuery) -> Result<()> {
    use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        let kb = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                "🧠 Select Model",
                "open_select_model",
            )],
            vec![InlineKeyboardButton::callback(
                "💳 Payment Settings",
                "open_payment_settings",
            )],
            vec![InlineKeyboardButton::callback(
                "📋 View My Settings",
                "open_my_settings",
            )],
            vec![InlineKeyboardButton::callback(
                "🧾 Summarization Settings",
                "open_summarization_settings",
            )],
            vec![InlineKeyboardButton::callback(
                "↩️ Close",
                "user_settings_close",
            )],
        ]);

        bot.edit_message_text(
            message.chat.id,
            message.id,
            "⚙️ <b>User Settings</b>\n\n• Manage your model, view current settings, and configure payment.\n\n💡 If no payment token is selected, the on-chain default will be used."
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(kb)
        .await?;
    }

    bot.answer_callback_query(query.id).await?;
    Ok(())
}
