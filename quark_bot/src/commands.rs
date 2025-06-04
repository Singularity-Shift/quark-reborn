//! Command handlers for quark_bot Telegram bot.

use teloxide::prelude::*;
use sled::Db;
use crate::utils;
use contracts::aptos::simulate_aptos_contract_call;

pub async fn handle_login_user(bot: Bot, msg: Message, db: Db) -> Result<(), teloxide::RequestError> {
    let _db = db;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let mention = if let Some(user) = msg.from.as_ref() {
        format!("<a href=\"tg://user?id={}\">{}</a>", user.id.0, user.first_name)
    } else {
        format!("<a href=\"tg://user?id={}\">{}</a>", user_id, user_id)
    };
    let fake_address = format!("fake_address_{}", user_id);
    let _ = simulate_aptos_contract_call(user_id); // log only
    let reply = format!(
        "logged in as {}, please add 📒 token to address <code>{}</code> to use the bot",
        mention, fake_address
    );
    bot.send_message(msg.chat.id, reply)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn handle_login_group(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "under development").await?;
    Ok(())
}

pub async fn handle_help(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    use teloxide::utils::command::BotCommands;
    use crate::Command;
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

pub async fn handle_add_files(bot: Bot, msg: Message) -> Result<(), teloxide::RequestError> {
    bot.send_message(msg.chat.id, "📎 Please attach the files you wish to upload in your next message.\n\n✅ Supported: Documents, Photos, Videos, Audio files\n💡 You can send multiple files in one message!").await?;
    Ok(())
}

pub async fn handle_list_files(bot: Bot, msg: Message, db: Db, openai_api_key: String) -> Result<(), teloxide::RequestError> {
    let _openai_api_key = openai_api_key;
    use quark_backend::ai::list_user_files_with_names;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let user_convos = quark_backend::db::UserConversations::new(&db).unwrap();
    if let Some(_vector_store_id) = user_convos.get_vector_store_id(user_id) {
        match list_user_files_with_names(user_id, &db) {
            Ok(files) => {
                if files.is_empty() {
                    bot.send_message(msg.chat.id, "📁 <b>Your Document Library</b>\n\n<i>No files uploaded yet</i>\n\n💡 Use /add_files to start building your personal AI knowledge base!")
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                } else {
                    let file_list = files.iter()
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
                    use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
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
                            format!("delete_file:{}", file.id)
                        );
                        keyboard_rows.push(vec![delete_button]);
                    }
                    if files.len() > 1 {
                        let clear_all_button = InlineKeyboardButton::callback(
                            "🗑️ Clear All Files",
                            "clear_all_files"
                        );
                        keyboard_rows.push(vec![clear_all_button]);
                    }
                    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);
                    bot.send_message(msg.chat.id, response)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(keyboard)
                        .await?;
                }
            },
            Err(e) => {
                bot.send_message(msg.chat.id, format!("❌ <b>Error accessing your files</b>\n\n<i>Technical details:</i> {}", e))
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

pub async fn handle_chat(bot: Bot, msg: Message, db: Db, openai_api_key: String) -> Result<(), teloxide::RequestError> {
    if let Some(text) = msg.text() {
        let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
        let reply = match quark_backend::ai::generate_response(user_id, text, &db, &openai_api_key).await {
            Ok(resp) => resp,
            Err(e) => format!("[AI error]: {}", e),
        };
        bot.send_message(msg.chat.id, reply).await?;
    } else {
        bot.send_message(msg.chat.id, "Usage: /chat <your message>").await?;
    }
    Ok(())
}

pub async fn handle_new_chat(bot: Bot, msg: Message, db: Db) -> Result<(), teloxide::RequestError> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let user_convos = quark_backend::db::UserConversations::new(&db).unwrap();
    
    match user_convos.clear_response_id(user_id) {
        Ok(_) => {
            bot.send_message(msg.chat.id, "🆕 <b>New conversation started!</b>\n\n✨ Your previous chat history has been cleared. Your next /chat command will start a fresh conversation thread.\n\n💡 <i>Your uploaded files and settings remain intact</i>")
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        },
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ <b>Error starting new chat</b>\n\n<i>Technical details:</i> {}", e))
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
    }
    Ok(())
} 