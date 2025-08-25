use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode},
};

use crate::dependencies::BotDependencies;
use crate::filters::dto::{MatchType, PendingFilterStep, PendingFilterWizardState, ResponseType, FilterError};
use crate::filters::helpers::parse_triggers;
use crate::utils;

pub async fn handle_filters_callback(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(data) = &query.data {
        let user_id = query.from.id;

        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                let is_admin = utils::is_admin(&bot, m.chat.id, user_id).await;

                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage filters")
                        .await?;
                    return Ok(());
                }

                match data.as_str() {
                    "filters_main" => {
                        show_filters_main_menu(&bot, &query, &bot_deps, m.chat.id).await?;
                    }
                    "filters_add" => {
                        start_filter_wizard(&bot, &query, &bot_deps, m.chat.id, user_id).await?;
                    }
                    "filters_view" => {
                        show_view_filters_menu(&bot, &query, &bot_deps, m.chat.id).await?;
                    }
                    "filters_reset_confirm" => {
                        show_reset_confirmation(&bot, &query, m.chat.id).await?;
                    }
                    "filters_reset_execute" => {
                        execute_reset_filters(&bot, &query, &bot_deps, m.chat.id).await?;
                    }
                    "filters_back_to_settings" => {
                        show_group_settings_menu(&bot, &query, m.chat.id).await?;
                    }
                    "filters_confirm" => {
                        confirm_and_create_filter(&bot, &query, &bot_deps, m.chat.id, user_id).await?;
                    }
                    "filters_cancel" => {
                        cancel_filter_wizard(&bot, &query, &bot_deps, m.chat.id, user_id).await?;
                    }
                    _ if data.starts_with("filters_remove:") => {
                        let filter_id = data.strip_prefix("filters_remove:").unwrap();
                        remove_filter(&bot, &query, &bot_deps, m.chat.id, filter_id).await?;
                    }
                    _ => {
                        bot.answer_callback_query(query.id)
                            .text("Unknown filter action")
                            .await?;
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn process_message_for_filters(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> Result<bool> {
    if msg.chat.is_private() {
        return Ok(false);
    }

    if let Some(text) = msg.text() {
        let group_id = msg.chat.id.to_string();
        
        match bot_deps.filters.find_matching_filters(&group_id, text) {
            Ok(matches) => {
                if let Some(filter_match) = matches.first() {
                    let response = &filter_match.filter.response;
                    
                    let parse_mode = match filter_match.filter.response_type {
                        ResponseType::Markdown => Some(ParseMode::MarkdownV2),
                        ResponseType::Text => None,
                    };

                    let mut send_message = bot
                        .send_message(msg.chat.id, response);

                    if let Some(mode) = parse_mode {
                        send_message = send_message.parse_mode(mode);
                    }

                    if let Err(e) = send_message.await {
                        log::error!("Failed to send filter response: {}", e);
                        
                        // Fallback to simple message without reply
                        bot.send_message(msg.chat.id, response).await?;
                    }

                    if let Some(user) = &msg.from {
                        let _ = bot_deps.filters.record_filter_usage(
                            &group_id,
                            &filter_match.filter.id,
                            user.id.0 as i64,
                        );
                    }

                    return Ok(true);
                }
            }
            Err(e) => {
                log::error!("Error processing filters for message: {}", e);
            }
        }
    }

    Ok(false)
}

async fn start_filter_wizard(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
    user_id: teloxide::types::UserId,
) -> Result<()> {
    let wizard_key = format!("filter_{}_{}", chat_id.0, user_id.0);
    
    let wizard_state = PendingFilterWizardState {
        group_id: chat_id.0,
        creator_user_id: user_id.0 as i64,
        step: PendingFilterStep::AwaitingTrigger,
        trigger: None,
        response: None,
        match_type: MatchType::Contains, // Default
        response_type: ResponseType::Text, // Default
    };
    
            if let Err(e) = bot_deps.filters.put_pending_settings(wizard_key, &wizard_state) {
        log::error!("Failed to save wizard state: {}", e);
        bot.answer_callback_query(query.id.clone())
            .text("❌ Failed to start filter wizard")
            .await?;
        return Ok(());
    }
    
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "❌ Cancel",
            "filters_main",
        )],
    ]);

    let text = "🔍 <b>Add New Filter - Step 1/3</b>\n\nPlease send the trigger(s) for your filter. You can send multiple triggers separated by \", \".\n\n<b>Syntax:</b>\n• Single-word: <code>hello, bye, gm</code>\n• Multi-word (use brackets): <code>[good morning], [see you later]</code>\n• Mixed: <code>gm, [good morning], morning</code>\n\n<b>Examples:</b>\n• <code>gm, [good morning], morning</code>\n• <code>bye, [see you later], goodbye</code>\n• <code>help, [need help], support</code>\n\n💡 <i>Tip: Triggers are automatically converted to lowercase and match anywhere in a message (case-insensitive).</i>";

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(query.id.clone()).await?;
    Ok(())
}

async fn show_filters_main_menu(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
) -> Result<()> {
    let group_id = chat_id.to_string();
    let filter_count = match bot_deps.filters.get_group_filters(&group_id) {
        Ok(filters) => filters.len(),
        Err(_) => 0,
    };

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "➕ Add Filter",
            "filters_add",
        )],
        vec![InlineKeyboardButton::callback(
            format!("📋 View Filters ({})", filter_count),
            "filters_view",
        )],
        vec![InlineKeyboardButton::callback(
            "🗑️ Reset All Filters",
            "filters_reset_confirm",
        )],
        vec![InlineKeyboardButton::callback(
            "↩️ Back to Settings",
            "filters_back_to_settings",
        )],
    ]);

    let text = format!(
        "🔍 <b>Filters</b>\n\nMake your chat more lively with filters! The bot will reply to certain words.\n\nFilters are case insensitive; every time someone says your trigger words, Quark will reply something else! Can be used to create your own commands, if desired.\n\n<b>Current filters:</b> {} active",
        filter_count
    );

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(query.id.clone()).await?;
    Ok(())
}



async fn show_view_filters_menu(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
) -> Result<()> {
    let group_id = chat_id.to_string();
    let filters = match bot_deps.filters.get_group_filters(&group_id) {
        Ok(filters) => filters,
        Err(_) => Vec::new(),
    };

    if filters.is_empty() {
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                "➕ Add Filter",
                "filters_add",
            )],
            vec![InlineKeyboardButton::callback(
                "↩️ Back to Filters",
                "filters_main",
            )],
        ]);

        let text = "📋 <b>Active Filters</b>\n\n<i>No filters found for this group.</i>\n\n💡 Use the button below to create your first filter!";

        if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
            bot.edit_message_text(message.chat.id, message.id, text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }
    } else {
        let mut keyboard_rows = Vec::new();
        
        for filter in &filters {
            let stats = bot_deps.filters.get_filter_stats(&group_id, &filter.id)
                .unwrap_or_else(|_| crate::filters::dto::FilterStats {
                    group_id: group_id.clone(),
                    filter_id: filter.id.clone(),
                    usage_count: 0,
                    last_triggered: None,
                    last_triggered_by: None,
                });

            let display_trigger = if filter.trigger.len() > 20 {
                format!("{}...", &filter.trigger[..17])
            } else {
                filter.trigger.clone()
            };

            let button_text = format!("🗑️ {} ({}x)", display_trigger, stats.usage_count);
            let remove_button = InlineKeyboardButton::callback(
                button_text,
                format!("filters_remove:{}", filter.id),
            );
            keyboard_rows.push(vec![remove_button]);
        }

        keyboard_rows.push(vec![
            InlineKeyboardButton::callback("➕ Add New", "filters_add"),
            InlineKeyboardButton::callback("🗑️ Reset All", "filters_reset_confirm"),
        ]);
        keyboard_rows.push(vec![InlineKeyboardButton::callback(
            "↩️ Back to Filters",
            "filters_main",
        )]);

        let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

        let mut text = format!("📋 <b>Active Filters ({})</b>\n\n", filters.len());
        
        for filter in &filters {
            let stats = bot_deps.filters.get_filter_stats(&group_id, &filter.id)
                .unwrap_or_else(|_| crate::filters::dto::FilterStats {
                    group_id: group_id.clone(),
                    filter_id: filter.id.clone(),
                    usage_count: 0,
                    last_triggered: None,
                    last_triggered_by: None,
                });

            let response_preview = if filter.response.len() > 50 {
                format!("{}...", &filter.response[..47])
            } else {
                filter.response.clone()
            };

            text.push_str(&format!(
                "🔹 <b>{}</b>\nResponse: \"{}\"\nUsed: {} times\n\n",
                filter.trigger, response_preview, stats.usage_count
            ));
        }

        text.push_str("💡 <i>Tap any button below to remove a filter.</i>");

        if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
            bot.edit_message_text(message.chat.id, message.id, text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }
    }

    bot.answer_callback_query(query.id.clone()).await?;
    Ok(())
}

async fn show_reset_confirmation(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    _chat_id: teloxide::types::ChatId,
) -> Result<()> {
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("✅ Yes, Reset All", "filters_reset_execute"),
            InlineKeyboardButton::callback("❌ Cancel", "filters_main"),
        ],
    ]);

    let text = "🗑️ <b>Reset All Filters</b>\n\n⚠️ <b>Warning:</b> This will permanently delete ALL filters in this group.\n\nAre you sure you want to continue?";

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(query.id.clone()).await?;
    Ok(())
}

async fn execute_reset_filters(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
) -> Result<()> {
    let group_id = chat_id.to_string();
    
    match bot_deps.filters.reset_group_filters(&group_id) {
        Ok(count) => {
            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::callback(
                    "↩️ Back to Filters",
                    "filters_main",
                )],
            ]);

            let text = format!(
                "✅ <b>Filters Reset Successfully</b>\n\n🗑️ Removed {} filters from this group.\n\n💡 You can now create new filters using the Add Filter option.",
                count
            );

            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
                bot.edit_message_text(message.chat.id, message.id, text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }

            bot.answer_callback_query(query.id.clone())
                .text("✅ All filters reset successfully")
                .await?;
        }
        Err(e) => {
            log::error!("Failed to reset filters: {}", e);
            bot.answer_callback_query(query.id.clone())
                .text("❌ Failed to reset filters")
                .await?;
        }
    }

    Ok(())
}

async fn remove_filter(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
    filter_id: &str,
) -> Result<()> {
    let group_id = chat_id.to_string();
    
    match bot_deps.filters.remove_filter(&group_id, filter_id) {
        Ok(_) => {
            show_view_filters_menu(bot, query, bot_deps, chat_id).await?;
            bot.answer_callback_query(query.id.clone())
                .text("✅ Filter removed successfully")
                .await?;
        }
        Err(e) => {
            log::error!("Failed to remove filter: {}", e);
            bot.answer_callback_query(query.id.clone())
                .text("❌ Failed to remove filter")
                .await?;
        }
    }

    Ok(())
}

async fn show_group_settings_menu(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    _chat_id: teloxide::types::ChatId,
) -> Result<()> {
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "💳 Payment Settings",
            "open_group_payment_settings",
        )],
        vec![InlineKeyboardButton::callback(
            "🏛️ DAO Preferences",
            "open_dao_preferences",
        )],
        vec![InlineKeyboardButton::callback(
            "🛡️ Moderation",
            "open_moderation_settings",
        )],
        vec![InlineKeyboardButton::callback(
            "🎯 Sponsor Settings",
            "open_sponsor_settings",
        )],
        vec![InlineKeyboardButton::callback(
            "👋 Welcome Settings",
            "welcome_settings",
        )],
        vec![InlineKeyboardButton::callback(
            "🔍 Filters",
            "filters_main",
        )],
        vec![InlineKeyboardButton::callback(
            "🔄 Migrate Group ID",
            "open_migrate_group_id",
        )],
        vec![InlineKeyboardButton::callback(
            "↩️ Close",
            "group_settings_close",
        )],
    ]);

    let text = "⚙️ <b>Group Settings</b>\n\n• Configure payment token, DAO preferences, moderation, sponsor settings, filters, and group migration.\n\n💡 Only group administrators can access these settings.";

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(query.id.clone()).await?;
    Ok(())
}

async fn confirm_and_create_filter(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
    user_id: teloxide::types::UserId,
) -> Result<()> {
    let wizard_key = format!("filter_{}_{}", chat_id.0, user_id.0);
    
            if let Some(wizard_state) = bot_deps.filters.get_pending_settings(&wizard_key) {
        if wizard_state.step == PendingFilterStep::AwaitingConfirm {
            let trigger_input = wizard_state.trigger.clone().unwrap_or_default();
            let triggers = parse_triggers(&trigger_input);
            let response_text = wizard_state.response.clone().unwrap_or_default();

            let mut created: Vec<String> = Vec::new();
            let mut duplicates: Vec<String> = Vec::new();
            let mut failures: Vec<(String, String)> = Vec::new();

            for t in triggers {
                let filter = crate::filters::dto::FilterDefinition {
                    trigger: t.clone(),
                    response: response_text.clone(),
                    group_id: wizard_state.group_id.to_string(),
                    created_by: wizard_state.creator_user_id,
                    created_at: chrono::Utc::now().timestamp(),
                    is_active: true,
                    match_type: wizard_state.match_type.clone(),
                    response_type: wizard_state.response_type.clone(),
                    id: uuid::Uuid::new_v4().to_string(),
                };

                match bot_deps.filters.create_filter(filter) {
                    Ok(_) => created.push(t),
                    Err(FilterError::_DuplicateFilter(_)) => duplicates.push(t),
                    Err(err) => failures.push((t, format!("{}", err))),
                }
            }

            // Clean up wizard state regardless
            if let Err(e) = bot_deps.filters.remove_pending_settings(&wizard_key) {
                log::error!("Failed to remove filter wizard state: {}", e);
            }

            // Build result message
            let mut msg_parts: Vec<String> = Vec::new();
            if !created.is_empty() {
                let list = created.iter().map(|t| format!("<code>{}</code>", t)).collect::<Vec<_>>().join(", ");
                msg_parts.push(format!("✅ <b>Created</b>: {}", list));
            }
            if !duplicates.is_empty() {
                let list = duplicates.iter().map(|t| format!("<code>{}</code>", t)).collect::<Vec<_>>().join(", ");
                msg_parts.push(format!("⚠️ <b>Skipped (duplicate)</b>: {}", list));
            }
            if !failures.is_empty() {
                let list = failures
                    .iter()
                    .map(|(t, e)| format!("<code>{}</code> ({})", t, e))
                    .collect::<Vec<_>>()
                    .join(", ");
                msg_parts.push(format!("❌ <b>Failed</b>: {}", list));
            }
            if msg_parts.is_empty() {
                msg_parts.push("❌ No valid triggers provided.".to_string());
            }

            let success_text = format!(
                "✅ <b>Filter Creation Result</b>\n\n{}\n\n💬 Response: <code>{}</code>\n\n💡 You can add more filters from the Filters menu.",
                msg_parts.join("\n"),
                response_text
            );

            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::callback("🔍 Back to Filters", "filters_main")],
            ]);

            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
                bot.edit_message_text(message.chat.id, message.id, success_text)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
            }

            bot.answer_callback_query(query.id.clone())
                .text("✅ Processed triggers")
                .await?;
        } else {
            bot.answer_callback_query(query.id.clone())
                .text("❌ Invalid wizard state")
                .await?;
        }
    } else {
        bot.answer_callback_query(query.id.clone())
            .text("❌ No active filter wizard found")
            .await?;
    }
    
    Ok(())
}

async fn cancel_filter_wizard(
    bot: &Bot,
    query: &teloxide::types::CallbackQuery,
    bot_deps: &BotDependencies,
    chat_id: teloxide::types::ChatId,
    user_id: teloxide::types::UserId,
) -> Result<()> {
    let wizard_key = format!("filter_{}_{}", chat_id.0, user_id.0);
    
    // Clean up wizard state
    if let Err(e) = bot_deps.filters.remove_pending_settings(&wizard_key) {
        log::error!("Failed to remove filter wizard state: {}", e);
    }
    
    // Show cancellation message
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔍 Back to Filters", "filters_main")],
    ]);
    
    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, "❌ <b>Filter Creation Cancelled</b>\n\nNo filter was created.")
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }
    
    bot.answer_callback_query(query.id.clone())
        .text("✅ Filter creation cancelled")
        .await?;
    
    Ok(())
}
