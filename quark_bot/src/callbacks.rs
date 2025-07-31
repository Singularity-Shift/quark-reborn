//! Callback query handlers for quark_bot.

use crate::ai::vector_store::{
    delete_file_from_vector_store, delete_vector_store, list_user_files_with_names,
};
use crate::dependencies::BotDependencies;
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
                        // Update the message to show user was unmuted
                        let updated_text = message
                            .text()
                            .unwrap_or("")
                            .replace("🔇 User has been muted", "🔊 User has been unmuted");

                        bot.edit_message_text(message.chat.id, message.id, updated_text)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;

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
            let user_id_str = data.strip_prefix("ban:").unwrap();
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

                match bot
                    .ban_chat_member(
                        message.chat.id,
                        teloxide::types::UserId(target_user_id as u64),
                    )
                    .await
                {
                    Ok(_) => {
                        // Update the message to show user was banned
                        let updated_text = message
                            .text()
                            .unwrap_or("")
                            .replace("🔇 User has been muted", "🚫 User has been banned");

                        // Remove the buttons since actions are complete
                        bot.edit_message_text(message.chat.id, message.id, updated_text)
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(InlineKeyboardMarkup::new(
                                vec![] as Vec<Vec<InlineKeyboardButton>>
                            ))
                            .await?;

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
            || data.starts_with("select_reasoning_model:")
            || data.starts_with("set_effort:")
        {
            // Handle model preference callbacks
            handle_model_preferences_callback(bot, query, bot_deps.user_model_prefs.clone())
                .await?;
        } else if data == "dao_preferences_done"
            || data.starts_with("dao_set_expiration_")
            || data.starts_with("dao_set_notifications_")
            || data.starts_with("dao_set_results_notifications_")
            || data.starts_with("dao_set_token_")
            || data.starts_with("dao_exp_")
            || data.starts_with("dao_notif_")
            || data.starts_with("dao_res_notif_")
            || data == "dao_preferences_back"
        {
            // Handle DAO preferences callbacks
            crate::dao::handler::handle_dao_preference_callback(bot, query, bot_deps).await?;
        } else if data == "voting_help" {
            // Handle voting help callback
            bot.answer_callback_query(query.id)
                .text("📱 Mini App: Opens voting interface inside Telegram\n🌐 Browser: Opens voting page in external browser\n\nBoth options work the same way!")
                .show_alert(true)
                .await?;
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
