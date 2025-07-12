use super::dto::{ChatModel, ReasoningModel, effort_to_display_string};
use super::handler::{UserModelPreferences, get_temperature_keyboard, get_effort_keyboard};
use anyhow::Result;
use open_ai_rust_responses_by_sshift::types::Effort;

use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, ParseMode};

pub async fn handle_model_preferences_callback(
    bot: Bot,
    query: CallbackQuery,
    user_model_prefs: UserModelPreferences,
) -> Result<()> {
    let data = query.data.as_ref().unwrap();
    let user = &query.from;
    
    let username = user.username.as_ref();
    if username.is_none() {
        bot.answer_callback_query(query.id)
            .text("❌ Username not found, required for this feature")
            .await?;
        return Ok(());
    }
    let username = username.unwrap();

    let prefs_handler = &user_model_prefs;

    if data.starts_with("select_chat_model:") {
        let model_str = data.strip_prefix("select_chat_model:").unwrap();
        let model = match model_str {
            "GPT4o" => ChatModel::GPT4o,
            "GPT41" => ChatModel::GPT41,
            "GPT41Mini" => ChatModel::GPT41Mini,
            _ => {
                bot.answer_callback_query(query.id)
                    .text("❌ Invalid model selection")
                    .await?;
                return Ok(());
            }
        };

        // Store the selected model temporarily in the callback data
        let keyboard = get_temperature_keyboard();
        
        // Update the message to show temperature selection
        if let Some(message) = query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!(
                        "🌡️ **Select temperature for {}:**\n\nChoose the creativity level for your chat responses:",
                        model.to_display_string()
                    )
                )
                .reply_markup(keyboard)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            }
        }

        // Store selected model in callback for next step
        bot.answer_callback_query(query.id)
            .text(format!("Selected {}", model.to_display_string()))
            .await?;

    } else if data.starts_with("set_temperature:") {
        let temp_str = data.strip_prefix("set_temperature:").unwrap();
        let temperature: f32 = temp_str.parse().unwrap_or(0.6);
        
        // We need to get the previously selected model from the message context
        // For now, we'll parse it from the message text
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                let message_text = msg.text().unwrap_or("");
                let model = if message_text.contains("GPT-4o") {
                    ChatModel::GPT4o
                } else if message_text.contains("GPT-4.1-Mini") {
                    ChatModel::GPT41Mini
                } else if message_text.contains("GPT-4.1") {
                    ChatModel::GPT41
                } else {
                    ChatModel::GPT41Mini // fallback
                };

                prefs_handler.set_chat_preferences(username, model.clone(), temperature)?;

                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!(
                        "✅ **Chat model preferences saved!**\n\n🤖 Model: {}\n🌡️ Temperature: {}\n\nYour /c commands will now use these settings.",
                        model.to_display_string(),
                        temperature
                    )
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            }
        }

        bot.answer_callback_query(query.id)
            .text("Preferences saved!")
            .await?;

    } else if data.starts_with("select_reasoning_model:") {
        let model_str = data.strip_prefix("select_reasoning_model:").unwrap();
        let model = match model_str {
            "O3" => ReasoningModel::O3,
            "O4Mini" => ReasoningModel::O4Mini,
            _ => {
                bot.answer_callback_query(query.id)
                    .text("❌ Invalid model selection")
                    .await?;
                return Ok(());
            }
        };

        let keyboard = get_effort_keyboard();
        
        // Update the message to show effort selection
        if let Some(message) = query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!(
                        "⚡ **Select effort level for {}:**\n\nChoose how much reasoning effort to use:",
                        model.to_display_string()
                    )
                )
                .reply_markup(keyboard)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            }
        }

        bot.answer_callback_query(query.id)
            .text(format!("Selected {}", model.to_display_string()))
            .await?;

    } else if data.starts_with("set_effort:") {
        let effort_str = data.strip_prefix("set_effort:").unwrap();
        let effort = match effort_str {
            "Low" => Effort::Low,
            "Medium" => Effort::Medium,
            "High" => Effort::High,
            _ => Effort::Low, // fallback
        };
        
        // Parse model from message text
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                let message_text = msg.text().unwrap_or("");
                let model = if message_text.contains("O3") {
                    ReasoningModel::O3
                } else {
                    ReasoningModel::O4Mini // fallback
                };

                prefs_handler.set_reasoning_preferences(username, model.clone(), effort.clone())?;

                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!(
                        "✅ **Reasoning model preferences saved!**\n\n🧠 Model: {}\n⚡ Effort: {}\n\nYour /r commands will now use these settings.",
                        model.to_display_string(),
                        effort_to_display_string(&effort)
                    )
                )
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            }
        }

        bot.answer_callback_query(query.id)
            .text("Preferences saved!")
            .await?;
    }

    Ok(())
} 