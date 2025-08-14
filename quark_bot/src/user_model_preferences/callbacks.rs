use super::dto::{ChatModel, Gpt5Mode, gpt5_effort_to_display_string, gpt5_mode_to_display_string};
use super::handler::{UserModelPreferences, get_temperature_keyboard};
use anyhow::Result;
// no Effort import; GPT-5 uses ReasoningEffort

use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

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
            "GPT5" => ChatModel::GPT5,
            "GPT5Mini" => ChatModel::GPT5Mini,
            _ => {
                bot.answer_callback_query(query.id)
                    .text("❌ Invalid model selection")
                    .await?;
                return Ok(());
            }
        };

        // For 4-series, ask for temperature; for 5-series, branch into Mode/Effort/Verbosity
        let is_four_series = matches!(
            model,
            ChatModel::GPT41 | ChatModel::GPT41Mini | ChatModel::GPT4o
        );
        if is_four_series {
            let keyboard = get_temperature_keyboard();
            if let Some(message) = query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        format!(
                            "🌡️ <b>Select temperature for {}:</b>\n\nChoose the creativity level for your chat responses.\n\n<i>Note: Temperature applies to 4‑series models only.</i>",
                            model.to_display_string()
                        )
                    )
                    .reply_markup(keyboard)
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
            }

            bot.answer_callback_query(query.id)
                .text(format!("Selected {}", model.to_display_string()))
                .await?;
        } else {
            // Save baseline for GPT-5: default temp still stored (unused), set defaults for mode/verbosity
            prefs_handler.set_chat_preferences(username, model.clone(), 0.6)?;

            // Ask GPT-5 Mode selection next and confirm model choice
            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::callback(
                    "Regular",
                    "set_gpt5_mode:Regular",
                )],
                vec![InlineKeyboardButton::callback(
                    "Reasoning",
                    "set_gpt5_mode:Reasoning",
                )],
            ]);

            if let Some(message) = query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        format!(
                            "✅ <b>Model selected:</b> {}\n\n🧩 <b>Select GPT‑5 Mode:</b>\nChoose between regular responses or reasoning mode.\n<i>Note: Reasoning uses more LLM tokens and may cost more.</i>",
                            model.to_display_string()
                        )
                    )
                    .reply_markup(keyboard)
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
            }

            bot.answer_callback_query(query.id)
                .text(format!("Model: {}", model.to_display_string()))
                .await?;
        }
    } else if data.starts_with("set_temperature:") {
        let temp_str = data.strip_prefix("set_temperature:").unwrap();
        let temperature: f32 = temp_str.parse().unwrap_or(0.6);

        // We need to get the previously selected model from the message context
        // For now, we'll parse it from the message text
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                let message_text = msg.text().unwrap_or("");
                let model = if message_text.contains("GPT-5-Mini") {
                    ChatModel::GPT5Mini
                } else if message_text.contains("GPT-5") {
                    ChatModel::GPT5
                } else if message_text.contains("GPT-4.1") {
                    ChatModel::GPT41
                } else if message_text.contains("GPT-4.1-Mini") {
                    ChatModel::GPT41Mini
                } else if message_text.contains("GPT-4o") {
                    ChatModel::GPT4o
                } else {
                    ChatModel::GPT5Mini // fallback to new default
                };

                prefs_handler.set_chat_preferences(username, model.clone(), temperature)?;

                // Show popup notification
                bot.answer_callback_query(query.id.clone())
                    .text("Preferences saved!")
                    .await?;

                // Return to user settings menu instead of closing
                let keyboard = InlineKeyboardMarkup::new(vec![
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
                        "↩️ Close",
                        "user_settings_close",
                    )],
                ]);

                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    "⚙️ <b>User Settings</b>\n\n• Manage your model, view current settings, and configure payment.\n\n💡 If no payment token is selected, the on-chain default will be used."
                )
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
            }
        }

        bot.answer_callback_query(query.id)
            .text("Preferences saved!")
            .await?;
    } else if data.starts_with("set_gpt5_mode:") {
        let mode_str = data.strip_prefix("set_gpt5_mode:").unwrap();
        let mode = match mode_str {
            "Regular" => Gpt5Mode::Regular,
            "Reasoning" => Gpt5Mode::Reasoning,
            _ => Gpt5Mode::Regular,
        };

        // Persist mode
        let mut prefs = user_model_prefs.get_preferences(username);
        prefs.gpt5_mode = Some(mode.clone());
        // Default verbosity
        if prefs.gpt5_verbosity.is_none() {
            prefs.gpt5_verbosity = Some(open_ai_rust_responses_by_sshift::Verbosity::Medium);
        }
        user_model_prefs.set_preferences(username, &prefs)?;

        // If Reasoning, ask Effort; then ask Verbosity
        if let Some(message) = query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                if mode == Gpt5Mode::Reasoning {
                    let keyboard = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            "Minimal (💸 Cheapest)",
                            "set_gpt5_effort:Minimal",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "Medium (💰 Standard)",
                            "set_gpt5_effort:Medium",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "High (💸💸 Most Expensive)",
                            "set_gpt5_effort:High",
                        )],
                    ]);
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        format!(
                            "✅ <b>Mode:</b> {}\n\n⚡ <b>Select GPT‑5 Reasoning Effort:</b>\n<i>Lower effort is cheaper; higher effort uses more LLM tokens and may cost more.</i>",
                            gpt5_mode_to_display_string(&mode)
                        )
                    )
                    .reply_markup(keyboard)
                    .parse_mode(ParseMode::Html)
                    .await?;
                } else {
                    // Ask verbosity directly
                    let keyboard = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            "Low (💸 Cheapest)",
                            "set_gpt5_verbosity:Low",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "Medium (💰 Standard)",
                            "set_gpt5_verbosity:Medium",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "High (💸💸 Most Expensive)",
                            "set_gpt5_verbosity:High",
                        )],
                    ]);
                    bot.edit_message_text(
                        msg.chat.id,
                        msg.id,
                        format!(
                            "✅ <b>Mode:</b> {}\n\n🗣️ <b>Select GPT‑5 Verbosity:</b>\n<i>Lower verbosity is cheaper; higher verbosity uses more LLM tokens and may cost more.</i>",
                            gpt5_mode_to_display_string(&mode)
                        )
                    )
                    .reply_markup(keyboard)
                    .parse_mode(ParseMode::Html)
                    .await?;
                }
            }
        }

        bot.answer_callback_query(query.id)
            .text(format!("Mode set: {}", gpt5_mode_to_display_string(&mode)))
            .await?;
    } else if data.starts_with("set_gpt5_effort:") {
        let effort_str = data.strip_prefix("set_gpt5_effort:").unwrap();
        let eff = match effort_str {
            "Minimal" => open_ai_rust_responses_by_sshift::ReasoningEffort::Minimal,
            "Medium" => open_ai_rust_responses_by_sshift::ReasoningEffort::Medium,
            "High" => open_ai_rust_responses_by_sshift::ReasoningEffort::High,
            _ => open_ai_rust_responses_by_sshift::ReasoningEffort::Medium,
        };

        let mut prefs = user_model_prefs.get_preferences(username);
        prefs.gpt5_effort = Some(eff.clone());
        user_model_prefs.set_preferences(username, &prefs)?;

        // Next: ask verbosity
        if let Some(message) = query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                let keyboard = InlineKeyboardMarkup::new(vec![
                    vec![InlineKeyboardButton::callback(
                        "Low (💸 Cheapest)",
                        "set_gpt5_verbosity:Low",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "Medium (💰 Standard)",
                        "set_gpt5_verbosity:Medium",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "High (💸💸 Most Expensive)",
                        "set_gpt5_verbosity:High",
                    )],
                ]);
                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    format!(
                        "✅ Effort set: {}\n\n🗣️ <b>Select GPT‑5 Verbosity:</b>\n<i>Lower verbosity is cheaper; higher verbosity uses more LLM tokens and may cost more.</i>",
                        gpt5_effort_to_display_string(&eff)
                    )
                )
                .reply_markup(keyboard)
                .parse_mode(ParseMode::Html)
                .await?;
            }
        }

        bot.answer_callback_query(query.id)
            .text("Effort saved")
            .await?;
    } else if data.starts_with("set_gpt5_verbosity:") {
        let v_str = data.strip_prefix("set_gpt5_verbosity:").unwrap();
        let v = match v_str {
            "Low" => open_ai_rust_responses_by_sshift::Verbosity::Low,
            "Medium" => open_ai_rust_responses_by_sshift::Verbosity::Medium,
            "High" => open_ai_rust_responses_by_sshift::Verbosity::High,
            _ => open_ai_rust_responses_by_sshift::Verbosity::Medium,
        };

        let mut prefs = user_model_prefs.get_preferences(username);
        prefs.gpt5_verbosity = Some(v.clone());
        user_model_prefs.set_preferences(username, &prefs)?;

        if let Some(message) = query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                // Show popup notification
                bot.answer_callback_query(query.id.clone())
                    .text("Verbosity saved")
                    .await?;

                // Return to user settings menu instead of closing
                let keyboard = InlineKeyboardMarkup::new(vec![
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
                        "↩️ Close",
                        "user_settings_close",
                    )],
                ]);

                bot.edit_message_text(
                    msg.chat.id,
                    msg.id,
                    "⚙️ <b>User Settings</b>\n\n• Manage your model, view current settings, and configure payment.\n\n💡 If no payment token is selected, the on-chain default will be used."
                )
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
            }
        }
    }

    Ok(())
}
