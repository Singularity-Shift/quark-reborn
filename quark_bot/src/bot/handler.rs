//! Command handlers for quark_bot Telegram bot.
use crate::{
    ai::moderation::dto::{ModerationSettings, ModerationState}, assets::handler::handle_file_upload, bot::hooks::{fund_account_hook, pay_users_hook, withdraw_funds_hook}, credentials::dto::CredentialsPayload, dependencies::BotDependencies, group::dto::GroupCredentials, payment::dto::PaymentPrefs, sponsor::dto::SponsorInterval, utils::{self, create_purchase_request}
};
use anyhow::Result as AnyResult;
use aptos_rust_sdk_types::api_types::view::ViewRequest;
use serde_json::value;

use crate::{
    ai::{
        moderation::ModerationOverrides,
        vector_store::list_user_files_with_names,
    },
    user_model_preferences::handler::initialize_user_preferences,
};

use crate::scheduled_prompts::dto::PendingStep;
use crate::scheduled_prompts::helpers::build_hours_keyboard;
use chrono;
use open_ai_rust_responses_by_sshift::Model;
use quark_core::helpers::{bot_commands::Command, dto::CreateGroupRequest};
use regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
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

/// Split a Telegram-HTML message into chunks without cutting inside tags/entities.
fn split_message(text: &str) -> Vec<String> {
    if text.len() <= TELEGRAM_MESSAGE_LIMIT {
        return vec![text.to_string()];
    }

    // Track whether a tag requires closing
    fn is_closing_required(tag: &str) -> bool {
        matches!(
            tag,
            "b" | "strong"
                | "i"
                | "em"
                | "u"
                | "ins"
                | "s"
                | "strike"
                | "del"
                | "code"
                | "pre"
                | "a"
                | "tg-spoiler"
                | "span"
                | "blockquote"
        )
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut last_safe_break: Option<usize> = None; // index in buf safe to split
    let mut inside_tag = false;
    let mut inside_entity = false;
    let mut tag_buf = String::new();
    let mut open_stack: Vec<String> = Vec::new();
    let mut tag_start_in_buf: usize = 0; // start index of current tag
    let mut last_anchor_start: Option<usize> = None; // avoid splitting inside <a>

    let push_chunk = |buf: &mut String, chunks: &mut Vec<String>| {
        if !buf.trim().is_empty() {
            chunks.push(buf.trim().to_string());
        }
        buf.clear();
    };

    for ch in text.chars() {
        match ch {
            '<' => {
                inside_tag = true;
                tag_buf.clear();
                tag_start_in_buf = buf.len();
                buf.push(ch);
            }
            '>' => {
                buf.push(ch);
                if inside_tag {
                    // parse tag name
                    let tag_content = tag_buf.trim();
                    let is_end = tag_content.starts_with('/')
                        || tag_content.starts_with("/ ")
                        || tag_content.starts_with(" /");
                    let name = tag_content
                        .trim_start_matches('/')
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_lowercase();
                    if !name.is_empty() && is_closing_required(&name) {
                        if is_end {
                            if let Some(pos) = open_stack.iter().rposition(|t| t == &name) {
                                open_stack.remove(pos);
                            }
                            if name == "a" {
                                last_anchor_start = None;
                            }
                        } else {
                            open_stack.push(name.clone());
                            if name == "a" {
                                last_anchor_start = Some(tag_start_in_buf);
                            }
                        }
                    }
                }
                inside_tag = false;
                if !inside_entity && open_stack.is_empty() {
                    last_safe_break = Some(buf.len());
                }
            }
            '&' => {
                inside_entity = true;
                buf.push(ch);
            }
            ';' => {
                buf.push(ch);
                if inside_entity {
                    inside_entity = false;
                    if !inside_tag && open_stack.is_empty() {
                        last_safe_break = Some(buf.len());
                    }
                }
            }
            _ => {
                if inside_tag {
                    tag_buf.push(ch);
                }
                buf.push(ch);
                if (ch == ' ' || ch == '\n' || ch == '\t')
                    && !inside_tag
                    && !inside_entity
                    && open_stack.is_empty()
                {
                    last_safe_break = Some(buf.len());
                }
            }
        }

        if buf.len() >= TELEGRAM_MESSAGE_LIMIT {
            if let Some(idx) = last_safe_break {
                let remainder = buf.split_off(idx);
                let chunk = buf.trim().to_string();
                if !chunk.is_empty() {
                    chunks.push(chunk);
                }
                buf = remainder;
            } else if last_anchor_start.is_some() {
                // Split before the anchor started to avoid cutting inside <a>
                let pos = last_anchor_start.unwrap();
                if pos > 0 {
                    let remainder = buf.split_off(pos);
                    let chunk = buf.trim().to_string();
                    if !chunk.is_empty() {
                        chunks.push(chunk);
                    }
                    buf = remainder;
                } else {
                    // Anchor starts at 0; fall back to pushing the whole buffer to make progress
                    push_chunk(&mut buf, &mut chunks);
                }
            } else if open_stack.iter().any(|t| t == "pre" || t == "code") {
                // Close pre/code at boundary and reopen in next chunk
                let closable: Vec<&str> = open_stack
                    .iter()
                    .map(|s| s.as_str())
                    .filter(|t| *t == "pre" || *t == "code")
                    .collect();
                for t in closable.iter().rev() {
                    buf.push_str(&format!("</{}>", t));
                }
                let reopen = closable
                    .iter()
                    .map(|t| format!("<{}>", t))
                    .collect::<Vec<_>>()
                    .join("");
                let chunk = buf.trim().to_string();
                if !chunk.is_empty() {
                    chunks.push(chunk);
                }
                buf.clear();
                buf.push_str(&reopen);
            } else {
                // Last resort: push whatever we have (should be rare)
                push_chunk(&mut buf, &mut chunks);
            }
            last_safe_break = None;
        }
    }

    if !buf.trim().is_empty() {
        chunks.push(buf.trim().to_string());
    }

    chunks
}

/// Extract all <pre>...</pre> blocks and return the text without them, plus the list of pre contents
fn split_off_pre_blocks(text: &str) -> (String, Vec<String>) {
    let re = regex::Regex::new(r"(?s)<pre[^>]*>(.*?)</pre>").unwrap();
    let mut pre_blocks: Vec<String> = Vec::new();
    let without_pre = re
        .replace_all(text, |caps: &regex::Captures| {
            pre_blocks.push(caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string());
            "".to_string()
        })
        .to_string();
    (without_pre, pre_blocks)
}

/// Send a long <pre> block safely by chunking and wrapping each chunk in <pre> tags
async fn send_pre_block(bot: &Bot, chat_id: ChatId, title: &str, content: &str) -> AnyResult<()> {
    // Escape HTML special chars inside the <pre> block
    let escaped = teloxide::utils::html::escape(content);
    let prefix = format!("{}\n<pre>", title);
    let suffix = "</pre>";
    // Leave some headroom for prefix/suffix
    let max_payload = TELEGRAM_MESSAGE_LIMIT.saturating_sub(prefix.len() + suffix.len() + 16);
    let mut current = String::new();
    for ch in escaped.chars() {
        if current.chars().count() + 1 > max_payload {
            let msg = format!("{}{}{}", prefix, current, suffix);
            match bot
                .send_message(chat_id, msg)
                .parse_mode(ParseMode::Html)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    let err_text = e.to_string();
                    log::error!("Error sending <pre> chunk: {}", err_text);
                    if err_text.contains("can't parse entities")
                        || err_text.contains("Unsupported start tag")
                    {
                        let _ = bot
                            .send_message(
                                chat_id,
                                "Sorry — I made an error in my output. Please try again or start a /newchat.",
                            )
                            .await;
                        return Ok(());
                    }
                    return Err(e.into());
                }
            }
            current.clear();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        let msg = format!("{}{}{}", prefix, current, suffix);
        match bot
            .send_message(chat_id, msg)
            .parse_mode(ParseMode::Html)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                let err_text = e.to_string();
                log::error!("Error sending final <pre> chunk: {}", err_text);
                if err_text.contains("can't parse entities")
                    || err_text.contains("Unsupported start tag")
                {
                    let _ = bot
                        .send_message(
                            chat_id,
                            "Sorry — I made an error in my output. Please try again or start a /newchat.",
                        )
                        .await;
                    return Ok(());
                }
                return Err(e.into());
            }
        }
    }
    Ok(())
}

/// Send a potentially long message, splitting it into multiple messages if necessary
async fn send_long_message(bot: &Bot, chat_id: ChatId, text: &str) -> AnyResult<()> {
    // Convert markdown (including ``` code fences) to Telegram-compatible HTML
    let html_text = utils::markdown_to_html(text);
    // Normalize image anchor to point to the public GCS URL when present
    let html_text = utils::normalize_image_url_anchor(&html_text);
    let chunks = split_message(&html_text);

    for (i, chunk) in chunks.iter().enumerate() {
        if i > 0 {
            // Small delay between messages to avoid rate limiting
            sleep(Duration::from_millis(100)).await;
        }

        match bot
            .send_message(chat_id, chunk)
            .parse_mode(ParseMode::Html)
            .await
        {
            Ok(_) => {}
            Err(e) => {
                let err_text = e.to_string();
                log::error!("Error sending message chunk: {}", err_text);
                if err_text.contains("can't parse entities")
                    || err_text.contains("Unsupported start tag")
                {
                    let _ = bot
                        .send_message(
                            chat_id,
                            "Sorry — I made an error in my output. Please try again or start a /newchat.",
                        )
                        .await;
                    return Ok(());
                }
                return Err(e.into());
            }
        }
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

    let account_seed = bot_deps.group.account_seed.clone();

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

pub async fn handle_chat(
    bot: Bot,
    msg: Message,
    prompt: String,
    group_id: Option<String>,
    is_sponsor: bool,
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

    let user_id = user.unwrap().id.to_string();
    let username = user.unwrap().username.as_ref();

    if username.is_none() {
        typing_indicator_handle.abort();
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let credentials = bot_deps.auth.get_credentials(&username);
    if credentials.is_none() && !is_sponsor {
        typing_indicator_handle.abort();
        bot.send_message(msg.chat.id, "❌ Unable to verify permissions.")
            .await?;
        return Ok(());
    }

    let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

    // Load user's chat model preferences
    let preferences = bot_deps.user_model_prefs.get_preferences(username);

    let chat_model = preferences.chat_model.to_openai_model();
    // Only pass temperature for models that support it
    let temperature = match chat_model {
        Model::GPT41 | Model::GPT41Mini | Model::GPT4o => Some(preferences.temperature),
        _ => None,
    };

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
                    credentials.unwrap().jwt
                }
            } else {
                credentials.unwrap().jwt
            };

            if profile != "dev" {
                let response = create_purchase_request(
                    file_search,
                    web_search,
                    image_gen,
                    ai_response.total_tokens,
                    ai_response.model,
                    &jwt,
                    group_id,
                    user_id,
                    bot_deps.clone(),
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
                // Strip <pre> blocks from caption to avoid unbalanced HTML when truncated
                let (text_without_pre, pre_blocks) = split_off_pre_blocks(&ai_response.text);
                let caption = if text_without_pre.len() > 1024 {
                    &text_without_pre[..1024]
                } else {
                    &text_without_pre
                };
                bot.send_photo(msg.chat.id, photo)
                    .caption(caption)
                    .parse_mode(ParseMode::Html)
                    .await?;
                // Send any extracted <pre> blocks safely in full
                for pre in pre_blocks {
                    send_pre_block(&bot, msg.chat.id, "", &pre).await?;
                }
                // If the text_without_pre is longer than 1024, send the remainder
                if text_without_pre.len() > 1024 {
                    send_long_message(&bot, msg.chat.id, &text_without_pre[1024..]).await?;
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

                    let group_id_i64 = group_id_for_hook
                        .as_ref()
                        .and_then(|gid| gid.parse::<i64>().ok());

                    if let Some(pending_transaction) = bot_deps
                        .pending_transactions
                        .get_pending_transaction(user_id, group_id_i64)
                    {
                        pay_users_hook(
                            bot,
                            msg,
                            ai_response.text,
                            group_id_for_hook,
                            pending_transaction.transaction_id,
                            bot_deps.clone(),
                        )
                        .await?;
                    } else {
                        log::warn!(
                            "No pending transaction found for user {} in group {:?}",
                            user_id,
                            group_id_i64
                        );
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
            bot.send_message(msg.chat.id, format!("An error occurred while processing your request: {}", e))
                .parse_mode(ParseMode::Html)
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
        let chat_id = msg.chat.id;
        let user = msg.from.clone();

        if user.is_none() {
            return Ok(());
        }

        let user_id = user.as_ref().unwrap().id.to_string();

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

            // Try to find the pending token input with the formatted group ID
            let formatted_group_id =
                format!("{}-{}", current_group_id, bot_deps.group.account_seed);
            let key = format!("{}_{}", user_id, formatted_group_id);

            if let Ok(_) = bot_deps.dao.get_pending_tokens(key.clone()) {
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

                        // Update DAO token preference using the formatted group ID
                        if let Ok(mut prefs) = bot_deps
                            .dao
                            .get_dao_admin_preferences(formatted_group_id.clone())
                        {
                            prefs.default_dao_token = Some(processed_token.clone());
                            if let Ok(_) = bot_deps
                                .dao
                                .set_dao_admin_preferences(formatted_group_id.clone(), prefs)
                            {
                                // Clear the pending state
                                bot_deps.dao.remove_pending_tokens(key).unwrap();

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
                        bot_deps.dao.remove_pending_tokens(key).unwrap();
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

        // Welcome custom message input mode: capture replies
        if let Some(user) = &msg.from {
            let current_group_id = msg.chat.id.to_string();
            let key = format!("welcome_custom_msg_input:{}", current_group_id);
            
            log::info!("Checking welcome input state for key: {}", key);
            
            // Check if there's an active welcome custom message input mode for this group
            if let Some(_input_state) = bot_deps.welcome_service.get_input_state(&key) {
                log::info!("Found welcome input state for group: {}", current_group_id);
                // Only process if the user is an admin
                let is_admin = utils::is_admin(&bot, msg.chat.id, user.id).await;
                if !is_admin {
                    // Non-admin users typing during welcome setup - ignore silently
                    return Ok(());
                }

                if let Some(text) = msg.text() {
                    let text = text.trim();
                    if !text.is_empty() {
                        if text == "/cancel" {
                            // Cancel the custom message input
                            bot_deps.welcome_service.clear_input_state(&key)?;
                            bot.send_message(
                                msg.chat.id,
                                "❌ Custom message input cancelled."
                            ).await?;
                            return Ok(());
                        }

                        // Update the welcome settings with custom message
                        let mut settings = bot_deps.welcome_service.get_settings(msg.chat.id);
                        settings.custom_message = Some(text.to_string());
                        settings.last_updated = chrono::Utc::now().timestamp();

                        if let Err(e) = bot_deps.welcome_service.save_settings(msg.chat.id, settings) {
                            bot.send_message(
                                msg.chat.id,
                                format!("❌ Failed to save custom message: {}", e)
                            ).await?;
                            return Ok(());
                        }

                        // Clear the input state
                        bot_deps.welcome_service.clear_input_state(&key)?;

                        // Send success message
                        bot.send_message(
                            msg.chat.id,
                            "✅ <b>Custom welcome message updated successfully!</b>\n\n\
                            New members will now see your custom message with placeholders replaced."
                        ).parse_mode(teloxide::types::ParseMode::Html).await?;

                        return Ok(());
                    } else {
                        // Empty text, ask for valid input
                        bot.send_message(
                            msg.chat.id,
                            "❌ Please enter a valid welcome message. Use /cancel to cancel."
                        ).await?;
                        return Ok(());
                    }
                }
            }
        }

        // Sponsor settings input mode: capture replies
        if let Some(user) = &msg.from {
            let current_group_id = msg.chat.id.to_string();
            
            // Check if there's an active sponsor input mode for this group
            if let Some(sponsor_state) = bot_deps.sponsor.get_sponsor_state(current_group_id.clone()) {
                // Only process if the user is an admin
                let is_admin = utils::is_admin(&bot, msg.chat.id, user.id).await;
                if !is_admin {
                    // Non-admin users typing during sponsor setup - ignore silently
                    return Ok(());
                }

                // Check if this admin is the one who started the action
                if let Some(admin_user_id) = sponsor_state.admin_user_id {
                    if admin_user_id != user.id.0 {
                        // Other admin users typing during sponsor setup - ignore silently
                        return Ok(());
                    }
                }

                if let Some(text) = msg.text() {
                    let text = text.trim();
                    if !text.is_empty() {
                        match sponsor_state.step {
                            crate::sponsor::dto::SponsorStep::AwaitingRequestLimit => {
                                // Parse the request limit number
                                match text.parse::<u64>() {
                                    Ok(limit) => {
                                        // Validate the limit
                                        if limit == 0 {
                                            bot.send_message(
                                                msg.chat.id,
                                                "❌ Request limit cannot be 0. Please enter a number greater than 0."
                                            )
                                            .await?;
                                            return Ok(());
                                        }

                                        // Update the sponsor settings
                                        let mut settings = bot_deps.sponsor.get_sponsor_settings(current_group_id.clone());
                                        settings.requests = limit;

                                        if let Err(e) = bot_deps
                                            .sponsor
                                            .set_or_update_sponsor_settings(current_group_id.clone(), settings.clone())
                                        {
                                            bot.send_message(
                                                msg.chat.id,
                                                format!("❌ Failed to update request limit: {}", e)
                                            )
                                            .await?;
                                            return Ok(());
                                        }

                                        // Reset requests to new limit when limit changes
                                        let new_requests = crate::sponsor::dto::SponsorRequest {
                                            requests_left: limit,
                                            last_request: chrono::Utc::now().timestamp() as u64,
                                        };

                                        if let Err(e) = bot_deps
                                            .sponsor
                                            .set_or_update_sponsor_requests(current_group_id.clone(), new_requests)
                                        {
                                            log::warn!("Failed to reset requests after limit change: {}", e);
                                        }

                                        // Clear the sponsor state
                                        if let Err(e) = bot_deps.sponsor.remove_sponsor_state(current_group_id.clone()) {
                                            log::warn!("Failed to remove sponsor state: {}", e);
                                        }

                                        // Send success message
                                        bot.send_message(
                                            msg.chat.id,
                                            format!("✅ <b>Request limit updated to {} per interval</b>", limit)
                                        )
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await?;

                                        let settings = bot_deps.sponsor.get_sponsor_settings(current_group_id.to_string());
                                        let (requests_left, total_requests) = bot_deps
                                            .sponsor
                                            .get_request_status(current_group_id.to_string())
                                            .unwrap_or((0, 0));

                                        let interval_text = match settings.interval {
                                            SponsorInterval::Hourly => "Hourly",
                                            SponsorInterval::Daily => "Daily",
                                            SponsorInterval::Weekly => "Weekly",
                                            SponsorInterval::Monthly => "Monthly",
                                        };

                                        let text = format!(
                                            "🎯 <b>Sponsor Settings</b>\n\n\
                                            <b>Current Status:</b>\n\
                                            • Total Requests: <b>{}</b>\n\
                                            • Requests Left: <b>{}</b>\n\
                                            • Interval: <b>{}</b>\n\n\
                                            <b>How it works:</b>\n\
                                            • Users can use <code>/g</code> command\n\
                                            • No registration required\n\
                                            • Requests reset every interval\n\
                                            • Only admins can change settings\n\n\
                                            Choose an action below:",
                                            total_requests, requests_left, interval_text
                                        );

                                        let kb = InlineKeyboardMarkup::new(vec![
                                            vec![InlineKeyboardButton::callback(
                                                "📊 Set Request Limit",
                                                "sponsor_set_requests",
                                            )],
                                            vec![InlineKeyboardButton::callback(
                                                "⏰ Set Interval",
                                                "sponsor_set_interval",
                                            )],
                                            vec![InlineKeyboardButton::callback(
                                                "🚫 Disable Sponsor",
                                                "sponsor_disable",
                                            )],
                                            vec![InlineKeyboardButton::callback(
                                                "↩️ Back",
                                                "back_to_group_settings",
                                            )],
                                        ]);

                                        bot.send_message(chat_id, text)
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(kb)
                                            .await?;

                                        return Ok(());
                                    }
                                    Err(_) => {
                                        bot.send_message(
                                            msg.chat.id,
                                            "❌ Invalid input. Please enter a valid number (e.g., 5, 10, 25, 100)."
                                        )
                                        .await?;
                                        return Ok(());
                                    }
                                }
                            }
                            _ => {
                                // Unknown step, clear sponsor state
                                if let Err(e) = bot_deps.sponsor.remove_sponsor_state(current_group_id.clone()) {
                                    log::warn!("Failed to remove sponsor state: {}", e);
                                }
                                bot.send_message(msg.chat.id, "❌ Unknown input step. Please try again.")
                                    .await?;
                                return Ok(());
                            }
                        }
                    } else {
                        // Empty text, ask for valid input
                        bot.send_message(
                            msg.chat.id,
                            "❌ Please enter a valid number for the request limit."
                        )
                        .await?;
                        return Ok(());
                    }
                } else {
                    // No text, ask for valid input
                    bot.send_message(
                        msg.chat.id,
                        "❌ Please send a text message with the number for the request limit."
                    )
                    .await?;
                    return Ok(());
                }
            }
        }

        // Moderation settings wizard: capture replies
        if let Some(user) = &msg.from {
            // Only process moderation wizard if user is actually in wizard state
            if let Ok(mut moderation_state) = bot_deps.moderation.get_moderation_state(chat_id.to_string()) {
                let text = msg
                    .text()
                    .or_else(|| msg.caption())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    let parse_items = |s: &str| -> Vec<String> {
                    s.split(';')
                        .map(|x| x.trim())
                        .filter(|x| !x.is_empty())
                        .take(50)
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                    };
                    if moderation_state.step == "AwaitingAllowed" {
                        let is_skip = text.eq_ignore_ascii_case("na");
                        let items = if is_skip {
                            Vec::new()
                        } else {
                            parse_items(&text)
                        };
                        moderation_state.allowed_items = Some(items);
                        moderation_state.step = "AwaitingDisallowed".to_string();
                        // Remove previous prompt (Step 1) if present
                        if let Some(mid) = moderation_state.message_id {
                            let _ = bot
                                .delete_message(msg.chat.id, teloxide::types::MessageId(mid as i32))
                                .await;
                        }
                        let sent = bot
                            .send_message(
                                msg.chat.id,
                                "🛡️ <b>Moderation Settings — Step 2/2</b>\n\n<b>Now send DISALLOWED items</b> for this group.\n\n<b>Be specific</b>: include concrete phrases, patterns, and examples you want flagged.\n\n<b>Cancel anytime</b>: Tap <b>Back</b> or <b>Close</b> in the Moderation menu — this prompt will be removed.\n\n<b>Format</b>:\n- Send them in a <b>single message</b>\n- Separate each item with <code>;</code>\n- To skip this section, send <code>na</code>\n\n<b>Examples (community standards)</b>:\n<code>harassment, insults, or personal attacks; hate speech or slurs (racism, homophobia, etc.); doxxing or sharing private information; NSFW/explicit content; graphic violence/gore; off-topic spam or mass mentions; repeated flooding/emoji spam; political or religious debates (off-topic); promotion of unrelated/non-affiliated projects; misinformation/FUD targeting members</code>\n\n<i>Notes:</i> \n- Avoid duplicating default scam rules (phishing links, wallet approvals, DM requests, giveaways) — those are already enforced by Default Rules.\n- <b>Group Disallowed</b> > <b>Group Allowed</b> > <b>Default Rules</b> (strict priority).\n- If any Group Disallowed item matches, the message will be flagged.",
                            )
                            .parse_mode(ParseMode::Html)
                            .await?;
                        // Track Step 2 prompt for cleanup
                        moderation_state.message_id = Some(sent.id.0 as i64);
                        bot_deps.moderation.set_moderation_state(chat_id.to_string(), moderation_state).unwrap();
                        return Ok(());
                    } else if moderation_state.step == "AwaitingDisallowed" {
                        let is_skip = text.eq_ignore_ascii_case("na");
                        let disallowed = if is_skip {
                            Vec::new()
                        } else {
                            parse_items(&text)
                        };
                        let allowed = moderation_state.allowed_items.unwrap_or_default();
                        // Save to moderation_settings tree
                        let settings = ModerationSettings::from((allowed.clone(), disallowed.clone(), user.id.0 as i64, chrono::Utc::now().timestamp_millis()));
                        bot_deps.moderation.set_or_update_moderation_settings(chat_id.to_string(), settings).unwrap();
                        // Clear wizard and remove last prompt if present
                        if let Some(mid) = moderation_state.message_id {
                            let _ = bot
                                .delete_message(msg.chat.id, teloxide::types::MessageId(mid as i32))
                                .await;
                        }
                        bot_deps.moderation.remove_moderation_state(chat_id.to_string()).unwrap();
                        let allowed_list = if allowed.is_empty() {
                            "<i>(none)</i>".to_string()
                        } else {
                            allowed
                                .iter()
                                .map(|x| format!("• {}", teloxide::utils::html::escape(x)))
                                .collect::<Vec<_>>()
                                .join("\n")
                        };
                        let disallowed_list = if disallowed.is_empty() {
                            "<i>(none)</i>".to_string()
                        } else {
                            disallowed
                                .iter()
                                .map(|x| format!("• {}", teloxide::utils::html::escape(x)))
                                .collect::<Vec<_>>()
                                .join("\n")
                        };
                        let mut summary = format!(
                            "✅ <b>Custom moderation rules saved.</b>\n\n<b>Allowed ({})</b>:\n{}\n\n<b>Disallowed ({})</b>:\n{}",
                            allowed.len(),
                            allowed_list,
                            disallowed.len(),
                            disallowed_list,
                        );
                        if allowed.is_empty() && disallowed.is_empty() {
                            summary.push_str("\n\n<i>No custom rules recorded. Default moderation rules remain fully in effect.</i>");
                        }
                        bot.send_message(msg.chat.id, summary)
                            .parse_mode(ParseMode::Html)
                            .await?;
                        return Ok(());
                    }
                }
            }
        }

        // Scheduled prompts wizard: capture prompt text either as a reply or as a follow-up message
        if let Some(user) = &msg.from {
            let key = (&msg.chat.id.0, &(user.id.0 as i64));
            if let Some(mut st) = bot_deps.scheduled_storage.get_pending(key) {
                if st.step == PendingStep::AwaitingPrompt {
                    // Accept prompt if message is a reply OR a regular follow-up (non-command) from the same user
                    let is_reply = msg.reply_to_message().is_some();
                    let text_raw = msg.text().or_else(|| msg.caption()).unwrap_or("");
                    let is_command = text_raw.trim_start().starts_with('/');
                    if is_reply || (!is_command && !text_raw.trim().is_empty()) {
                        let text = text_raw.to_string();
                        // Guard scheduled prompt against forbidden tools
                        {
                            let guard = &bot_deps.schedule_guard;
                            match guard.check_prompt(&text).await {
                                Ok(res) => {
                                        // Bill the group for the guard check like moderation
                                        if let Some(group_credentials) = bot_deps.group.get_credentials(msg.chat.id) {
                                            if let Err(e) = create_purchase_request(
                                                0, // file_search
                                                0, // web_search
                                                0, // image_gen
                                                res.total_tokens,
                                                Model::GPT5Nano,
                                                &group_credentials.jwt,
                                                Some(msg.chat.id.0.to_string()),
                                                (user.id.0 as i64).to_string(),
                                                bot_deps.clone(),
                                            )
                                            .await
                                            {
                                                log::warn!("schedule guard purchase request failed: {}", e);
                                            }
                                        }
                                        if res.verdict == "F" {
                                            let reason = res
                                                .reason
                                                .unwrap_or_else(|| "Prompt requests a forbidden action for scheduled runs".to_string());
                                            let warn = format!(
                                                "❌ This prompt can't be scheduled. PLEASE TRY AGAIN\n\n<b>Reason:</b> {}\n\n<b>Allowed for schedules</b>: informational queries, analytics, web/file search, time, market snapshots, and image generation.\n\n<b>Blocked</b>: payments/transfers, withdrawals/funding, DAO/proposal creation, or any on-chain/interactive actions.\n\nPlease send a new prompt (you can just send it here without replying).",
                                                teloxide::utils::html::escape(&reason)
                                            );
                                            bot.send_message(msg.chat.id, warn)
                                                .parse_mode(ParseMode::Html)
                                                .await?;
                                            // Do not advance wizard; let user try again by sending a new prompt
                                            return Ok(());
                                        }
                                }
                                Err(e) => {
                                    log::warn!("schedule_guard check failed: {}", e);
                                }
                            }
                        }

                        st.prompt = Some(text);
                        st.step = PendingStep::AwaitingHour;
                        if let Err(e) = bot_deps.scheduled_storage.put_pending(key, &st) {
                            log::error!("Failed to persist scheduled wizard state: {}", e);
                            bot.send_message(msg.chat.id, "❌ Error saving schedule state. Please try /scheduleprompt again.")
                                    .await?;
                            return Ok(());
                        }
                        let kb = build_hours_keyboard();
                        bot.send_message(msg.chat.id, "Select start hour (UTC)")
                            .reply_markup(kb)
                            .await?;
                        return Ok(());
                    }
                }
            }
        }

        // Scheduled payments wizard: capture free text steps
        if let Some(user) = &msg.from {
            let pay_key = (&msg.chat.id.0, &(user.id.0 as i64));
            if let Some(mut st) = bot_deps.scheduled_payments.get_pending(pay_key) {
                let text_raw = msg.text().or_else(|| msg.caption()).unwrap_or("").trim().to_string();
                if text_raw.eq_ignore_ascii_case("/cancel") || text_raw.to_lowercase().starts_with("/cancel@") {
                    bot_deps.scheduled_payments.delete_pending(pay_key)?;
                    bot.send_message(msg.chat.id, "✅ Cancelled scheduled payment setup.").await?;
                    return Ok(());
                }
                if text_raw.is_empty() || text_raw.starts_with('/') { return Ok(()); }
                match st.step {
                    crate::scheduled_payments::dto::PendingPaymentStep::AwaitingRecipient => {
                        // Expect @username
                        let uname = text_raw.trim_start_matches('@').to_string();
                        if let Some(creds) = bot_deps.auth.get_credentials(&uname) {
                            st.recipient_username = Some(uname);
                            st.recipient_address = Some(creds.resource_account_address);
                            st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingToken;
                            bot_deps.scheduled_payments.put_pending(pay_key, &st)?;
                            bot.send_message(msg.chat.id, "💳 Send token symbol (e.g., APT, USDC, or emoji)").await?;
                        } else {
                            bot.send_message(msg.chat.id, "❌ Unknown user. Please send a valid @username.").await?;
                        }
                        return Ok(());
                    }
                    crate::scheduled_payments::dto::PendingPaymentStep::AwaitingToken => {
                        let symbol_input = if text_raw.chars().any(|c| c.is_ascii_alphabetic()) { text_raw.to_uppercase() } else { text_raw.clone() };
                        let (token_type, decimals, symbol) = if symbol_input.eq_ignore_ascii_case("APT") || symbol_input.eq_ignore_ascii_case("APTOS") {
                            ("0x1::aptos_coin::AptosCoin".to_string(), 8u8, "APT".to_string())
                        } else {
                            match bot_deps.panora.get_token_by_symbol(&symbol_input).await {
                                Ok(token) => {
                                    let t = if token.token_address.is_some() { token.token_address.unwrap() } else { token.fa_address };
                                    (t, token.decimals, token.symbol)
                                }
                                Err(_) => {
                                    bot.send_message(msg.chat.id, "❌ Token not found. Try again (e.g., APT, USDC)").await?;
                                    return Ok(());
                                }
                            }
                        };
                        st.symbol = Some(symbol);
                        st.token_type = Some(token_type);
                        st.decimals = Some(decimals);
                        st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingAmount;
                        bot_deps.scheduled_payments.put_pending(pay_key, &st)?;
                        bot.send_message(msg.chat.id, "💰 Send amount (decimal)").await?;
                        return Ok(());
                    }
                    crate::scheduled_payments::dto::PendingPaymentStep::AwaitingAmount => {
                        let parsed = text_raw.replace('_', "").replace(',', "");
                        match parsed.parse::<f64>() {
                            Ok(v) if v > 0.0 => {
                                st.amount_display = Some(v);
                                st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingDate;
                                bot_deps.scheduled_payments.put_pending(pay_key, &st)?;
                                bot.send_message(msg.chat.id, "📅 Send start date in YYYY-MM-DD (UTC)").await?;
                            }
                            _ => {
                                bot.send_message(msg.chat.id, "❌ Invalid amount. Please send a positive number.").await?;
                            }
                        }
                        return Ok(());
                    }
                    crate::scheduled_payments::dto::PendingPaymentStep::AwaitingDate => {
                        if chrono::NaiveDate::parse_from_str(&text_raw, "%Y-%m-%d").is_ok() {
                            st.date = Some(text_raw);
                            st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingHour;
                            bot_deps.scheduled_payments.put_pending(pay_key, &st)?;
                            let kb = crate::scheduled_payments::wizard::build_hours_keyboard_payments();
                            bot.send_message(msg.chat.id, "⏰ Select hour (UTC)").reply_markup(kb).await?;
                        } else {
                            bot.send_message(msg.chat.id, "❌ Invalid date. Use YYYY-MM-DD.").await?;
                        }
                        return Ok(());
                    }
                    crate::scheduled_payments::dto::PendingPaymentStep::AwaitingConfirm => {
                        // Support 'skip' to keep existing values during edit flow
                        if text_raw.eq_ignore_ascii_case("skip") {
                            // do nothing, keep values
                            bot.send_message(msg.chat.id, "✔️ Keeping existing values. Use buttons to confirm.").await?;
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check if sentinel is on for this group
        let sentinel_on = bot_deps.sentinel.get_sentinel(chat_id.to_string());
        if sentinel_on {
            // Skip moderation if this user is in moderation settings wizard
            if let Some(_) = &msg.from {
                let moderation_state = bot_deps.moderation.get_moderation_state(chat_id.to_string()).unwrap_or(ModerationState {
                    step: "AwaitingAllowed".to_string(),
                    allowed_items: None,
                    message_id: None,
                });
                if moderation_state.step == "AwaitingAllowed" {
                    return Ok(());
                }
            }
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
            }
            // Note: No early return for forwarded/channel messages (msg.from is None)
            // These will now be moderated to catch scams and spam

            let address = group_credentials.resource_account_address;

            let default_payment_prefs = bot_deps.default_payment_prefs.clone();

            let coin = bot_deps.payment.get_payment_token(msg.chat.id.to_string()).unwrap_or(PaymentPrefs::from((default_payment_prefs.label, default_payment_prefs.currency, default_payment_prefs.version)));

            let group_balance = bot_deps
            .panora
            .aptos
            .get_account_balance(&address, &coin.currency)
                .await?;

            let token = bot_deps.panora.get_token_by_symbol(&coin.label).await;

            if token.is_err() {
                bot.send_message(msg.chat.id, "❌ Token not found, please contact support")
                    .await?;
                return Ok(());
            }

            let token = token.unwrap();

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
                        "User balance is less than the minimum deposit. Please fund your account transfering {} to <code>{}</code> address. Minimum deposit: {} {} (Your balance: {} {})",
                        token.symbol, 
                        address,
                        min_deposit_formatted,
                        token.symbol,
                        group_balance_formatted,
                        token.symbol
                    )
                )
                .parse_mode(ParseMode::Html)
                .await?;
                return Ok(());
            }

            // Use the same moderation logic as /mod, via injected dependency
            let moderation_service = bot_deps.moderation.clone();
            // Load overrides
            let overrides = bot_deps.moderation.get_moderation_settings(chat_id.to_string());

            let overrides = match overrides {
                Ok(overrides) => Some(ModerationOverrides {
                    allowed_items: overrides.allowed_items,
                    disallowed_items: overrides.disallowed_items,
                }),
                Err(e) => {
                    log::error!("Failed to get moderation settings: {}", e);
                    None
                }
            };

            let message_text = msg.text().or_else(|| msg.caption()).unwrap_or("");
            match moderation_service
                .moderate_message(message_text, &bot, &msg, &msg, overrides)
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
                        // For forwarded messages, use a placeholder user_id since we can't identify the sender
                        let user_id = if let Some(user) = &msg.from {
                            user.id.0.to_string()
                        } else {
                            "forwarded_message".to_string()
                        };
                        
                        let purchase_result = create_purchase_request(
                            0,
                            0,
                            0,
                            result.total_tokens,
                            Model::GPT5Nano,
                            &group_credentials.jwt,
                            Some(msg.chat.id.0.to_string()),
                            user_id,
                            bot_deps,
                        )
                        .await;

                        if let Err(e) = purchase_result {
                            log::error!("Failed to purchase ai for flagged content: {}", e);
                            return Ok(());
                        }
                    }

                    if result.verdict == "F" {
                        // Handle both regular user messages and forwarded messages
                        if let Some(flagged_user) = &msg.from {
                            // Regular user message - can mute
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
                            
                            // Add admin buttons for regular users
                            let keyboard = InlineKeyboardMarkup::new(vec![vec![
                                InlineKeyboardButton::callback(
                                    "🔇 Unmute",
                                    format!("unmute:{}", flagged_user.id),
                                ),
                                InlineKeyboardButton::callback(
                                    "🚫 Ban",
                                    format!("ban:{}:{}", flagged_user.id, msg.id.0),
                                ),
                            ]]);
                            
                            // Build a visible user mention (prefer @username, else clickable name)
                            let user_mention = if let Some(username) = &flagged_user.username {
                                format!("@{}", username)
                            } else {
                                let name = teloxide::utils::html::escape(&flagged_user.first_name);
                                format!(
                                    "<a href=\"tg://user?id={}\">{}</a>",
                                    flagged_user.id.0, name
                                )
                            };

                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "🛡️ <b>Content Flagged & User Muted</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n🔇 User has been muted\n👤 <b>User:</b> {}\n\n💬 <i>Flagged message:</i>\n<blockquote><span class=\"tg-spoiler\">{}</span></blockquote>",
                                    msg.id,
                                    user_mention,
                                    teloxide::utils::html::escape(message_text)
                                )
                            )
                            .parse_mode(ParseMode::Html)
                            .reply_markup(keyboard)
                            .await?;
                        } else {
                            // Forwarded message - log details and show info
                            // Since we can't access forward metadata directly, we'll identify it by the absence of msg.from
                            log::info!(
                                "Forwarded message flagged: chat_id={}, msg_id={}, text={}",
                                msg.chat.id.0, msg.id.0, message_text
                            );
                            
                            let user_mention = "Forwarded message (no sender info)".to_string();
                            
                            // For forwarded messages, just show delete option
                            let keyboard = InlineKeyboardMarkup::new(vec![vec![
                                InlineKeyboardButton::callback(
                                    "🗑️ Delete Only",
                                    format!("delete_only:{}", msg.id.0),
                                ),
                            ]]);

                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "🛡️ <b>Forwarded Content Flagged</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n⚠️ Cannot mute (forwarded message)\n👤 <b>Source:</b> {}\n\n💬 <i>Flagged message:</i>\n<blockquote><span class=\"tg-spoiler\">{}</span></blockquote>",
                                    msg.id,
                                    user_mention,
                                    teloxide::utils::html::escape(message_text)
                                )
                            )
                            .parse_mode(ParseMode::Html)
                            .reply_markup(keyboard)
                            .await?;
                        }
                        
                        // Always delete the offending message (both regular and forwarded)
                        if let Err(e) = bot.delete_message(msg.chat.id, msg.id).await {
                            log::warn!(
                                "Failed to delete offending message {}: {}",
                                msg.id.0,
                                e
                            );
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

// removed: handle_sentinel — sentinel toggling is available in Group Settings → Moderation

// removed: handle_moderation_settings — wizard now launched via /groupsettings Moderation menu

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
        let sentinel_on = bot_deps.sentinel.get_sentinel(msg.chat.id.to_string());

        if sentinel_on {
            bot.send_message(
                msg.chat.id,
                "🛡️ <b>Sentinel Mode Active</b>\n\n/report is disabled while sentinel is ON. All messages are being automatically moderated."
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
        let user = reply_to_msg.from.clone();

        if user.is_none() {
            bot.send_message(msg.chat.id, "❌ User not found").await?;
            return Ok(());
        }

        let user = user.unwrap();

        let user_id = user.id.to_string();

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
        let moderation_service = bot_deps.moderation.clone();

        // Moderate the message
        // Load overrides
        let formatted_group_id = format!("{}-{}", msg.chat.id.0, bot_deps.group.account_seed);
        let settings_tree = bot_deps.db.open_tree("moderation_settings").unwrap();
        let overrides = if let Ok(Some(raw)) = settings_tree.get(formatted_group_id.as_bytes()) {
            #[derive(Serialize, Deserialize)]
            struct ModerationSettings {
                allowed_items: Vec<String>,
                disallowed_items: Vec<String>,
                updated_by_user_id: i64,
                updated_at_unix_ms: i64,
            }
            if let Ok(ms) = serde_json::from_slice::<ModerationSettings>(&raw) {
                Some(ModerationOverrides {
                    allowed_items: ms.allowed_items,
                    disallowed_items: ms.disallowed_items,
                })
            } else {
                None
            }
        } else {
            None
        };
        match moderation_service
            .moderate_message(message_text, &bot, &msg, &reply_to_msg, overrides)
            .await
        {
            Ok(result) => {
                log::info!(
                    "Manual moderation result: {} for message: {} (tokens: {})",
                    result.verdict,
                    message_text,
                    result.total_tokens
                );

                let purchase_result = create_purchase_request(
                    0,
                    0,
                    0,
                    result.total_tokens,
                    Model::GPT5Nano,
                    &group_credentials.unwrap().jwt,
                    Some(msg.chat.id.0.to_string()),
                    user_id,
                    bot_deps,
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
                                format!("ban:{}:{}", flagged_user.id, reply_to_msg.id.0),
                            ),
                        ]]);

                        // Build a visible user mention (prefer @username, else clickable name)
                        let user_mention = if let Some(username) = &flagged_user.username {
                            format!("@{}", username)
                        } else {
                            let name = teloxide::utils::html::escape(&flagged_user.first_name);
                            format!(
                                "<a href=\"tg://user?id={}\">{}</a>",
                                flagged_user.id.0, name
                            )
                        };

                        // Send the flagged message response
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🛡️ <b>Content Flagged & User Muted</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n🔇 User has been muted\n👤 <b>User:</b> {}\n\n💬 <i>Flagged message:</i>\n<blockquote><span class=\"tg-spoiler\">{}</span></blockquote>",
                                reply_to_msg.id,
                                user_mention,
                                teloxide::utils::html::escape(message_text)
                            )
                        )
                        .parse_mode(ParseMode::Html)
                        .reply_markup(keyboard)
                        .await?;
                        // Immediately remove the offending message from the chat
                        if let Err(e) = bot.delete_message(msg.chat.id, reply_to_msg.id).await {
                            log::warn!(
                                "Failed to delete offending replied message {}: {}",
                                reply_to_msg.id.0,
                                e
                            );
                        }
                    } else {
                        // Fallback if no user found in the replied message
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🛡️ <b>Content Flagged</b>\n\n📝 Message ID: <code>{}</code>\n\n❌ Status: <b>FLAGGED</b> 🔴\n⚠️ Could not identify user to mute\n\n💬 <i>Flagged message:</i>\n<blockquote><span class=\"tg-spoiler\">{}</span></blockquote>",
                                reply_to_msg.id,
                                teloxide::utils::html::escape(message_text)
                            )
                        )
                        .parse_mode(ParseMode::Html)
                        .await?;
                        // Remove the offending message regardless
                        if let Err(e) = bot.delete_message(msg.chat.id, reply_to_msg.id).await {
                            log::warn!(
                                "Failed to delete offending replied message {}: {}",
                                reply_to_msg.id.0,
                                e
                            );
                        }
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
            "❌ <b>Invalid Usage</b>\n\n📝 The <code>/report</code> command must be used in reply to a message.\n\n💡 <b>How to use:</b>\n1. Find the message you want to moderate\n2. Reply to that message with <code>/report</code>\n\n🛡️ This will analyze the content of the replied message for violations."
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
        format!("💰 <b>Balance</b>: {:.6} {}", human_balance, token_symbol),
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
        format!("💰 <b>Balance</b>: {:.6} {}", human_balance, token_symbol),
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

async fn check_group_resource_account_address(
    bot: &Bot,
    group_credentials: GroupCredentials,
    msg: Message,
    bot_deps: &BotDependencies,
) -> AnyResult<GroupCredentials> {
    let group_id = group_credentials.group_id.clone();

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
