//! Command handlers for quark_bot Telegram bot.
use crate::{
    assets::{
        command_image_collector::CommandImageCollector, handler::handle_file_upload,
        media_aggregator::MediaGroupAggregator,
    },
    bot::hooks::{fund_account_hook, withdraw_funds_hook},
    credentials::dto::CredentialsPayload,
    utils,
};
use anyhow::Result as AnyResult;

use crate::{
    ai::{handler::AI, vector_store::list_user_files_with_names},
    credentials::helpers::generate_new_jwt,
    user_conversation::handler::UserConversations,
    user_model_preferences::handler::{UserModelPreferences, initialize_user_preferences},
};

use open_ai_rust_responses_by_sshift::Model;
use open_ai_rust_responses_by_sshift::types::Effort;
use open_ai_rust_responses_by_sshift::types::{ReasoningParams, SummarySetting};
use quark_core::helpers::{bot_commands::Command, jwt::JwtManager};
use regex;
use reqwest::Url;
use sled::{Db, Tree};
use std::time::Duration;
use std::{env, sync::Arc};
use teloxide::types::{
    ChatAction, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, WebAppInfo,
};
use teloxide::types::{KeyboardMarkup, ParseMode};
use teloxide::{net::Download, utils::command::BotCommands};
use teloxide::{
    prelude::*,
    types::{ButtonRequest, KeyboardButton},
};
use tokio::fs::File;
use tokio::time::sleep;

const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Split a message into chunks that fit within Telegram's message limit
fn split_message(text: &str) -> Vec<String> {
    if text.len() <= TELEGRAM_MESSAGE_LIMIT {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    // Split by lines first to avoid breaking in the middle of sentences
    for line in text.lines() {
        // If adding this line would exceed the limit, save current chunk and start new one
        if current_chunk.len() + line.len() + 1 > TELEGRAM_MESSAGE_LIMIT {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                current_chunk.clear();
            }

            // If a single line is too long, split it by words
            if line.len() > TELEGRAM_MESSAGE_LIMIT {
                let words: Vec<&str> = line.split_whitespace().collect();
                let mut word_chunk = String::new();

                for word in words {
                    if word_chunk.len() + word.len() + 1 > TELEGRAM_MESSAGE_LIMIT {
                        if !word_chunk.is_empty() {
                            chunks.push(word_chunk.trim().to_string());
                            word_chunk.clear();
                        }
                    }

                    if !word_chunk.is_empty() {
                        word_chunk.push(' ');
                    }
                    word_chunk.push_str(word);
                }

                if !word_chunk.is_empty() {
                    current_chunk = word_chunk;
                }
            } else {
                current_chunk = line.to_string();
            }
        } else {
            if !current_chunk.is_empty() {
                current_chunk.push('\n');
            }
            current_chunk.push_str(line);
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

/// Send a potentially long message, splitting it into multiple messages if necessary
async fn send_long_message(bot: &Bot, chat_id: ChatId, text: &str) -> AnyResult<()> {
    // Convert markdown to HTML to avoid Telegram parsing issues
    let html_text = utils::markdown_to_html(text);
    let chunks = split_message(&html_text);

    for (i, chunk) in chunks.iter().enumerate() {
        if i > 0 {
            // Small delay between messages to avoid rate limiting
            sleep(Duration::from_millis(100)).await;
        }

        bot.send_message(chat_id, chunk)
            .parse_mode(ParseMode::Html)
            .await?;
    }

    Ok(())
}

pub async fn handle_aptos_connect(bot: Bot, msg: Message) -> AnyResult<()> {
    if !msg.chat.is_private() {
        bot.send_message(
            msg.chat.id,
            "❌ This command can only be used in a private chat with the bot.",
        )
        .await?;
    }

    let aptos_connect_url = "https://aptosconnect.app";

    let url = Url::parse(&aptos_connect_url).expect("Invalid URL");
    let web_app_info = WebAppInfo { url };

    let aptos_connect_button = InlineKeyboardButton::web_app("Open Aptos Connect", web_app_info);

    bot.send_message(
        msg.chat.id,
        "Click the button below to login to your quark account",
    )
    .reply_markup(InlineKeyboardMarkup::new(vec![vec![aptos_connect_button]]))
    .await?;

    return Ok(());
}

pub async fn handle_login_user(bot: Bot, msg: Message) -> AnyResult<()> {
    if !msg.chat.is_private() {
        bot.send_message(
            msg.chat.id,
            "❌ This command can only be used in a private chat with the bot.",
        )
        .await?;
        return Ok(());
    }

    let user = msg.from;

    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let user_id = user.unwrap().id;

    let app_url = env::var("APP_URL").expect("APP_URL must be set");
    let url_to_build = format!("{}/login?userId={}", app_url, user_id);

    let url = Url::parse(&url_to_build).expect("Invalid URL");

    let web_app_info = WebAppInfo { url };

    let request = ButtonRequest::WebApp(web_app_info);

    let login_button = KeyboardButton::new("Login to your Quark account");

    let login_button = login_button.request(request);

    let login_markup = KeyboardMarkup::new(vec![vec![login_button]]);

    bot.send_message(
        msg.chat.id,
        "Click the button below to login to your quark account",
    )
    .reply_markup(login_markup)
    .await?;

    return Ok(());
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
            bot.send_message(
                msg.chat.id,
                "❌ Only group administrators can use this command.",
            )
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
    bot.send_message(
        msg.chat.id,
        "👍 Group login acknowledged (feature under development).",
    )
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
                        .parse_mode(ParseMode::Html)
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
            .parse_mode(ParseMode::Html)
            .await?;
    }
    Ok(())
}

pub async fn handle_reasoning_chat(
    bot: Bot,
    msg: Message,
    ai: AI,
    db: Db,
    tree: Tree,
    prompt: String,
) -> AnyResult<()> {
    // --- Start Typing Indicator Immediately ---
    let bot_clone = bot.clone();
    let typing_indicator_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = bot_clone
                .send_chat_action(msg.chat.id, ChatAction::Typing)
                .await
            {
                log::warn!("Failed to send typing action: {}", e);
                break;
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    let user = msg.from.as_ref();

    if user.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let user_id = user.unwrap().id;
    let username = user.unwrap().username.as_ref();
    
    // Load user's reasoning model preferences
    let (reasoning_model, effort) = if let Some(username) = username {
        let prefs_handler = UserModelPreferences::new(&db)?;
        let preferences = prefs_handler.get_preferences(username);
        (preferences.reasoning_model.to_openai_model(), preferences.effort)
    } else {
        // Fallback to defaults if no username
        (Model::O4Mini, Effort::Low)
    };

    // --- Vision Support: Check for replied-to images ---
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
            let file_info = bot.get_file(file_id.clone()).await?;
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
    let user_uploaded_image_urls = match ai.upload_user_images(user_uploaded_image_paths).await {
        Ok(urls) => urls,
        Err(e) => {
            log::error!("Failed to upload user images: {}", e);
            typing_indicator_handle.abort();
            bot.send_message(
                msg.chat.id,
                "Sorry, I couldn't upload your image. Please try again.",
            )
            .await?;
            // We should probably stop execution here
            return Ok(());
        }
    };

    // Asynchronously generate the response
    let response_result = ai
        .generate_response(
            msg.clone(),
            user_id.0 as i64,
            &prompt,
            &db,
            tree,
            image_url_from_reply,
            user_uploaded_image_urls,
            reasoning_model,
            20000,
            None,
            Some(
                ReasoningParams::new()
                    .with_effort(effort)
                    .with_summary(SummarySetting::Detailed),
            ),
        )
        .await;

    typing_indicator_handle.abort();

    match response_result {
        Ok(response) => {
            log::info!("Reasoning response generated successfully");

            // Check for image data and send as a photo if present
            if let Some(image_data) = response.image_data {
                let photo = InputFile::memory(image_data);
                let caption = if response.text.len() > 1024 {
                    &response.text[..1024]
                } else {
                    &response.text
                };
                bot.send_photo(msg.chat.id, photo)
                    .caption(caption)
                    .parse_mode(ParseMode::Markdown)
                    .await?;
                // If the text is longer than 1024, send the rest as a follow-up message
                if response.text.len() > 1024 {
                    send_long_message(&bot, msg.chat.id, &response.text[1024..]).await?;
                }
            } else {
                let text_to_send = if response.text.is_empty() {
                    "_(The model processed the request but returned no text.)_".to_string()
                } else {
                    response.text
                };
                // Use the new send_long_message function for text responses
                send_long_message(&bot, msg.chat.id, &text_to_send).await?;
            }
        }
        Err(e) => {
            log::error!("Error generating reasoning response: {}", e);
            bot.send_message(
                msg.chat.id,
                "Sorry, I encountered an error while processing your reasoning request.",
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn handle_chat(
    bot: Bot,
    msg: Message,
    ai: AI,
    db: Db,
    tree: Tree,
    prompt: String,
) -> AnyResult<()> {
    // --- Start Typing Indicator Immediately ---
    let bot_clone = bot.clone();
    let typing_indicator_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = bot_clone
                .send_chat_action(msg.chat.id, ChatAction::Typing)
                .await
            {
                log::warn!("Failed to send typing action: {}", e);
                break;
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    let user = msg.from.as_ref();

    if user.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let user_id = user.unwrap().id;
    let username = user.unwrap().username.as_ref();
    
    // Load user's chat model preferences
    let (chat_model, temperature) = if let Some(username) = username {
        let prefs_handler = UserModelPreferences::new(&db)?;
        let preferences = prefs_handler.get_preferences(username);
        (preferences.chat_model.to_openai_model(), Some(preferences.temperature))
    } else {
        // Fallback to defaults if no username
        (Model::GPT41Mini, Some(0.6))
    };

    // --- Vision Support: Check for replied-to images ---
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
            let file_info = bot.get_file(file_id.clone()).await?;
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
    let user_uploaded_image_urls = match ai.upload_user_images(user_uploaded_image_paths).await {
        Ok(urls) => urls,
        Err(e) => {
            log::error!("Failed to upload user images: {}", e);
            typing_indicator_handle.abort();
            bot.send_message(
                msg.chat.id,
                "Sorry, I couldn't upload your image. Please try again.",
            )
            .await?;
            // We should probably stop execution here
            return Ok(());
        }
    };

    // Asynchronously generate the response
    let response_result = ai
        .generate_response(
            msg.clone(),
            user_id.0 as i64,
            &prompt,
            &db,
            tree,
            image_url_from_reply,
            user_uploaded_image_urls,
            chat_model,
            8192,
            temperature,
            None,
        )
        .await;

    typing_indicator_handle.abort();

    match response_result {
        Ok(response) => {
            // Check for image data and send as a photo if present
            if let Some(image_data) = response.image_data {
                let photo = InputFile::memory(image_data);
                bot.send_photo(msg.chat.id, photo)
                    .caption(response.text)
                    .parse_mode(ParseMode::Markdown)
                    .await?;
            } else if let Some(ref tool_calls) = response.tool_calls {
                if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "withdraw_funds")
                {
                    withdraw_funds_hook(bot, msg, response.text).await?;
                } else if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "fund_account")
                {
                    fund_account_hook(bot, msg, response.text).await?;
                } else {
                    let html_text = utils::markdown_to_html(&response.text);
                    bot.send_message(msg.chat.id, html_text)
                        .parse_mode(ParseMode::Html)
                        .await?;
                }
            } else {
                let html_text = utils::markdown_to_html(&response.text);
                bot.send_message(msg.chat.id, html_text)
                    .parse_mode(ParseMode::Html)
                    .await?;
            }
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("An error occurred while processing your request: {}", e),
            )
            .await?;
        }
    }

    Ok(())
}

pub async fn handle_grouped_chat(
    bot: Bot,
    messages: Vec<Message>,
    db: Db,
    ai: AI,
    tree: Tree,
) -> AnyResult<()> {
    // Determine the user who initiated the conversation
    let user = messages.first().and_then(|m| m.from.clone());
    if user.is_none() {
        if let Some(first_msg) = messages.first() {
            bot.send_message(first_msg.chat.id, "❌ Unable to identify sender.")
                .await?;
        }
        return Ok(());
    }
    let user_id = user.unwrap().id.0 as i64;
    let representative_msg = messages.first().unwrap().clone();

    // --- Start Typing Indicator Immediately ---
    let bot_clone = bot.clone();
    let chat_id = representative_msg.chat.id;
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

    // --- Download all user-attached images ---
    let mut user_uploaded_image_paths: Vec<(String, String)> = Vec::new();
    for msg in &messages {
        if let Some(photos) = msg.photo() {
            // Process all photos in each message, not just the last one
            for photo in photos {
                let file_id = &photo.file.id;
                let file_info = bot.get_file(file_id.clone()).await?;
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
    let mut all_image_urls = match ai.upload_user_images(user_uploaded_image_paths).await {
        Ok(urls) => urls,
        Err(e) => {
            log::error!("Failed to upload user images: {}", e);
            typing_indicator_handle.abort();
            bot.send_message(
                representative_msg.chat.id,
                "Sorry, I couldn't upload your images. Please try again.",
            )
            .await?;
            return Ok(());
        }
    };

    // Extract all image URLs from the message group (reply or user-uploaded)
    let mut combined_text_input = String::new();

    for msg in &messages {
        // Look for URLs in replies
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
                                all_image_urls.push(mat.as_str().to_string());
                            }
                        }
                    }
                }
            }
        }

        // Aggregate text from all messages
        if let Some(text) = msg.text() {
            if !text.is_empty() {
                if !combined_text_input.is_empty() {
                    combined_text_input.push(' ');
                }
                combined_text_input.push_str(text);
            }
        } else if let Some(caption) = msg.caption() {
            if !caption.is_empty() {
                if !combined_text_input.is_empty() {
                    combined_text_input.push(' ');
                }
                combined_text_input.push_str(caption);
            }
        }
    }

    // Use the aggregated text as the final input
    let final_input = if combined_text_input.is_empty() {
        "Describe the attached images." // Default if no text at all
    } else {
        // Clean up command prefix from the combined text if present
        if let Some(stripped) = combined_text_input.strip_prefix("/c ") {
            stripped
        } else {
            &combined_text_input
        }
    };

    // Asynchronously generate the response
    let response_result = ai
        .generate_response(
            representative_msg.clone(),
            user_id,
            final_input,
            &db,
            tree,
            None,
            all_image_urls,
            Model::GPT41Mini,
            8192,
            Some(0.5),
            None,
        )
        .await;

    typing_indicator_handle.abort();

    match response_result {
        Ok(response) => {
            // Check for image data and send as a photo if present
            if let Some(image_data) = response.image_data {
                let photo = InputFile::memory(image_data);
                bot.send_photo(representative_msg.chat.id, photo)
                    .caption(response.text)
                    .parse_mode(ParseMode::Markdown)
                    .await?;
            } else {
                bot.send_message(representative_msg.chat.id, response.text)
                    .parse_mode(ParseMode::Markdown)
                    .await?;
            }
        }
        Err(e) => {
            bot.send_message(
                representative_msg.chat.id,
                format!("An error occurred while processing your request: {}", e),
            )
            .await?;
        }
    }

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
                .parse_mode(ParseMode::Html)
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
            .parse_mode(ParseMode::Html)
            .await?;
        }
    }
    Ok(())
}

pub async fn handle_web_app_data(bot: Bot, msg: Message, tree: Tree, db: Db) -> AnyResult<()> {
    let web_app_data = msg.web_app_data().unwrap();
    let payload = web_app_data.data.clone();

    let payload = serde_json::from_str::<CredentialsPayload>(&payload);

    if payload.is_err() {
        bot.send_message(msg.chat.id, "❌ Error parsing payload")
            .await?;
        return Ok(());
    };

    let payload = payload.unwrap();

    let user = msg.from;

    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        bot.send_message(msg.chat.id, "❌ Username not found, required for login")
            .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let user_id = user.id;

    let jwt_manager = JwtManager::new();

    generate_new_jwt(
        username.clone(),
        user_id,
        payload.account_address,
        payload.resource_account_address,
        jwt_manager,
        tree,
    )
    .await;

    // Initialize default model preferences for new user
    let _ = initialize_user_preferences(&username, &db).await;

    return Ok(());
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    ai: AI,
    media_aggregator: Arc<MediaGroupAggregator>,
    cmd_collector: Arc<CommandImageCollector>,
    db: Db,
    tree: Tree,
) -> AnyResult<()> {
    if msg.media_group_id().is_some() && msg.photo().is_some() {
        media_aggregator.add_message(msg, ai, tree).await;
        return Ok(());
    }

    // Photo-only message (no text/caption) may belong to a pending command
    if msg.text().is_none() && msg.caption().is_none() && msg.photo().is_some() {
        cmd_collector.try_attach_photo(msg, ai, tree).await;
        return Ok(());
    }

    if msg.caption().is_none()
        && msg.chat.is_private()
        && (msg.document().is_some()
            || msg.photo().is_some()
            || msg.video().is_some()
            || msg.audio().is_some())
    {
        handle_file_upload(bot, msg, db, ai).await?;
    }
    Ok(())
}
