//! Command handlers for quark_bot Telegram bot.

use crate::assets::command_image_collector::CommandImageCollector;
use crate::credentials::dto::Credentials;
use crate::credentials::helpers::save_credentials;
use crate::utils;
use anyhow::Result as AnyResult;
use quark_core::helpers::jwt::JwtManager;
use quark_core::{
    ai::{handler::AI, vector_store::list_user_files_with_names},
    helpers::bot_commands::Command,
    user_conversation::handler::UserConversations,
};
use regex;
use sled::{Db, Tree};
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, InputFile};
use teloxide::{net::Download, utils::command::BotCommands};
use tokio::fs::File;
use tokio::time::sleep;
use teloxide::types::ParseMode;

pub async fn handle_login_user(bot: Bot, msg: Message, db: Tree) -> AnyResult<()> {
    // Ensure this command is used in a private chat (DM)
    if !msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ This command can only be used in a private chat with the bot.")
            .await?;
        return Ok(());
    }
    let user = msg.from.clone();

    if let Some(user) = user {
        let username = user.username;

        if username.is_none() {
            return Err(anyhow::anyhow!("Username not found"));
        }

        let username = username.unwrap();

        // Generate JWT token
        let jwt_manager = JwtManager::new();
        match jwt_manager.generate_token(user.id) {
            Ok(token) => {
                let credentials = Credentials::from((token, user.id));

                let saved = save_credentials(&username, credentials, db);

                if saved.is_err() {
                    bot.send_message(msg.chat.id, "❌ Failed to save credentials")
                        .await?;
                }

                bot.send_message(
                    msg.chat.id,
                    "✅ Successfully logged in! You can now use commands like /c.",
                )
                .await?;
            }
            Err(e) => {
                bot.send_message(msg.chat.id, &format!("❌ Login failed: {}", e))
                    .await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, "❌ Unable to identify user for login.")
            .await?;
    }
    Ok(())
}

pub async fn handle_login_group(bot: Bot, msg: Message) -> AnyResult<()> {
    // Ensure this command is used in a group chat
    if msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ This command must be used in a group chat.")
            .await?;
        return Ok(());
    }

    // Allow only group administrators to invoke
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let requester_id = msg.from.as_ref().map(|u| u.id);
    if let Some(uid) = requester_id {
        let is_admin = admins.iter().any(|member| member.user.id == uid);
        if !is_admin {
            bot.send_message(msg.chat.id, "❌ Only group administrators can use this command.")
                .await?;
            return Ok(());
        }
    } else {
        // Cannot identify sender; deny action
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    // TODO: implement actual group login flow
    bot.send_message(msg.chat.id, "👍 Group login acknowledged (feature under development).")
        .await?;
    Ok(())
}

pub async fn handle_help(bot: Bot, msg: Message) -> AnyResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

pub async fn handle_add_files(bot: Bot, msg: Message) -> AnyResult<()> {
    if !msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ Please DM the bot to upload files.")
            .await?;
        return Ok(());
    }
    bot.send_message(msg.chat.id, "📎 Please attach the files you wish to upload in your next message.\n\n✅ Supported: Documents, Photos, Videos, Audio files\n💡 You can send multiple files in one message!").await?;
    Ok(())
}

pub async fn handle_list_files(
    bot: Bot,
    msg: Message,
    db: Db,
    user_convos: UserConversations,
) -> AnyResult<()> {
    if !msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ Please DM the bot to list your files.")
            .await?;
        return Ok(());
    }
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    if let Some(_vector_store_id) = user_convos.get_vector_store_id(user_id) {
        match list_user_files_with_names(user_id, &db) {
            Ok(files) => {
                if files.is_empty() {
                    bot.send_message(msg.chat.id, "📁 <b>Your Document Library</b>\n\n<i>No files uploaded yet</i>\n\n💡 Use /add_files to start building your personal AI knowledge base!")
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
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
                    use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
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
                        let clear_all_button =
                            InlineKeyboardButton::callback("🗑️ Clear All Files", "clear_all_files");
                        keyboard_rows.push(vec![clear_all_button]);
                    }
                    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);
                    bot.send_message(msg.chat.id, response)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(keyboard)
                        .await?;
                }
            }
            Err(e) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "❌ <b>Error accessing your files</b>\n\n<i>Technical details:</i> {}",
                        e
                    ),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, "🆕 <b>Welcome to Your Document Library!</b>\n\n<i>No documents uploaded yet</i>\n\n💡 Use /add_files to upload your first files and start building your AI-powered knowledge base!")
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
    }
    Ok(())
}

pub async fn handle_chat(bot: Bot, msg: Message, ai: AI, db: Db, prompt: String) -> AnyResult<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let chat_id = msg.chat.id;

    // --- Extract image URL from reply ---
    let mut image_url_from_reply: Option<String> = None;
    if let Some(reply) = msg.reply_to_message() {
        if let Some(from) = reply.from.as_ref() {
            if from.is_bot {
                let reply_text = reply.text().or_else(|| reply.caption());
                if let Some(text) = reply_text {
                    // A simple regex to find the GCS URL
                    if let Ok(re) = regex::Regex::new(
                        r"https://storage\.googleapis\.com/sshift-gpt-bucket/[^\s]+",
                    ) {
                        if let Some(mat) = re.find(text) {
                            image_url_from_reply = Some(mat.as_str().to_string());
                        }
                    }
                }
            }
        }
    }

    // --- Download user-attached images ---
    let mut user_uploaded_image_paths: Vec<(String, String)> = Vec::new();
    if let Some(photos) = msg.photo() {
        // Process all photos, not just the last one
        for photo in photos {
            let file_id = &photo.file.id;
            let file_info = bot.get_file(file_id).await?;
            let extension = file_info
                .path
                .split('.')
                .last()
                .unwrap_or("jpg")
                .to_string();
            let temp_path = format!("/tmp/{}_{}.{}", user_id, photo.file.unique_id, extension);
            let mut file = File::create(&temp_path)
                .await
                .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
            bot.download_file(&file_info.path, &mut file)
                .await
                .map_err(|e| teloxide::RequestError::from(e))?;
            user_uploaded_image_paths.push((temp_path, extension));
        }
    }

    // --- Upload user images to GCS ---
    let mut user_uploaded_image_urls: Vec<String> = Vec::new();
    if !user_uploaded_image_paths.is_empty() {
        match ai.upload_user_images(user_uploaded_image_paths).await {
            Ok(urls) => {
                user_uploaded_image_urls = urls;
            }
            Err(e) => {
                log::error!("Failed to upload user images: {}", e);
                bot.send_message(
                    chat_id,
                    "Sorry, I couldn't upload your image. Please try again.",
                )
                .await?;
                // We should probably stop execution here
                return Ok(());
            }
        }
    }

    // --- Typing Indicator Task ---
    let bot_clone = bot.clone();
    let typing_indicator_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = bot_clone
                .send_chat_action(chat_id, ChatAction::Typing)
                .await
            {
                log::warn!("Failed to send typing action: {}", e);
                break;
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    let ai_response_result = ai
        .generate_response(
            user_id,
            &prompt,
            &db,
            image_url_from_reply,
            user_uploaded_image_urls,
        )
        .await;

    // Stop the typing indicator task
    typing_indicator_handle.abort();

    // --- Handle AI Response ---
    match ai_response_result {
        Ok(ai_response) => {
            if let Some(image_bytes) = ai_response.image_data {
                let photo = InputFile::memory(image_bytes);
                let mut request = bot.send_photo(chat_id, photo);
                if !ai_response.text.is_empty() {
                    let formatted = crate::utils::markdown_to_html(&ai_response.text);
                    request = request.caption(formatted).parse_mode(ParseMode::Html);
                }
                request.await?;
            } else {
                if !ai_response.text.is_empty() {
                    let formatted = crate::utils::markdown_to_html(&ai_response.text);
                    bot.send_message(chat_id, formatted).parse_mode(ParseMode::Html).await?;
                }
            }
        }
        Err(e) => {
            let error_message = format!("[AI error]: {}", e);
            bot.send_message(chat_id, error_message).await?;
        }
    };
    
    Ok(())
}

pub async fn handle_grouped_chat(
    bot: Bot,
    messages: Vec<Message>,
    db: Db,
    ai: AI,
) -> AnyResult<()> {
    // Assumption: all messages have the same chat_id and from user.
    let first_msg = if let Some(msg) = messages.first() {
        msg
    } else {
        return Ok(()); // Should not happen
    };

    let user_id = first_msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let chat_id = first_msg.chat.id;

    // Find the caption from any of the messages
    let caption = messages.iter().find_map(|m| m.caption()).unwrap_or("");

    // --- Download all user-attached images ---
    let mut user_uploaded_image_paths: Vec<(String, String)> = Vec::new();
    for msg in &messages {
        if let Some(photos) = msg.photo() {
            // Process all photos in each message, not just the last one
            for photo in photos {
                let file_id = &photo.file.id;
                let file_info = bot.get_file(file_id).await?;
                let extension = file_info
                    .path
                    .split('.')
                    .last()
                    .unwrap_or("jpg")
                    .to_string();
                let temp_path = format!("/tmp/{}_{}.{}", user_id, photo.file.unique_id, extension);
                let mut file = File::create(&temp_path)
                    .await
                    .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                bot.download_file(&file_info.path, &mut file)
                    .await
                    .map_err(|e| teloxide::RequestError::from(e))?;
                user_uploaded_image_paths.push((temp_path, extension));
            }
        }
    }

    // --- Upload user images to GCS ---
    let mut user_uploaded_image_urls: Vec<String> = Vec::new();
    if !user_uploaded_image_paths.is_empty() {
        match ai.upload_user_images(user_uploaded_image_paths).await {
            Ok(urls) => {
                user_uploaded_image_urls = urls;
            }
            Err(e) => {
                log::error!("Failed to upload user images: {}", e);
                bot.send_message(
                    chat_id,
                    "Sorry, I couldn't upload your images. Please try again.",
                )
                .await?;
                return Ok(());
            }
        }
    }

    // No need to check for replies in a media group context.
    let image_url_from_reply: Option<String> = None;

    // --- Typing Indicator Task ---
    let bot_clone = bot.clone();
    let typing_indicator_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = bot_clone
                .send_chat_action(chat_id, ChatAction::Typing)
                .await
            {
                log::warn!("Failed to send typing action: {}", e);
                break;
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    let ai_response_result = ai
        .generate_response(
            user_id,
            caption,
            &db,
            image_url_from_reply,
            user_uploaded_image_urls,
        )
        .await;

    typing_indicator_handle.abort();

    // --- Handle AI Response ---
    match ai_response_result {
        Ok(ai_response) => {
            if let Some(image_bytes) = ai_response.image_data {
                let photo = InputFile::memory(image_bytes);
                let mut request = bot.send_photo(chat_id, photo);
                if !ai_response.text.is_empty() {
                    let formatted = crate::utils::markdown_to_html(&ai_response.text);
                    request = request.caption(formatted).parse_mode(ParseMode::Html);
                }
                request.await?;
            } else {
                if !ai_response.text.is_empty() {
                    let formatted = crate::utils::markdown_to_html(&ai_response.text);
                    bot.send_message(chat_id, formatted).parse_mode(ParseMode::Html).await?;
                }
            }
        }
        Err(e) => {
            let error_message = format!("[AI error]: {}", e);
            bot.send_message(chat_id, error_message).await?;
        }
    };

    Ok(())
}

pub async fn handle_new_chat(
    bot: Bot,
    msg: Message,
    user_convos: UserConversations,
) -> AnyResult<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;

    match user_convos.clear_response_id(user_id) {
        Ok(_) => {
            bot.send_message(msg.chat.id, "🆕 <b>New conversation started!</b>\n\n✨ Your previous chat history has been cleared. Your next /chat command will start a fresh conversation thread.\n\n💡 <i>Your uploaded files and settings remain intact</i>")
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "❌ <b>Error starting new chat</b>\n\n<i>Technical details:</i> {}",
                    e
                ),
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
    }
    Ok(())
}
