//! Callback query handlers for quark_bot.

use crate::ai::vector_store::{
    delete_file_from_vector_store, delete_vector_store, list_user_files_with_names,
};
use crate::dao::handler::{handle_dao_preference_callback, handle_disable_notifications_callback};
use crate::dependencies::BotDependencies;
// use crate::scheduled_prompts; // not needed directly
use crate::user_model_preferences::callbacks::handle_model_preferences_callback;
use crate::scheduled_prompts::callbacks::handle_scheduled_prompts_callback;
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
            // Support formats: "ban:<user_id>" and "ban:<user_id>:<offending_message_id>"
            let parts: Vec<&str> = data.split(':').collect();
            if parts.len() < 2 { 
                bot.answer_callback_query(query.id)
                    .text("❌ Invalid ban action")
                    .await?;
                return Ok(());
            }
            let target_user_id: i64 = parts[1].parse().unwrap_or(0);
            let offending_message_id: Option<teloxide::types::MessageId> = if parts.len() >= 3 { parts[2].parse::<i32>().ok().map(teloxide::types::MessageId) } else { None };

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
                        // If we know the offending message id, attempt to delete it
                        if let Some(offend_id) = offending_message_id {
                            if let Err(e) = bot.delete_message(message.chat.id, offend_id).await {
                                log::warn!("Failed to delete offending message {}: {}", offend_id.0, e);
                            }
                        } else {
                            // Fallback: try to extract Message ID from the moderation text block
                            if let Some(text) = message.text() {
                                if let Some(start) = text.find("Message ID: ") {
                                    let after = &text[start + "Message ID: ".len()..];
                                    // Attempt to capture the numeric ID between <code> and </code>
                                    if let (Some(code_open), Some(code_close)) = (after.find("<code>"), after.find("</code>")) {
                                        let inner = &after[code_open + "<code>".len()..code_close];
                                        if let Ok(mid) = inner.trim().parse::<i32>() {
                                            if let Err(e) = bot.delete_message(message.chat.id, teloxide::types::MessageId(mid)).await {
                                                log::warn!("Failed fallback delete for offending message {}: {}", mid, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Delete the moderation notification message itself
                        if let Err(e) = bot.delete_message(message.chat.id, message.id).await {
                            log::warn!("Failed to delete moderation message {}: {}", message.id.0, e);
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
    let group_id_opt = if group_id_i64 == 0 { None } else { Some(group_id_i64) };

    // Get the pending transaction
    let pending_transaction = bot_deps.pending_transactions.get_pending_transaction(user_id, group_id_opt);
    
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
        
        log::warn!("Transaction ID mismatch: callback={}, stored={}", transaction_id, pending_transaction.transaction_id);
        return Ok(());
    }
    
    // Check if transaction has expired
    if crate::pending_transactions::handler::PendingTransactions::is_expired(&pending_transaction) {
        // Remove expired transaction
        if let Err(e) = bot_deps.pending_transactions.delete_pending_transaction(user_id, group_id_opt) {
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
                    pending_transaction.original_usernames.iter()
                        .map(|username| format!("@{}", username))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                let expired_message = format!(
                    "⏰ <b>Transaction expired</b>\n\n💰 {:.2} {} to {} was not sent.\n\n<i>Transactions expire after 1 minute for security.</i>",
                    pending_transaction.per_user_amount * pending_transaction.original_usernames.len() as f64,
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
            let pay_request = crate::pending_transactions::handler::PendingTransactions::to_pay_users_request(&pending_transaction);
            
            let result = if pending_transaction.is_group_transfer {
                bot_deps.service.pay_members(pending_transaction.jwt_token, pay_request).await
            } else {
                bot_deps.service.pay_users(pending_transaction.jwt_token, pay_request).await
            };

            match result {
                Ok(response) => {
                    // Delete the pending transaction ONLY after successful payment
                    if let Err(e) = bot_deps.pending_transactions.delete_pending_transaction(user_id, group_id_opt) {
                        log::warn!("Failed to delete pending transaction after payment: {}", e);
                    }
                    
                    let network = std::env::var("APTOS_NETWORK")
                        .unwrap_or("mainnet".to_string())
                        .to_lowercase();

                    let recipients_text = if pending_transaction.original_usernames.len() == 1 {
                        format!("@{}", pending_transaction.original_usernames[0])
                    } else {
                        pending_transaction.original_usernames.iter()
                            .map(|username| format!("@{}", username))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };

                    let success_message = format!(
                        "✅ <b>Payment sent successfully!</b>\n\n💰 {:.2} {} sent to {} ({:.2} each)\n\n🔗 <a href=\"https://explorer.aptoslabs.com/txn/{}?network={}\">View transaction</a>",
                        pending_transaction.per_user_amount * pending_transaction.original_usernames.len() as f64,
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
            if let Err(e) = bot_deps.pending_transactions.delete_pending_transaction(user_id, group_id_opt) {
                log::warn!("Failed to delete pending transaction on cancel: {}", e);
            }

            let recipients_text = if pending_transaction.original_usernames.len() == 1 {
                format!("@{}", pending_transaction.original_usernames[0])
            } else {
                pending_transaction.original_usernames.iter()
                    .map(|username| format!("@{}", username))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let cancel_message = format!(
                "❌ <b>Payment cancelled</b>\n\n💰 {:.2} {} to {} was not sent.",
                pending_transaction.per_user_amount * pending_transaction.original_usernames.len() as f64,
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
