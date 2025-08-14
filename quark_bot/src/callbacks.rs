//! Callback query handlers for quark_bot.

use crate::ai::vector_store::{
    delete_file_from_vector_store, delete_vector_store, list_user_files_with_names,
};
use crate::dao::handler::{handle_dao_preference_callback, handle_disable_notifications_callback};
use crate::dependencies::BotDependencies;
use crate::scheduled_prompts::callbacks::handle_scheduled_prompts_callback;
use crate::user_model_preferences::callbacks::handle_model_preferences_callback;
use crate::utils;
use anyhow::Result;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

pub async fn handle_callback_query(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(data) = &query.data {
        let user_id = query.from.id.0 as i64;

        if data.starts_with("delete_file:") {
            let file_id = data.strip_prefix("delete_file:").unwrap();

            if let Some(vector_store_id) = bot_deps.user_convos.get_vector_store_id(user_id) {
                match delete_file_from_vector_store(
                    user_id,
                    bot_deps.clone(),
                    &vector_store_id,
                    file_id,
                )
                .await
                {
                    Ok(_) => {
                        bot.answer_callback_query(query.id.clone()).await?;

                        match list_user_files_with_names(user_id, bot_deps.clone()) {
                            Ok(files) => {
                                if files.is_empty() {
                                    if let Some(
                                        teloxide::types::MaybeInaccessibleMessage::Regular(message),
                                    ) = &query.message
                                    {
                                        bot.edit_message_text(message.chat.id, message.id, "✅ <b>File deleted successfully!</b>\n\n📁 <i>Your document library is now empty</i>\n\n💡 Use /add_files to upload new documents")
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(InlineKeyboardMarkup::new(vec![] as Vec<Vec<InlineKeyboardButton>>))
                                            .await?;
                                    }
                                } else {
                                    let file_list = files
                                        .iter()
                                        .map(|file| {
                                            let icon = utils::get_file_icon(&file.name);
                                            let clean_name = utils::clean_filename(&file.name);
                                            format!("{}  <b>{}</b>", icon, clean_name)
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    let response = format!(
                                        "🗂️ <b>Your Document Library</b> ({} files)\n\n{}\n\n💡 <i>Tap any button below to manage your files</i>",
                                        files.len(),
                                        file_list
                                    );
                                    let mut keyboard_rows = Vec::new();
                                    for file in &files {
                                        let clean_name = utils::clean_filename(&file.name);
                                        let button_text = if clean_name.len() > 25 {
                                            format!("🗑️ {}", &clean_name[..22].trim_end())
                                        } else {
                                            format!("🗑️ {}", clean_name)
                                        };
                                        let delete_button = InlineKeyboardButton::callback(
                                            button_text,
                                            format!("delete_file:{}", file.id),
                                        );
                                        keyboard_rows.push(vec![delete_button]);
                                    }
                                    if files.len() > 1 {
                                        let clear_all_button = InlineKeyboardButton::callback(
                                            "🗑️ Clear All Files",
                                            "clear_all_files",
                                        );
                                        keyboard_rows.push(vec![clear_all_button]);
                                    }
                                    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

                                    if let Some(
                                        teloxide::types::MaybeInaccessibleMessage::Regular(message),
                                    ) = &query.message
                                    {
                                        bot.edit_message_text(
                                            message.chat.id,
                                            message.id,
                                            response,
                                        )
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .reply_markup(keyboard)
                                        .await?;
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to list files after deletion: {}", e);
                                bot.answer_callback_query(query.id)
                                    .text("❌ Error refreshing file list. Please try /list_files again.")
                                    .await?;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("File deletion failed: {}", e);
                        let error_msg = e.to_string();

                        // Check if it's a vector store not found error
                        if error_msg.contains("document library is no longer available") {
                            bot.answer_callback_query(query.id)
                                .text("📁 Your document library was removed. Use /add_files to create a new one!")
                                .await?;
                        } else {
                            bot.answer_callback_query(query.id)
                                .text(&format!("❌ Failed to delete file. Error: {}", e))
                                .await?;
                        }
                    }
                }
            } else {
                bot.answer_callback_query(query.id)
                    .text("❌ No document library found. Please try /list_files again.")
                    .await?;
            }
        } else if data == "clear_all_files" {
            match delete_vector_store(user_id, bot_deps.clone()).await {
                Ok(_) => {
                    bot.answer_callback_query(query.id).await?;
                    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) =
                        &query.message
                    {
                        bot.edit_message_text(message.chat.id, message.id, "✅ <b>All files cleared successfully!</b>\n\n🗑️ <i>Your entire document library has been deleted</i>\n\n💡 Use /add_files to start building your library again")
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(InlineKeyboardMarkup::new(vec![] as Vec<Vec<InlineKeyboardButton>>))
                            .await?;
                    }
                }
                Err(e) => {
                    log::error!("Failed to clear all files: {}", e);
                    bot.answer_callback_query(query.id)
                        .text(&format!("❌ Failed to clear files. Error: {}", e))
                        .await?;
                }
            }
        } else if data.starts_with("unmute:") {
            // Handle unmute callback - admin only
            let user_id_str = data.strip_prefix("unmute:").unwrap();
            let target_user_id: i64 = user_id_str.parse().unwrap_or(0);

            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) =
                &query.message
            {
                // Check if the user clicking the button is an admin
                let admins = bot.get_chat_administrators(message.chat.id).await?;
                let requester_id = query.from.id;
                let is_admin = admins.iter().any(|member| member.user.id == requester_id);

                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can use this action")
                        .await?;
                    return Ok(());
                }

                // Create full permissions to unmute the user
                let full_permissions = teloxide::types::ChatPermissions::all();

                match bot
                    .restrict_chat_member(
                        message.chat.id,
                        teloxide::types::UserId(target_user_id as u64),
                        full_permissions,
                    )
                    .await
                {
                    Ok(_) => {
                        // Delete the moderation notification message
                        if let Err(e) = bot.delete_message(message.chat.id, message.id).await {
                            log::warn!("Failed to delete moderation notification: {}", e);
                        }

                        bot.answer_callback_query(query.id)
                            .text("✅ User unmuted successfully")
                            .await?;

                        log::info!("Admin {} unmuted user {}", requester_id, target_user_id);
                    }
                    Err(e) => {
                        log::error!("Failed to unmute user {}: {}", target_user_id, e);
                        bot.answer_callback_query(query.id)
                            .text("❌ Failed to unmute user")
                            .await?;
                    }
                }
            }
        } else if data.starts_with("ban:") {
            // Handle ban callback - admin only
            // Support formats: "ban:<user_id>" and legacy "ban:<user_id>:<message_id>" (offending message is already deleted on flag)
            let parts: Vec<&str> = data.split(':').collect();
            if parts.len() < 2 {
                bot.answer_callback_query(query.id)
                    .text("❌ Invalid ban action")
                    .await?;
                return Ok(());
            }
            let target_user_id: i64 = parts[1].parse().unwrap_or(0);

            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) =
                &query.message
            {
                // Check if the user clicking the button is an admin
                let admins = bot.get_chat_administrators(message.chat.id).await?;
                let requester_id = query.from.id;
                let is_admin = admins.iter().any(|member| member.user.id == requester_id);

                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can use this action")
                        .await?;
                    return Ok(());
                }

                match bot
                    .ban_chat_member(
                        message.chat.id,
                        teloxide::types::UserId(target_user_id as u64),
                    )
                    .await
                {
                    Ok(_) => {
                        // Delete the moderation notification message itself
                        if let Err(e) = bot.delete_message(message.chat.id, message.id).await {
                            log::warn!(
                                "Failed to delete moderation message {}: {}",
                                message.id.0,
                                e
                            );
                        }

                        bot.answer_callback_query(query.id)
                            .text("✅ User banned successfully")
                            .await?;

                        log::info!("Admin {} banned user {}", requester_id, target_user_id);
                    }
                    Err(e) => {
                        log::error!("Failed to ban user {}: {}", target_user_id, e);
                        bot.answer_callback_query(query.id)
                            .text("❌ Failed to ban user")
                            .await?;
                    }
                }
            }
        } else if data.starts_with("select_chat_model:")
            || data.starts_with("set_temperature:")
            || data.starts_with("set_gpt5_mode:")
            || data.starts_with("set_gpt5_effort:")
            || data.starts_with("set_gpt5_verbosity:")
        {
            // Handle model preference callbacks
            handle_model_preferences_callback(bot, query, bot_deps.user_model_prefs.clone())
                .await?;
        } else if data == "open_select_model" {
            // Start the same workflow as /selectmodel: show chat model options
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    let keyboard = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            "GPT-5 (💰 Expensive)",
                            "select_chat_model:GPT5",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "GPT-4.1 (💸 Cheap)",
                            "select_chat_model:GPT41",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "GPT-5-Mini (💵 Cheapest)",
                            "select_chat_model:GPT5Mini",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "↩️ Back to Settings",
                            "back_to_user_settings",
                        )],
                    ]);
                    bot.edit_message_text(
                        m.chat.id,
                        m.id,
                        "🤖 <b>Select your chat model:</b>\n\nChoose which model to use for regular chat commands (/c):",
                    )
                    .reply_markup(keyboard)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                }
            }
        } else if data == "open_my_settings" {
            // Render user's current settings using existing logic
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    // Build comprehensive settings view: Model, Mode, Verbosity, Token selected
                    let user = query.from.username.clone();
                    let id = query.from.id;
                    if let Some(username) = user {
                        let prefs = bot_deps.user_model_prefs.get_preferences(&username);
                        // Resolve selected token from user prefs; fall back to default
                        let token_label = if let Some(token) =
                            bot_deps.payment.get_payment_token_session(id.to_string())
                        {
                            token.label
                        } else {
                            bot_deps.default_payment_prefs.label
                        };

                        let mode_text = prefs
                            .gpt5_mode
                            .as_ref()
                            .map(super::user_model_preferences::dto::gpt5_mode_to_display_string)
                            .unwrap_or("Regular");
                        let verbosity_text = prefs
                            .gpt5_verbosity
                            .as_ref()
                            .map(super::user_model_preferences::dto::verbosity_to_display_string)
                            .unwrap_or("Medium");

                        let text = format!(
                            "⚙️ <b>Your Settings</b>\n\n🤖 Model: {}\n🧩 Mode: {}\n🗣️ Verbosity: {}\n💳 Token: <code>{}</code>",
                            prefs.chat_model.to_display_string(),
                            mode_text,
                            verbosity_text,
                            token_label
                        );

                        let keyboard =
                            InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
                                "↩️ Back to Settings",
                                "back_to_user_settings",
                            )]]);

                        bot.edit_message_text(m.chat.id, m.id, text)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(keyboard)
                            .await?;
                    } else {
                        bot.answer_callback_query(query.id)
                            .text("❌ Username required")
                            .await?;
                    }
                }
            }
        } else if data == "open_payment_settings" {
            // Show submenu with the choose token action and the default currency
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    let mut default_currency = bot_deps.default_payment_prefs.label;

                    let prefs = bot_deps
                        .payment
                        .get_payment_token_session(m.chat.id.to_string());

                    if prefs.is_some() {
                        let prefs = prefs.unwrap();
                        default_currency = prefs.label;
                    }

                    let kb = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            "💳 Choose Payment Token",
                            "payment_selected",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "↩️ Back",
                            "back_to_user_settings",
                        )],
                    ]);
                    bot.edit_message_text(
                        m.chat.id,
                        m.id,
                        format!(
                            "💳 <b>Payment Settings</b>\n\nDefault currency: <code>{}</code>",
                            default_currency
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(kb)
                    .await?;
                }
            }
        } else if data == "back_to_user_settings" {
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
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
                            "↩️ Close",
                            "user_settings_close",
                        )],
                    ]);
                    bot.edit_message_text(m.chat.id, m.id, "⚙️ <b>User Settings</b>\n\n• Manage your model, view current settings, and configure payment.\n\n💡 If no payment token is selected, the on-chain default will be used.")
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(kb)
                        .await?;
                }
            }
        } else if data == "user_settings_close" {
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    let _ = bot.edit_message_reply_markup(m.chat.id, m.id).await;
                    bot.answer_callback_query(query.id).text("Closed").await?;
                }
            }
        } else if data == "open_group_payment_settings" {
            // Show group payment settings submenu
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;

                    if !is_admin {
                        bot.answer_callback_query(query.id)
                            .text("❌ Only administrators can manage group payment settings")
                            .await?;
                        return Ok(());
                    }

                    let default_currency = match bot_deps.panora.aptos.get_token_address().await {
                        Ok(addr) => addr,
                        Err(_) => "0x1::aptos_coin::AptosCoin".to_string(),
                    };
                    let kb = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            "💳 Choose Group Payment Token",
                            "payment_selected",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "↩️ Back",
                            "back_to_group_settings",
                        )],
                    ]);
                    bot.edit_message_text(
                        m.chat.id,
                        m.id,
                        format!(
                            "💳 <b>Group Payment Settings</b>\n\nDefault currency: <code>{}</code>",
                            default_currency
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(kb)
                    .await?;
                }
            }
        } else if data == "open_dao_preferences" {
            // Open DAO preferences menu
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    // Check if user is admin
                    let admins = bot.get_chat_administrators(m.chat.id).await?;
                    let requester_id = query.from.id;
                    let is_admin = admins.iter().any(|member| member.user.id == requester_id);

                    if !is_admin {
                        bot.answer_callback_query(query.id)
                            .text("❌ Only administrators can manage DAO preferences")
                            .await?;
                        return Ok(());
                    }

                    let group_id = m.chat.id.to_string();
                    let group_id_formatted =
                        format!("{}-{}", group_id, bot_deps.group.account_seed);

                    // Get current DAO admin preferences
                    let dao_admin_preferences = bot_deps
                        .dao
                        .get_dao_admin_preferences(group_id_formatted.clone());
                    let current_prefs = match dao_admin_preferences {
                        Ok(prefs) => prefs,
                        Err(_) => {
                            // Create default preferences if none exist
                            use crate::dao::dto::DaoAdminPreferences;
                            let default_prefs = DaoAdminPreferences {
                                group_id: group_id_formatted.clone(),
                                expiration_time: 7 * 24 * 60 * 60, // 7 days in seconds
                                interval_active_proposal_notifications: 60 * 60, // 1 hour in seconds
                                interval_dao_results_notifications: 3600,
                                default_dao_token: None,
                                vote_duration: Some(24 * 60 * 60), // Default to 24 hours
                            };

                            // Save default preferences
                            if let Err(_e) = bot_deps.dao.set_dao_admin_preferences(
                                group_id_formatted.clone(),
                                default_prefs.clone(),
                            ) {
                                bot.answer_callback_query(query.id)
                                    .text("❌ Error creating default DAO preferences")
                                    .await?;
                                return Ok(());
                            }
                            default_prefs
                        }
                    };

                    // Create keyboard with current values
                    let keyboard = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            format!(
                                "🗑️ Deletion After Conclusion: {}",
                                utils::format_time_duration(current_prefs.expiration_time)
                            ),
                            format!("dao_set_expiration_{}", group_id_formatted),
                        )],
                        vec![InlineKeyboardButton::callback(
                            format!(
                                "🔔 Notification Interval: {}",
                                utils::format_time_duration(
                                    current_prefs.interval_active_proposal_notifications
                                )
                            ),
                            format!("dao_set_notifications_{}", group_id_formatted),
                        )],
                        vec![InlineKeyboardButton::callback(
                            format!(
                                "🔔 Results Notification: {}",
                                utils::format_time_duration(
                                    current_prefs.interval_dao_results_notifications
                                )
                            ),
                            format!("dao_set_results_notifications_{}", group_id_formatted),
                        )],
                        vec![InlineKeyboardButton::callback(
                            format!(
                                "🗳️ Vote Duration: {}",
                                utils::format_time_duration(
                                    current_prefs.vote_duration.unwrap_or(24 * 60 * 60)
                                )
                            ),
                            format!("dao_set_vote_duration_{}", group_id_formatted),
                        )],
                        vec![InlineKeyboardButton::callback(
                            "↩️ Back",
                            "back_to_group_settings",
                        )],
                    ]);

                    bot.edit_message_text(
                        m.chat.id,
                        m.id,
                        "🏛️ <b>DAO Preferences</b>\n\nConfigure group DAO settings:",
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(keyboard)
                    .await?;
                }
            }
        } else if data == "open_migrate_group_id" {
            // Handle group ID migration
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    // Check if user is admin
                    let admins = bot.get_chat_administrators(m.chat.id).await?;
                    let requester_id = query.from.id;
                    let is_admin = admins.iter().any(|member| member.user.id == requester_id);

                    if !is_admin {
                        bot.answer_callback_query(query.id)
                            .text("❌ Only administrators can migrate group ID")
                            .await?;
                        return Ok(());
                    }

                    let group_credentials = bot_deps.group.get_credentials(m.chat.id);
                    if group_credentials.is_none() {
                        bot.answer_callback_query(query.id)
                            .text("❌ Group not found")
                            .await?;
                        return Ok(());
                    }

                    let group_credentials = group_credentials.unwrap();
                    let transaction_response = bot_deps
                        .service
                        .migrate_group_id(group_credentials.jwt)
                        .await;

                    match transaction_response {
                        Ok(_response) => {
                            // Show popup notification
                            bot.answer_callback_query(query.id.clone())
                                .text("✅ Group ID migrated successfully!")
                                .await?;

                            // Return to group settings menu
                            let kb = InlineKeyboardMarkup::new(vec![
                                vec![InlineKeyboardButton::callback(
                                    "💳 Payment Settings",
                                    "open_group_payment_settings",
                                )],
                                vec![InlineKeyboardButton::callback(
                                    "🏛️ DAO Preferences",
                                    "open_dao_preferences",
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

                            bot.edit_message_text(
                                m.chat.id,
                                m.id,
                                "⚙️ <b>Group Settings</b>\n\n• Configure payment token, DAO preferences, and group migration.\n\n💡 Only group administrators can access these settings."
                            )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(kb)
                            .await?;
                        }
                        Err(e) => {
                            bot.answer_callback_query(query.id)
                                .text(&format!("❌ Error migrating group ID: {}", e))
                                .await?;
                        }
                    }
                }
            }
        } else if data == "back_to_group_settings" {
            // Return to main group settings menu
            if let Some(message) = &query.message {
                let is_admin = utils::is_admin(&bot, message.chat().id, query.from.id).await;

                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage group settings")
                        .await?;
                    return Ok(());
                }

                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    let kb = InlineKeyboardMarkup::new(vec![
                        vec![InlineKeyboardButton::callback(
                            "💳 Payment Settings",
                            "open_group_payment_settings",
                        )],
                        vec![InlineKeyboardButton::callback(
                            "🏛️ DAO Preferences",
                            "open_dao_preferences",
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
                    bot.edit_message_text(m.chat.id, m.id, "⚙️ <b>Group Settings</b>\n\n• Configure payment token, DAO preferences, and group migration.\n\n💡 Only group administrators can access these settings.")
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(kb)
                        .await?;
                }
            }
        } else if data == "group_settings_close" {
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                    let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;

                    if !is_admin {
                        bot.answer_callback_query(query.id)
                            .text("❌ Only administrators can manage group settings")
                            .await?;
                        return Ok(());
                    }
                    let _ = bot.edit_message_reply_markup(m.chat.id, m.id).await;
                    bot.answer_callback_query(query.id).text("Closed").await?;
                }
            }
        } else if data == "dao_preferences_done"
            || data.starts_with("dao_set_expiration_")
            || data.starts_with("dao_set_notifications_")
            || data.starts_with("dao_set_results_notifications_")
            || data.starts_with("dao_set_token_")
            || data.starts_with("dao_set_vote_duration_")
            || data.starts_with("dao_manage_disabled_")
            || data.starts_with("dao_enable_notifications_")
            || data.starts_with("dao_exp_")
            || data.starts_with("dao_notif_")
            || data.starts_with("dao_res_notif_")
            || data.starts_with("dao_vote_duration_")
            || data == "dao_preferences_back"
        {
            // Handle DAO preferences callbacks
            handle_dao_preference_callback(bot, query, bot_deps).await?;
        } else if data == "disable_notifications" {
            // Handle disable notifications callback
            handle_disable_notifications_callback(bot, query, bot_deps).await?;
        } else if data == "voting_help" {
            // Handle voting help callback
            bot.answer_callback_query(query.id)
                .text("📱 Mini App: Opens voting interface inside Telegram\n🌐 Browser: Opens voting page in external browser\n\nBoth options work the same way!")
                .show_alert(true)
                .await?;
        } else if data.starts_with("pay_accept:") || data.starts_with("pay_reject:") {
            // Handle payment confirmation callbacks
            handle_payment_callback(bot, query, bot_deps).await?;
        } else if data.starts_with("sched_") {
            // Handle scheduled prompts wizard and management callbacks
            handle_scheduled_prompts_callback(bot, query, bot_deps).await?;
        } else if data == "open_payment_settings"
            || data == "open_group_payment_settings"
            || data == "payment_selected"
            || data.starts_with("pay_tokpage:")
            || data.starts_with("pay_selid-")
        {
            // Handle all payment-related callbacks
            crate::payment::handler::handle_payment(bot, query, bot_deps).await?;
        } else {
            bot.answer_callback_query(query.id)
                .text("❌ Unknown action")
                .await?;
        }
    } else {
        bot.answer_callback_query(query.id)
            .text("❌ No action specified")
            .await?;
    }

    Ok(())
}

pub async fn handle_payment_callback(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    let data = query.data.as_ref().unwrap();

    // Parse callback data: pay_accept:user_id:group_id:transaction_id or pay_reject:user_id:group_id:transaction_id
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 4 {
        bot.answer_callback_query(query.id)
            .text("❌ Invalid callback data")
            .await?;
        return Ok(());
    }

    let action = parts[0];
    let user_id = parts[1].parse::<i64>();
    let group_id_i64 = parts[2].parse::<i64>();
    let transaction_id = parts[3].to_string();

    if user_id.is_err() || group_id_i64.is_err() {
        bot.answer_callback_query(query.id)
            .text("❌ Invalid user or group ID")
            .await?;
        return Ok(());
    }

    let user_id = user_id.unwrap();
    let group_id_i64 = group_id_i64.unwrap();

    // SECURITY CHECK: Verify that the user clicking the button is authorized
    let callback_user_id = query.from.id.0 as i64;

    // Only the original requester can confirm/cancel transactions (both individual and group context)
    if callback_user_id != user_id {
        bot.answer_callback_query(query.id)
            .text("❌ Only the user who requested this transaction can confirm or cancel it")
            .await?;
        return Ok(());
    }

    // Convert group_id back to Option<i64> format used by pending_transactions
    let group_id_opt = if group_id_i64 == 0 {
        None
    } else {
        Some(group_id_i64)
    };

    // Get the pending transaction
    let pending_transaction = bot_deps
        .pending_transactions
        .get_pending_transaction(user_id, group_id_opt);

    if pending_transaction.is_none() {
        bot.answer_callback_query(query.id)
            .text("❌ No pending transaction found")
            .await?;

        // Edit the message to remove buttons
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                if let Err(e) = bot.edit_message_reply_markup(msg.chat.id, msg.id).await {
                    log::warn!("Failed to clear reply markup: {}", e);
                }
            }
        }
        return Ok(());
    }

    let pending_transaction = pending_transaction.unwrap();

    // TRANSACTION ID VALIDATION: Verify that the callback transaction ID matches the stored transaction
    if pending_transaction.transaction_id != transaction_id {
        bot.answer_callback_query(query.id)
            .text("❌ Transaction ID mismatch - invalid callback")
            .await?;

        log::warn!(
            "Transaction ID mismatch: callback={}, stored={}",
            transaction_id,
            pending_transaction.transaction_id
        );
        return Ok(());
    }

    // Check if transaction has expired
    if crate::pending_transactions::handler::PendingTransactions::is_expired(&pending_transaction) {
        // Remove expired transaction
        if let Err(e) = bot_deps
            .pending_transactions
            .delete_pending_transaction(user_id, group_id_opt)
        {
            log::warn!("Failed to delete expired pending transaction: {}", e);
        }

        bot.answer_callback_query(query.id)
            .text("❌ Transaction has expired (1 minute timeout)")
            .await?;

        // Edit the message to show expiration
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                let recipients_text = if pending_transaction.original_usernames.len() == 1 {
                    format!("@{}", pending_transaction.original_usernames[0])
                } else {
                    pending_transaction
                        .original_usernames
                        .iter()
                        .map(|username| format!("@{}", username))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let expired_message = format!(
                    "⏰ <b>Transaction expired</b>\n\n💰 {:.2} {} to {} was not sent.\n\n<i>Transactions expire after 1 minute for security.</i>",
                    pending_transaction.per_user_amount
                        * pending_transaction.original_usernames.len() as f64,
                    pending_transaction.symbol,
                    recipients_text
                );

                bot.edit_message_text(msg.chat.id, msg.id, expired_message)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
        }
        return Ok(());
    }

    match action {
        "pay_accept" => {
            // Execute the transaction
            let pay_request =
                crate::pending_transactions::handler::PendingTransactions::to_pay_users_request(
                    &pending_transaction,
                );

            let result = if pending_transaction.is_group_transfer {
                bot_deps
                    .service
                    .pay_members(pending_transaction.jwt_token, pay_request)
                    .await
            } else {
                bot_deps
                    .service
                    .pay_users(pending_transaction.jwt_token, pay_request)
                    .await
            };

            match result {
                Ok(response) => {
                    // Delete the pending transaction ONLY after successful payment
                    if let Err(e) = bot_deps
                        .pending_transactions
                        .delete_pending_transaction(user_id, group_id_opt)
                    {
                        log::warn!("Failed to delete pending transaction after payment: {}", e);
                    }

                    let network = std::env::var("APTOS_NETWORK")
                        .unwrap_or("mainnet".to_string())
                        .to_lowercase();

                    let recipients_text = if pending_transaction.original_usernames.len() == 1 {
                        format!("@{}", pending_transaction.original_usernames[0])
                    } else {
                        pending_transaction
                            .original_usernames
                            .iter()
                            .map(|username| format!("@{}", username))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    let success_message = format!(
                        "✅ <b>Payment sent successfully!</b>\n\n💰 {:.2} {} sent to {} ({:.2} each)\n\n🔗 <a href=\"https://explorer.aptoslabs.com/txn/{}?network={}\">View transaction</a>",
                        pending_transaction.per_user_amount
                            * pending_transaction.original_usernames.len() as f64,
                        pending_transaction.symbol,
                        recipients_text,
                        pending_transaction.per_user_amount,
                        response.hash,
                        network
                    );

                    // Edit the original message
                    if let Some(message) = &query.message {
                        if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                            bot.edit_message_text(msg.chat.id, msg.id, success_message)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                        }
                    }

                    bot.answer_callback_query(query.id)
                        .text("✅ Payment executed successfully!")
                        .await?;
                }
                Err(e) => {
                    let error_message = format!("❌ <b>Payment failed</b>\n\n{}", e);

                    // Edit the original message
                    if let Some(message) = &query.message {
                        if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                            bot.edit_message_text(msg.chat.id, msg.id, error_message)
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                        }
                    }

                    bot.answer_callback_query(query.id)
                        .text("❌ Payment failed")
                        .await?;
                }
            }
        }
        "pay_reject" => {
            // Delete the pending transaction
            if let Err(e) = bot_deps
                .pending_transactions
                .delete_pending_transaction(user_id, group_id_opt)
            {
                log::warn!("Failed to delete pending transaction on cancel: {}", e);
            }

            let recipients_text = if pending_transaction.original_usernames.len() == 1 {
                format!("@{}", pending_transaction.original_usernames[0])
            } else {
                pending_transaction
                    .original_usernames
                    .iter()
                    .map(|username| format!("@{}", username))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let cancel_message = format!(
                "❌ <b>Payment cancelled</b>\n\n💰 {:.2} {} to {} was not sent.",
                pending_transaction.per_user_amount
                    * pending_transaction.original_usernames.len() as f64,
                pending_transaction.symbol,
                recipients_text
            );

            // Edit the original message
            if let Some(message) = &query.message {
                if let teloxide::types::MaybeInaccessibleMessage::Regular(msg) = message {
                    bot.edit_message_text(msg.chat.id, msg.id, cancel_message)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
            }

            bot.answer_callback_query(query.id)
                .text("❌ Payment cancelled")
                .await?;
        }
        _ => {
            bot.answer_callback_query(query.id)
                .text("❌ Unknown action")
                .await?;
        }
    }

    Ok(())
}
