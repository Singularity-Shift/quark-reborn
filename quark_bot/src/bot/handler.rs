//! Command handlers for quark_bot Telegram bot.
use crate::{
    assets::handler::handle_file_upload,
    bot::hooks::{fund_account_hook, pay_users_hook, withdraw_funds_hook},
    credentials::dto::CredentialsPayload,
    dependencies::BotDependencies,
    group::dto::GroupCredentials,
    utils::{self, create_purchase_request},
};
use anyhow::Result as AnyResult;
use aptos_rust_sdk_types::api_types::view::ViewRequest;
use serde_json::value;

use crate::{
    ai::{moderation::ModerationService, vector_store::list_user_files_with_names},
    user_model_preferences::handler::initialize_user_preferences,
};

use open_ai_rust_responses_by_sshift::{
    Model,
    types::{ReasoningParams, SummarySetting},
};
use quark_core::helpers::{
    bot_commands::Command,
    dto::{CreateGroupRequest, PurchaseRequest},
};
use regex;
use reqwest::Url;
use std::env;
use std::time::Duration;
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

pub async fn handle_login_group(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    // Ensure this command is used in a group chat
    if msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ This command must be used in a group chat.")
            .await?;
        return Ok(());
    }

    let account_seed = env::var("ACCOUNT_SEED");

    if account_seed.is_err() {
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let account_seed = account_seed.unwrap();

    // Allow only group administrators to invoke
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let requester_id = msg.from.as_ref().map(|u| u.id);
    let group_id = msg.chat.id;

    let group_id_formatted = format!("{}-{}", msg.chat.id, account_seed);

    let payload: GroupCredentials;

    if let Some(uid) = requester_id {
        let is_admin = admins.iter().any(|member| member.user.id == uid);
        if !is_admin {
            bot.send_message(
                group_id,
                "❌ Only group administrators can use this command.",
            )
            .await?;
            return Ok(());
        }
    } else {
        // Cannot identify sender; deny action
        bot.send_message(group_id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let group_exists = bot_deps
        .group
        .group_exists(group_id, bot_deps.panora.clone())
        .await;

    if !group_exists {
        let group_result = bot_deps
            .service
            .create_group(CreateGroupRequest {
                group_id: group_id_formatted.clone(),
            })
            .await;

        if group_result.is_err() {
            bot.send_message(msg.chat.id, "❌ Unable to create group.")
                .await?;
            return Ok(());
        }
    }

    let jwt = bot_deps.group.generate_new_jwt(group_id);

    if !jwt {
        bot.send_message(group_id, "❌ Unable to generate JWT.")
            .await?;
        return Ok(());
    }

    let payload_response = bot_deps.group.get_credentials(group_id);

    if payload_response.is_none() {
        bot.send_message(group_id, "❌ Unable to get credentials.")
            .await?;
        return Ok(());
    }

    payload = payload_response.unwrap();

    let updated_credentials =
        check_group_resource_account_address(&bot, payload, msg.clone(), &bot_deps).await;

    if updated_credentials.is_err() {
        bot.send_message(msg.chat.id, "❌ Unable to save credentials.")
            .await?;
        return Ok(());
    }

    bot.send_message(msg.chat.id, format!("🔑 <b>Group Login Successful!</b>\n\n<i>You can now use the group's Quark account to interact with the bot.</i>\n\n💡 <i>Use /groupwalletaddress to get the group's wallet address and /groupbalance to get the group's balance of a token.</i>"))
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn handle_help(bot: Bot, msg: Message) -> AnyResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

pub async fn handle_prices(bot: Bot, msg: Message) -> AnyResult<()> {
    let pricing_info = crate::ai::actions::execute_prices(&serde_json::json!({})).await;
    bot.send_message(msg.chat.id, pricing_info)
        .parse_mode(ParseMode::Html)
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

pub async fn handle_list_files(bot: Bot, msg: Message, bot_deps: BotDependencies) -> AnyResult<()> {
    if !msg.chat.is_private() {
        bot.send_message(msg.chat.id, "❌ Please DM the bot to list your files.")
            .await?;
        return Ok(());
    }
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    if let Some(_vector_store_id) = bot_deps.user_convos.get_vector_store_id(user_id) {
        match list_user_files_with_names(user_id, bot_deps.clone()) {
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
    prompt: String,
    bot_deps: BotDependencies,
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

    if username.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(
            msg.chat.id,
            "❌ Unable to verify permissions. Please set username in your user account.",
        )
        .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let credentials = bot_deps.auth.get_credentials(&username);

    if credentials.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(
            msg.chat.id,
            "❌ Unable to verify permissions, please try to login your user account.",
        )
        .await?;
        return Ok(());
    }

    let credentials = credentials.unwrap();

    // Load user's reasoning model preferences
    let preferences = bot_deps.user_model_prefs.get_preferences(username);

    let (reasoning_model, effort) = (
        preferences.reasoning_model.to_openai_model(),
        preferences.effort,
    );

    // --- Vision Support: Check for replied-to images ---
    let mut image_url_from_reply: Option<String> = None;
    // --- Context Support: Check for replied-to message text ---
    let mut replied_message_context: Option<String> = None;
    // --- Image Support: Process replied message images ---
    let mut replied_message_image_paths: Vec<(String, String)> = Vec::new();
    if let Some(reply) = msg.reply_to_message() {
        // Extract text content from replied message (following /mod pattern)
        let reply_text_content = reply.text().or_else(|| reply.caption()).unwrap_or_default();

        if !reply_text_content.is_empty() {
            if let Some(from) = reply.from.as_ref() {
                let username = from
                    .username
                    .as_ref()
                    .map(|u| format!("@{}", u))
                    .unwrap_or_else(|| from.first_name.clone());
                replied_message_context =
                    Some(format!("User {} said: {}", username, reply_text_content));
            } else {
                replied_message_context = Some(format!("Previous message: {}", reply_text_content));
            }
        }

        // Process images from replied message – only keep the largest resolution
        if let Some(photos) = reply.photo() {
            if let Some(photo) = photos.last() {
                let file_id = &photo.file.id;
                let file_info = bot.get_file(file_id.clone()).await?;
                let extension = file_info
                    .path
                    .split('.')
                    .last()
                    .unwrap_or("jpg")
                    .to_string();
                let temp_path = format!(
                    "/tmp/reply_{}_{}.{}",
                    user_id, photo.file.unique_id, extension
                );
                let mut file = File::create(&temp_path)
                    .await
                    .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                bot.download_file(&file_info.path, &mut file)
                    .await
                    .map_err(|e| teloxide::RequestError::from(e))?;
                replied_message_image_paths.push((temp_path, extension));
            }
        }

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
        // Only keep the largest PhotoSize (last element)
        if let Some(photo) = photos.last() {
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

    // --- Upload replied message images to GCS ---
    let replied_message_image_urls = if !replied_message_image_paths.is_empty() {
        match bot_deps
            .ai
            .upload_user_images(replied_message_image_paths)
            .await
        {
            Ok(urls) => urls,
            Err(e) => {
                log::error!("Failed to upload replied message images: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // --- Upload user images to GCS ---
    let user_uploaded_image_urls = match bot_deps
        .ai
        .upload_user_images(user_uploaded_image_paths)
        .await
    {
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

    // --- Combine all image URLs ---
    let mut all_image_urls = user_uploaded_image_urls;
    all_image_urls.extend(replied_message_image_urls);

    // Prepare the final prompt with context if available
    let final_prompt = if let Some(context) = replied_message_context {
        format!("{}\n\nUser asks: {}", context, prompt)
    } else {
        prompt
    };

    // Asynchronously generate the response
    let response_result = bot_deps
        .ai
        .generate_response(
            bot.clone(),
            msg.clone(),
            &final_prompt,
            image_url_from_reply,
            all_image_urls,
            reasoning_model,
            20000,
            None,
            Some(
                ReasoningParams::new()
                    .with_effort(effort)
                    .with_summary(SummarySetting::Detailed),
            ),
            bot_deps.clone(),
            None,
        )
        .await;

    typing_indicator_handle.abort();

    match response_result {
        Ok(ai_response) => {
            // Log tool usage if any tools were used
            let profile = env::var("PROFILE").unwrap_or("prod".to_string());
            let (web_search, file_search, image_gen, _) = ai_response.get_tool_usage_counts();

            if profile != "dev" {
                let response = create_purchase_request(
                    file_search,
                    web_search,
                    image_gen,
                    bot_deps.service.clone(),
                    ai_response.total_tokens,
                    ai_response.model,
                    &credentials.jwt,
                    None,
                )
                .await;

                if response.is_err() {
                    if response.as_ref().err().unwrap().to_string().contains("401")
                        || response.as_ref().err().unwrap().to_string().contains("403")
                    {
                        bot.send_message(
                            msg.chat.id,
                            "Your login has expired. Please login again.",
                        )
                        .await?;
                    } else {
                        bot.send_message(
                        msg.chat.id,
                        "Sorry, I encountered an error while processing your reasoning request.",
                    )
                    .await?;
                    }

                    return Ok(());
                }
            }

            // Check for image data and send as a photo if present
            if let Some(image_data) = ai_response.image_data {
                let photo = InputFile::memory(image_data);
                let caption = if ai_response.text.len() > 1024 {
                    &ai_response.text[..1024]
                } else {
                    &ai_response.text
                };
                bot.send_photo(msg.chat.id, photo)
                    .caption(caption)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                // If the text is longer than 1024, send the rest as a follow-up message
                if ai_response.text.len() > 1024 {
                    send_long_message(&bot, msg.chat.id, &ai_response.text[1024..]).await?;
                }
            } else if let Some(ref tool_calls) = ai_response.tool_calls {
                if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "withdraw_funds")
                {
                    withdraw_funds_hook(bot, msg, ai_response.text).await?;
                } else if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "fund_account")
                {
                    fund_account_hook(bot, msg, ai_response.text).await?;
                } else if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "get_pay_users")
                {
                    // Get transaction_id from the pending transaction - reasoning chat has no group_id
                    let user_id = if let Some(user) = &msg.from {
                        user.id.0 as i64
                    } else {
                        log::warn!("Unable to get user ID for pay_users_hook in reasoning chat");
                        send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
                        return Ok(());
                    };
                    
                    // Reasoning chat is always individual context (no group_id)
                    let group_id_i64 = None;
                    
                    if let Some(pending_transaction) = bot_deps.pending_transactions.get_pending_transaction(user_id, group_id_i64) {
                        pay_users_hook(bot, msg, ai_response.text, None, pending_transaction.transaction_id, bot_deps.clone()).await?;
                    } else {
                        log::warn!("No pending transaction found for user {} in reasoning chat", user_id);
                        send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
                    }
                } else {
                    send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
                }
            } else {
                let text_to_send = if ai_response.text.is_empty() {
                    "_(The model processed the request but returned no text.)_".to_string()
                } else {
                    ai_response.text
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
    prompt: String,
    group_id: Option<String>,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    // Store group_id for later use to avoid move issues
    let group_id_for_hook = group_id.clone();
    
    // --- Start Typing Indicator Immediately ---
    let bot_clone = bot.clone();
    let profile = env::var("PROFILE").unwrap_or("prod".to_string());
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

    if username.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let credentials = bot_deps.auth.get_credentials(&username);
    if credentials.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

    let credentials = credentials.unwrap();

    // Load user's chat model preferences
    let preferences = bot_deps.user_model_prefs.get_preferences(username);

    let (chat_model, temperature) = (
        preferences.chat_model.to_openai_model(),
        Some(preferences.temperature),
    );

    // --- Vision Support: Check for replied-to images ---
    let mut image_url_from_reply: Option<String> = None;
    // --- Context Support: Check for replied-to message text ---
    let mut replied_message_context: Option<String> = None;
    // --- Image Support: Process replied message images ---
    let mut replied_message_image_paths: Vec<(String, String)> = Vec::new();
    if let Some(reply) = msg.reply_to_message() {
        // Extract text content from replied message (following /mod pattern)
        let reply_text_content = reply.text().or_else(|| reply.caption()).unwrap_or_default();

        if !reply_text_content.is_empty() {
            if let Some(from) = reply.from.as_ref() {
                let username = from
                    .username
                    .as_ref()
                    .map(|u| format!("@{}", u))
                    .unwrap_or_else(|| from.first_name.clone());
                replied_message_context =
                    Some(format!("User {} said: {}", username, reply_text_content));
            } else {
                replied_message_context = Some(format!("Previous message: {}", reply_text_content));
            }
        }

        // Process images from replied message – only take the largest resolution (last PhotoSize)
        if let Some(photos) = reply.photo() {
            if let Some(photo) = photos.last() {
                let file_id = &photo.file.id;
                let file_info = bot.get_file(file_id.clone()).await?;
                let extension = file_info
                    .path
                    .split('.')
                    .last()
                    .unwrap_or("jpg")
                    .to_string();
                let temp_path = format!(
                    "/tmp/reply_{}_{}.{}",
                    user_id, photo.file.unique_id, extension
                );
                let mut file = File::create(&temp_path)
                    .await
                    .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
                bot.download_file(&file_info.path, &mut file)
                    .await
                    .map_err(|e| teloxide::RequestError::from(e))?;
                replied_message_image_paths.push((temp_path, extension));
            }
        }

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
        // Telegram orders PhotoSize from smallest to largest; take the last (largest)
        if let Some(photo) = photos.last() {
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

    // --- Upload replied message images to GCS ---
    let replied_message_image_urls = if !replied_message_image_paths.is_empty() {
        match bot_deps
            .ai
            .upload_user_images(replied_message_image_paths)
            .await
        {
            Ok(urls) => urls,
            Err(e) => {
                log::error!("Failed to upload replied message images: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // --- Upload user images to GCS ---
    let user_uploaded_image_urls = match bot_deps
        .ai
        .upload_user_images(user_uploaded_image_paths)
        .await
    {
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

    // --- Combine all image URLs ---
    let mut all_image_urls = user_uploaded_image_urls;
    all_image_urls.extend(replied_message_image_urls);

    // Prepare the final prompt with context if available
    let final_prompt = if let Some(context) = replied_message_context {
        format!("{}\n\nUser asks: {}", context, prompt)
    } else {
        prompt
    };

    // Asynchronously generate the response
    let response_result = bot_deps
        .ai
        .generate_response(
            bot.clone(),
            msg.clone(),
            &final_prompt,
            image_url_from_reply,
            all_image_urls,
            chat_model,
            8192,
            temperature,
            None,
            bot_deps.clone(),
            group_id.clone(),
        )
        .await;

    typing_indicator_handle.abort();

    match response_result {
        Ok(ai_response) => {
            let (web_search, file_search, image_gen, _) = ai_response.get_tool_usage_counts();

            let jwt = if group_id.is_some() {
                let group_credentials = group_credentials;

                if group_credentials.is_some() {
                    group_credentials.unwrap().jwt
                } else {
                    credentials.jwt
                }
            } else {
                credentials.jwt
            };

            if profile != "dev" {
                let response = create_purchase_request(
                    file_search,
                    web_search,
                    image_gen,
                    bot_deps.service.clone(),
                    ai_response.total_tokens,
                    ai_response.model,
                    &jwt,
                    group_id,
                )
                .await;

                if response.is_err() {
                    log::error!(
                        "Error purchasing tokens: {}",
                        response.as_ref().err().unwrap()
                    );

                    if response.as_ref().err().unwrap().to_string().contains("401")
                        || response.as_ref().err().unwrap().to_string().contains("403")
                    {
                        bot.send_message(
                            msg.chat.id,
                            "Your login has expired. Please login again.",
                        )
                        .await?;
                    } else {
                        bot.send_message(
                            msg.chat.id,
                            "Sorry, I encountered an error while processing your chat request.",
                        )
                        .await?;
                    }

                    return Ok(());
                }
            }

            if let Some(image_data) = ai_response.image_data {
                let photo = InputFile::memory(image_data);
                let caption = if ai_response.text.len() > 1024 {
                    &ai_response.text[..1024]
                } else {
                    &ai_response.text
                };
                bot.send_photo(msg.chat.id, photo).caption(caption).await?;
                // If the text is longer than 1024, send the rest as a follow-up message
                if ai_response.text.len() > 1024 {
                    send_long_message(&bot, msg.chat.id, &ai_response.text[1024..]).await?;
                }
            } else if let Some(ref tool_calls) = ai_response.tool_calls {
                if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "withdraw_funds")
                {
                    withdraw_funds_hook(bot, msg, ai_response.text).await?;
                } else if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "fund_account")
                {
                    fund_account_hook(bot, msg, ai_response.text).await?;
                } else if tool_calls
                    .iter()
                    .any(|tool_call| tool_call.name == "get_pay_users")
                {
                    // Get transaction_id from the pending transaction
                    let user_id = if let Some(user) = &msg.from {
                        user.id.0 as i64
                    } else {
                        log::warn!("Unable to get user ID for pay_users_hook");
                        send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
                        return Ok(());
                    };
                    
                    let group_id_i64 = group_id_for_hook.as_ref().and_then(|gid| gid.parse::<i64>().ok());
                    
                    if let Some(pending_transaction) = bot_deps.pending_transactions.get_pending_transaction(user_id, group_id_i64) {
                        pay_users_hook(bot, msg, ai_response.text, group_id_for_hook, pending_transaction.transaction_id, bot_deps.clone()).await?;
                    } else {
                        log::warn!("No pending transaction found for user {} in group {:?}", user_id, group_id_i64);
                        send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
                    }
                } else {
                    send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
                }
            } else {
                send_long_message(&bot, msg.chat.id, &ai_response.text).await?;
            }

            // Log tool calls if any
            if let Some(tool_calls) = &ai_response.tool_calls {
                if !tool_calls.is_empty() {
                    log::info!("Tool calls executed: {:?}", tool_calls);
                }
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

pub async fn handle_new_chat(bot: Bot, msg: Message, bot_deps: BotDependencies) -> AnyResult<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;

    match bot_deps.user_convos.clear_response_id(user_id) {
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

pub async fn handle_web_app_data(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
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

    bot_deps
        .auth
        .generate_new_jwt(
            username.clone(),
            user_id,
            payload.account_address,
            payload.resource_account_address,
        )
        .await;

    // Initialize default model preferences for new user
    let _ = initialize_user_preferences(&username, &bot_deps.user_model_prefs).await;

    return Ok(());
}

pub async fn handle_message(bot: Bot, msg: Message, bot_deps: BotDependencies) -> AnyResult<()> {
    // Sentinel: moderate every message in group if sentinel is on
    let mut group_id: Option<String> = None;
    if !msg.chat.is_private() {
        group_id = Some(msg.chat.id.to_string());
        let profile = std::env::var("PROFILE").unwrap_or("prod".to_string());
        let account_seed = std::env::var("ACCOUNT_SEED")
            .map_err(|e| anyhow::anyhow!("ACCOUNT_SEED is not set: {}", e))?;
        let sentinel_tree = bot_deps.db.open_tree("sentinel_state").unwrap();
        let chat_id = msg.chat.id.0.to_be_bytes();
        let user = msg.from.clone();

        let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

        if group_credentials.is_none() {
            log::error!("Group credentials not found");

            bot.send_message(msg.chat.id, "❌ Group not found, please login again")
                .await?;
            return Ok(());
        }

        let group_credentials = group_credentials.unwrap();

        if user.is_some() {
            let user = user.unwrap();

            let username = user.username;

            if username.is_some() {
                let username = username.unwrap();

                if !group_credentials.users.contains(&username) {
                    bot_deps
                        .group
                        .add_user_to_group(msg.chat.id, username)
                        .await?;
                }
            }
        }

        // Check for pending DAO token input
        if let Some(user) = &msg.from {
            let user_id = user.id.0.to_string();
            let current_group_id = msg.chat.id.to_string();
            let dao_token_input_tree = bot_deps.db.open_tree("dao_token_input_pending").unwrap();
            let key = format!("{}_{}", user_id, current_group_id);

            if let Ok(Some(_)) = dao_token_input_tree.get(key.as_bytes()) {
                // User is in token input mode
                if let Some(text) = msg.text() {
                    let text = text.trim();
                    if !text.is_empty() {
                        // Process the token: convert to uppercase except for emojis
                        let processed_token = if text.chars().any(|c| c.is_ascii_alphabetic()) {
                            // Contains letters, convert to uppercase
                            text.to_uppercase()
                        } else {
                            // Likely an emoji or special characters, keep as-is
                            text.to_string()
                        };

                        // Update DAO token preference
                        if let Ok(mut prefs) = bot_deps
                            .dao
                            .get_dao_admin_preferences(current_group_id.clone())
                        {
                            prefs.default_dao_token = Some(processed_token.clone());
                            if let Ok(_) = bot_deps
                                .dao
                                .set_dao_admin_preferences(current_group_id.clone(), prefs)
                            {
                                // Clear the pending state
                                dao_token_input_tree.remove(key.as_bytes()).unwrap();

                                bot.send_message(
                                    msg.chat.id,
                                    format!("✅ <b>DAO token updated to {}</b>", processed_token),
                                )
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;
                                return Ok(());
                            }
                        }

                        // If we get here, there was an error
                        dao_token_input_tree.remove(key.as_bytes()).unwrap();
                        bot.send_message(msg.chat.id, "❌ Error updating DAO token preference")
                            .await?;
                        return Ok(());
                    }
                }

                // Invalid input, ask again
                bot.send_message(
                    msg.chat.id,
                    "❌ Please send a valid token ticker or emojicoin. Example: APT, USDC, or 📒",
                )
                .await?;
                return Ok(());
            }
        }

        // Check if sentinel is on for this group
        let sentinel_on = sentinel_tree
            .get(chat_id)
            .unwrap()
            .map(|v| v == b"on")
            .unwrap_or(false);
        if sentinel_on {
            // Don't moderate admin or bot messages
            if let Some(user) = &msg.from {
                if user.is_bot {
                    return Ok(());
                }
                // Check admin status
                let admins = bot.get_chat_administrators(msg.chat.id).await?;
                let is_admin = admins.iter().any(|member| member.user.id == user.id);
                if is_admin {
                    return Ok(());
                }
            } else {
                return Ok(());
            }

            let address = group_credentials.resource_account_address;

            let coin_address = bot_deps.panora.aptos.get_token_address().await;

            if coin_address.is_err() {
                bot.send_message(
                    msg.chat.id,
                    "❌ Coin address not found, please contact support",
                )
                .await?;
                return Ok(());
            }

            let coin_address = coin_address.unwrap();

            let group_balance = bot_deps
                .panora
                .aptos
                .get_account_balance(&address, &coin_address)
                .await?;

            let token = bot_deps.panora.get_token_ai_fees().await?;

            let token_price = token.usd_price;

            if token_price.is_none() {
                bot.send_message(
                    msg.chat.id,
                    "❌ Token price not found, please contact support",
                )
                .await?;
                return Ok(());
            }

            let token_price = token_price.unwrap();

            let token_price = token_price.parse::<f64>();

            if token_price.is_err() {
                bot.send_message(
                    msg.chat.id,
                    "❌ Token price not found, please contact support",
                )
                .await?;
                return Ok(());
            }

            let token_price = token_price.unwrap();

            let token_decimals = token.decimals;

            if token_decimals.is_none() {
                return Err(anyhow::anyhow!("Token decimals not found"));
            }

            let token_decimals = token_decimals.unwrap();

            let min_deposit = (bot_deps.panora.min_deposit / 10_f64) / token_price;

            let min_deposit = (min_deposit as f64 * 10_f64.powi(token_decimals as i32)) as u64;

            if group_balance < min_deposit as i64 {
                let min_deposit_formatted = format!(
                    "{:.2}",
                    min_deposit as f64 / 10_f64.powi(token_decimals as i32)
                );

                let group_balance_formatted = format!(
                    "{:.2}",
                    group_balance as f64 / 10_f64.powi(token_decimals as i32)
                );

                bot.send_message(
                    msg.chat.id,
                    format!(
                        "User balance is less than the minimum deposit. Please fund your account transfering {} to {} address. Minimum deposit: {} {} (Your balance: {} {})",
                        token.symbol.clone().unwrap_or("".to_string()),
                        address,
                        min_deposit_formatted,
                        token.symbol.clone().unwrap_or("".to_string()),
                        group_balance_formatted,
                        token.symbol.unwrap_or("".to_string())
                    )
                )
                .await?;
                return Ok(());
            }

            // Use the same moderation logic as /mod
            let moderation_service =
                ModerationService::new(std::env::var("OPENAI_API_KEY").unwrap()).unwrap();
            let message_text = msg.text().or_else(|| msg.caption()).unwrap_or("");
            match moderation_service
                .moderate_message(message_text, &bot, &msg, &msg)
                .await
            {
                Ok(result) => {
                    log::info!(
                        "Sentinel moderation result: {} for message: {} (tokens: {})",
                        result.verdict,
                        message_text,
                        result.total_tokens
                    );

                    if profile != "dev" {
                        let purchase_result = bot_deps
                            .service
                            .group_purchase(
                                group_credentials.jwt,
                                PurchaseRequest {
                                    model: Model::GPT41Nano,
                                    tokens_used: result.total_tokens,
                                    tools_used: vec![],
                                    group_id: Some(format!(
                                        "{}-{}",
                                        msg.chat.id.0.to_string(),
                                        account_seed
                                    )),
                                },
                            )
                            .await;

                        if purchase_result.is_err() {
                            log::error!(
                                "Failed to purchase ai for flagged content: {}",
                                purchase_result.err().unwrap()
                            );
                            return Ok(());
                        }
                    }

                    if result.verdict == "F" {
                        // Mute the user
                        if let Some(flagged_user) = &msg.from {
                            let restricted_permissions = teloxide::types::ChatPermissions::empty();

                            // Check if the user is already muted
                            if let Err(mute_error) = bot
                                .restrict_chat_member(
                                    msg.chat.id,
                                    flagged_user.id,
                                    restricted_permissions,
                                )
                                .await
                            {
                                log::error!(
                                    "Failed to mute user {}: {}",
                                    flagged_user.id,
                                    mute_error
                                );
                            } else {
                                log::info!(
                                    "Successfully muted user {} for flagged content (sentinel)",
                                    flagged_user.id
                                );
                            }
                            // Add admin buttons
                            let keyboard = InlineKeyboardMarkup::new(vec![vec![
                                InlineKeyboardButton::callback(
                                    "🔇 Unmute",
                                    format!("unmute:{}", flagged_user.id),
                                ),
                                InlineKeyboardButton::callback(
                                    "🚫 Ban",
                                    format!("ban:{}", flagged_user.id),
                                ),
                            ]]);
                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "🛡️ <b>Content Flagged & User Muted</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n🔇 User has been muted\n\n💬 <i>Flagged message:</i>\n<blockquote>{}</blockquote>",
                                    msg.id,
                                    teloxide::utils::html::escape(message_text)
                                )
                            )
                            .parse_mode(ParseMode::Html)
                            .reply_markup(keyboard)
                            .await?;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Sentinel moderation failed: {}", e);
                }
            }
            return Ok(());
        }
    }

    if msg.media_group_id().is_some() && msg.photo().is_some() {
        let media_aggregator = bot_deps.media_aggregator.clone();
        media_aggregator.add_message(msg, bot_deps.clone()).await;
        return Ok(());
    }

    // Photo-only message (no text/caption) may belong to a pending command
    if msg.text().is_none() && msg.caption().is_none() && msg.photo().is_some() {
        let cmd_collector = bot_deps.cmd_collector.clone();
        cmd_collector
            .try_attach_photo(msg, bot_deps.clone(), group_id)
            .await;
        return Ok(());
    }

    if msg.caption().is_none()
        && msg.chat.is_private()
        && (msg.document().is_some()
            || msg.photo().is_some()
            || msg.video().is_some()
            || msg.audio().is_some())
    {
        handle_file_upload(bot, msg, bot_deps.clone()).await?;
    }
    Ok(())
}

pub async fn handle_sentinel(
    bot: Bot,
    msg: Message,
    param: String,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    // Only admins can use /sentinel
    if !msg.chat.is_private() {
        let admins = bot.get_chat_administrators(msg.chat.id).await?;
        let requester_id = msg.from.as_ref().map(|u| u.id);
        let is_admin = requester_id
            .map(|uid| admins.iter().any(|member| member.user.id == uid))
            .unwrap_or(false);
        if !is_admin {
            bot.send_message(
                msg.chat.id,
                "❌ <b>Permission Denied</b>\n\nOnly group administrators can use /sentinel.",
            )
            .parse_mode(ParseMode::Html)
            .await?;
            return Ok(());
        }
    }
    let param = param.trim().to_lowercase();
    let sentinel_tree = bot_deps.db.open_tree("sentinel_state").unwrap();
    let chat_id = msg.chat.id.0.to_be_bytes();
    match param.as_str() {
        "on" => {
            sentinel_tree.insert(chat_id, b"on").unwrap();
            bot.send_message(
                msg.chat.id,
                "🛡️ <b>Sentinel System</b>\n\n✅ <b>Sentinel is now ON</b>\n\nAll messages will be automatically moderated. /mod command is disabled."
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }
        "off" => {
            sentinel_tree.insert(chat_id, b"off").unwrap();
            bot.send_message(
                msg.chat.id,
                "🛡️ <b>Sentinel System</b>\n\n⏹️ <b>Sentinel is now OFF</b>\n\nManual moderation via /mod is re-enabled."
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }
        _ => {
            bot.send_message(
                msg.chat.id,
                "❌ <b>Invalid Parameter</b>\n\n📝 Usage: <code>/sentinel on</code> or <code>/sentinel off</code>\n\n💡 Please specify either 'on' or 'off' to control the sentinel system."
            )
            .parse_mode(ParseMode::Html)
            .await?;
        }
    }
    Ok(())
}

pub async fn handle_wallet_address(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    println!("handle_wallet_address");
    let user = msg.from;

    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        bot.send_message(msg.chat.id, "❌ Username not found")
            .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let user_credentials = bot_deps.auth.get_credentials(&username);

    if user_credentials.is_none() {
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let user_credentials = user_credentials.unwrap();

    let wallet_address = user_credentials.resource_account_address;

    bot.send_message(
        msg.chat.id,
        format!(
            "💰 <b>Your Wallet Address</b>\n\n<code>{}</code>",
            wallet_address
        ),
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(())
}

pub async fn handle_mod(bot: Bot, msg: Message, bot_deps: BotDependencies) -> AnyResult<()> {
    // Check if sentinel is on for this chat
    if !msg.chat.is_private() {
        let sentinel_tree = bot_deps.db.open_tree("sentinel_state").unwrap();
        let chat_id = msg.chat.id.0.to_be_bytes();
        let sentinel_on = sentinel_tree
            .get(chat_id)
            .unwrap()
            .map(|v| v == b"on")
            .unwrap_or(false);
        if sentinel_on {
            bot.send_message(
                msg.chat.id,
                "🛡️ <b>Sentinel Mode Active</b>\n\n/mod is disabled while sentinel is ON. All messages are being automatically moderated."
            )
            .parse_mode(ParseMode::Html)
            .await?;
            return Ok(());
        }
    }

    let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

    if group_credentials.is_none() {
        bot.send_message(msg.chat.id, "❌ Group not found").await?;
        return Ok(());
    }

    // Check if the command is used in reply to a message
    if let Some(reply_to_msg) = msg.reply_to_message() {
        // Extract text from the replied message
        let message_text = reply_to_msg
            .text()
            .or_else(|| reply_to_msg.caption())
            .unwrap_or_default();

        if message_text.is_empty() {
            bot.send_message(
                msg.chat.id,
                format!("⚠️ <b>No Text Found</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ The replied message contains no text to moderate.", reply_to_msg.id)
            )
            .parse_mode(ParseMode::Html)
            .await?;
            return Ok(());
        }

        // Create moderation service using environment API key
        let openai_api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not found in environment"))?;

        let moderation_service = ModerationService::new(openai_api_key)
            .map_err(|e| anyhow::anyhow!("Failed to create moderation service: {}", e))?;

        // Moderate the message
        match moderation_service
            .moderate_message(message_text, &bot, &msg, &reply_to_msg)
            .await
        {
            Ok(result) => {
                log::info!(
                    "Manual moderation result: {} for message: {} (tokens: {})",
                    result.verdict,
                    message_text,
                    result.total_tokens
                );

                let purchase_result = bot_deps
                    .service
                    .group_purchase(
                        group_credentials.unwrap().jwt,
                        PurchaseRequest {
                            model: Model::GPT41Nano,
                            tokens_used: result.total_tokens,
                            tools_used: vec![],
                            group_id: Some(msg.chat.id.0.to_string()),
                        },
                    )
                    .await;

                if purchase_result.is_err() {
                    log::error!(
                        "Failed to purchase ai for flagged content: {}",
                        purchase_result.err().unwrap()
                    );
                    return Ok(());
                }

                // Only respond if the message is flagged
                if result.verdict == "F" {
                    // First, mute the user who sent the flagged message
                    if let Some(flagged_user) = &reply_to_msg.from {
                        // Create restricted permissions (muted)
                        let restricted_permissions = teloxide::types::ChatPermissions::empty();

                        // Mute the user indefinitely
                        if let Err(mute_error) = bot
                            .restrict_chat_member(
                                msg.chat.id,
                                flagged_user.id,
                                restricted_permissions,
                            )
                            .await
                        {
                            log::error!("Failed to mute user {}: {}", flagged_user.id, mute_error);
                        } else {
                            log::info!(
                                "Successfully muted user {} for flagged content",
                                flagged_user.id
                            );
                        }

                        // Create keyboard with admin controls
                        let keyboard = InlineKeyboardMarkup::new(vec![vec![
                            InlineKeyboardButton::callback(
                                "🔇 Unmute",
                                format!("unmute:{}", flagged_user.id),
                            ),
                            InlineKeyboardButton::callback(
                                "🚫 Ban",
                                format!("ban:{}", flagged_user.id),
                            ),
                        ]]);

                        // Send the flagged message response
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🛡️ <b>Content Flagged & User Muted</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n🔇 User has been muted\n\n💬 <i>Flagged message:</i>\n<blockquote>{}</blockquote>",
                                reply_to_msg.id,
                                teloxide::utils::html::escape(message_text)
                            )
                        )
                        .parse_mode(ParseMode::Html)
                        .reply_markup(keyboard)
                        .await?;
                    } else {
                        // Fallback if no user found in the replied message
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🛡️ <b>Content Flagged</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n⚠️ Could not identify user to mute\n\n💬 <i>Flagged message:</i>\n<blockquote>{}</blockquote>",
                                reply_to_msg.id,
                                teloxide::utils::html::escape(message_text)
                            )
                        )
                        .parse_mode(ParseMode::Html)
                        .await?;
                    }
                }
                // Silent when passed (P) - no response
            }
            Err(e) => {
                log::error!("Moderation failed: {}", e);
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "🛡️ <b>Moderation Error</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ <b>Error:</b> Failed to analyze message. Please try again later.\n\n🔧 <i>Technical details:</i> {}",
                        reply_to_msg.id,
                        e
                    )
                )
                .parse_mode(ParseMode::Html)
                .await?;
            }
        }
    } else {
        // Not a reply to a message, show usage instructions
        bot.send_message(
            msg.chat.id,
            "❌ <b>Invalid Usage</b>\n\n📝 The <code>/mod</code> command must be used in reply to a message.\n\n💡 <b>How to use:</b>\n1. Find the message you want to moderate\n2. Reply to that message with <code>/mod</code>\n\n🛡️ This will analyze the content of the replied message for violations."
        )
        .parse_mode(ParseMode::Html)
        .await?;
    }
    Ok(())
}

pub async fn handle_balance(
    bot: Bot,
    msg: Message,
    symbol: &str,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    let user = msg.from;

    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        log::error!("❌ Username not found");
        bot.send_message(msg.chat.id, "❌ Username not found")
            .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let user_credentials = bot_deps.auth.get_credentials(&username);

    if user_credentials.is_none() {
        log::error!("❌ User not found");
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let (token_type, decimals, token_symbol) =
        if symbol.to_lowercase() == "apt" || symbol.to_lowercase() == "aptos" {
            (
                "0x1::aptos_coin::AptosCoin".to_string(),
                8u8,
                "APT".to_string(),
            )
        } else {
            let token = bot_deps.panora.get_token_by_symbol(symbol).await;

            if token.is_err() {
                log::error!("❌ Error getting token: {}", token.as_ref().err().unwrap());
                bot.send_message(msg.chat.id, "❌ Error getting token")
                    .await?;
                return Ok(());
            }

            let token = token.unwrap();

            let token_type = if token.token_address.as_ref().is_some() {
                token.token_address.as_ref().unwrap().to_string()
            } else {
                token.fa_address.clone()
            };

            (token_type, token.decimals, token.symbol.clone())
        };

    let user_credentials = user_credentials.unwrap();

    let balance = bot_deps
        .panora
        .aptos
        .node
        .get_account_balance(
            user_credentials.resource_account_address,
            token_type.to_string(),
        )
        .await;

    if balance.is_err() {
        log::error!(
            "❌ Error getting balance: {}",
            balance.as_ref().err().unwrap()
        );
        bot.send_message(msg.chat.id, "❌ Error getting balance")
            .await?;
        return Ok(());
    }

    let raw_balance = balance.unwrap().into_inner();

    let balance_i64 = raw_balance.as_i64();

    if balance_i64.is_none() {
        log::error!("❌ Balance not found");
        bot.send_message(msg.chat.id, "❌ Balance not found")
            .await?;
        return Ok(());
    }

    let raw_balance = balance_i64.unwrap();

    // Convert raw balance to human readable format using decimals
    let human_balance = raw_balance as f64 / 10_f64.powi(decimals as i32);

    println!(
        "Raw balance: {}, Human balance: {}",
        raw_balance, human_balance
    );

    bot.send_message(
        msg.chat.id,
        format!("💰 **Balance**: {:.6} {}", human_balance, token_symbol),
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(())
}

pub async fn handle_group_balance(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
    symbol: &str,
) -> AnyResult<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(msg.chat.id, "❌ This command can only be used in a group")
            .await?;
        return Ok(());
    }

    let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

    if group_credentials.is_none() {
        bot.send_message(msg.chat.id, "❌ Group not found").await?;
        return Ok(());
    }

    let group_credentials = group_credentials.unwrap();

    let (token_type, decimals, token_symbol) =
        if symbol.to_lowercase() == "apt" || symbol.to_lowercase() == "aptos" {
            (
                "0x1::aptos_coin::AptosCoin".to_string(),
                8u8,
                "APT".to_string(),
            )
        } else {
            let tokens = bot_deps.panora.get_token_by_symbol(symbol).await;

            if tokens.is_err() {
                log::error!("❌ Error getting token: {}", tokens.as_ref().err().unwrap());
                bot.send_message(msg.chat.id, "❌ Error getting token")
                    .await?;
                return Ok(());
            }

            let token = tokens.unwrap();

            let token_type = if token.token_address.as_ref().is_some() {
                token.token_address.as_ref().unwrap().to_string()
            } else {
                token.fa_address.clone()
            };

            (token_type, token.decimals, token.symbol.clone())
        };

    let balance = bot_deps
        .panora
        .aptos
        .node
        .get_account_balance(
            group_credentials.resource_account_address,
            token_type.to_string(),
        )
        .await;

    if balance.is_err() {
        log::error!(
            "❌ Error getting balance: {}",
            balance.as_ref().err().unwrap()
        );
        bot.send_message(msg.chat.id, "❌ Error getting balance")
            .await?;
        return Ok(());
    }

    let raw_balance = balance.unwrap().into_inner();

    let balance_i64 = raw_balance.as_i64();

    if balance_i64.is_none() {
        log::error!("❌ Balance not found");
        bot.send_message(msg.chat.id, "❌ Balance not found")
            .await?;
        return Ok(());
    }

    let raw_balance = balance_i64.unwrap();

    // Convert raw balance to human readable format using decimals
    let human_balance = raw_balance as f64 / 10_f64.powi(decimals as i32);

    bot.send_message(
        msg.chat.id,
        format!("💰 **Balance**: {:.6} {}", human_balance, token_symbol),
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(())
}

pub async fn handle_group_wallet_address(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(msg.chat.id, "❌ This command can only be used in a group")
            .await?;
        return Ok(());
    }

    let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

    log::info!("Group id: {:?}", msg.chat.id);

    if group_credentials.is_none() {
        bot.send_message(msg.chat.id, "❌ Group not found").await?;
        return Ok(());
    }

    let group_credentials = group_credentials.unwrap();

    bot.send_message(
        msg.chat.id,
        format!(
            "💰 <b>Group Wallet Address</b>\n\n<code>{}</code>",
            group_credentials.resource_account_address
        ),
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(())
}

pub async fn handle_moderation_rules(bot: Bot, msg: Message) -> AnyResult<()> {
    let rules = r#"
<b>🛡️ Moderation Rules</b>

To avoid being muted or banned, please follow these rules:

<b>1. No Promotion or Selling</b>
- Do not offer services, products, access, or benefits
- Do not position yourself as an authority/leader to gain trust
- Do not promise exclusive opportunities or deals
- No commercial solicitation of any kind

<b>2. No Private Communication Invites</b>
- Do not request to move conversation to DM/private
- Do not offer to send details privately
- Do not ask for personal contact information
- Do not attempt to bypass public group discussion

<b>Examples (not exhaustive):</b>
- "I can offer you whitelist access"
- "DM me for details"
- "React and I'll message you"
- "I'm a [title] and can help you"
- "Send me your wallet address"
- "Contact me privately"
- "I'll send you the link"

If you have questions, ask an admin before posting.
"#;
    bot.send_message(msg.chat.id, rules)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn handle_migrate_group_id(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    let group_admins = bot.get_chat_administrators(msg.chat.id).await?;

    let user = msg.from;

    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let user = user.unwrap();

    let is_admin = group_admins.iter().any(|admin| admin.user.id == user.id);

    if !is_admin {
        bot.send_message(msg.chat.id, "❌ Only admins can migrate group id")
            .await?;
        return Ok(());
    }

    let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

    if group_credentials.is_none() {
        bot.send_message(msg.chat.id, "❌ Group not found").await?;
        return Ok(());
    }

    let group_credentials = group_credentials.unwrap();

    let transaction_response = bot_deps
        .service
        .migrate_group_id(group_credentials.jwt)
        .await;

    if transaction_response.is_err() {
        bot.send_message(msg.chat.id, "❌ Error migrating group id")
            .await?;
        return Ok(());
    }

    let transaction_response = transaction_response.unwrap();

    bot.send_message(
        msg.chat.id,
        format!("✅ Group id migrated: {}", transaction_response.hash),
    )
    .await?;

    Ok(())
}

async fn check_group_resource_account_address(
    bot: &Bot,
    group_credentials: GroupCredentials,
    msg: Message,
    bot_deps: &BotDependencies,
) -> AnyResult<GroupCredentials> {
    let group_id = group_credentials.group_id.clone();

    if group_credentials.resource_account_address.is_empty() {
        const MAX_RETRIES: u32 = 5;
        const RETRY_DELAY_MS: u64 = 2000;

        for attempt in 1..=MAX_RETRIES {
            let resource_account_address = bot_deps
                .panora
                .aptos
                .node
                .view_function(ViewRequest {
                    function: format!(
                        "{}::group::get_group_account",
                        bot_deps.panora.aptos.contract_address
                    ),
                    type_arguments: vec![],
                    arguments: vec![value::Value::String(group_id.clone())],
                })
                .await;

            if resource_account_address.is_ok() {
                let resource_account_address = resource_account_address.unwrap().into_inner();

                let resource_account_address =
                    serde_json::from_value::<Vec<String>>(resource_account_address);

                if resource_account_address.is_ok() {
                    let resource_account_address = resource_account_address.unwrap();

                    let new_credentials = GroupCredentials {
                        jwt: group_credentials.jwt.clone(),
                        group_id: group_credentials.group_id.clone(),
                        resource_account_address: resource_account_address[0].clone(),
                        users: group_credentials.users.clone(),
                    };

                    bot_deps
                        .group
                        .save_credentials(new_credentials)
                        .map_err(|_| anyhow::anyhow!("Error saving group credentials"))?;

                    let updated_credentials = GroupCredentials {
                        jwt: group_credentials.jwt,
                        group_id: group_credentials.group_id,
                        resource_account_address: resource_account_address[0].clone(),
                        users: group_credentials.users,
                    };

                    return Ok(updated_credentials);
                }
            }

            // If this is not the last attempt, wait before retrying
            if attempt < MAX_RETRIES {
                log::warn!(
                    "Failed to get resource account address (attempt {}/{}), retrying in {}ms...",
                    attempt,
                    MAX_RETRIES,
                    RETRY_DELAY_MS
                );
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }

        // All retries failed
        bot.send_message(
            msg.chat.id,
            "❌ Error getting resource account address after multiple attempts",
        )
        .await?;
        return Err(anyhow::anyhow!(
            "Error getting resource account address after {} attempts",
            MAX_RETRIES
        ));
    }

    Ok(group_credentials)
}
