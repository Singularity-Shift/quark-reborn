//! Callback query handlers for quark_bot.

use teloxide::prelude::*;
use sled::Db;
use crate::utils;

pub async fn handle_callback_query(bot: Bot, query: teloxide::types::CallbackQuery, db: Db, openai_api_key: String) -> Result<(), teloxide::RequestError> {
    if let Some(data) = &query.data {
        let user_id = query.from.id.0 as i64;
        if data.starts_with("delete_file:") {
            let file_id = data.strip_prefix("delete_file:").unwrap();
            use quark_backend::ai::delete_file_from_vector_store;
            let user_convos = quark_backend::db::UserConversations::new(&db).unwrap();
            if let Some(vector_store_id) = user_convos.get_vector_store_id(user_id) {
                match delete_file_from_vector_store(user_id, &db, &vector_store_id, file_id, &openai_api_key).await {
                    Ok(_) => {
                        bot.answer_callback_query(&query.id).await?;
                        use quark_backend::ai::list_user_files_with_names;
                        match list_user_files_with_names(user_id, &db) {
                            Ok(files) => {
                                use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
                                if files.is_empty() {
                                    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
                                        bot.edit_message_text(message.chat.id, message.id, "✅ <b>File deleted successfully!</b>\n\n📁 <i>Your document library is now empty</i>\n\n💡 Use /add_files to upload new documents")
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(InlineKeyboardMarkup::new(vec![] as Vec<Vec<InlineKeyboardButton>>))
                                            .await?;
                                    }
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
                                    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
                                        bot.edit_message_text(message.chat.id, message.id, response)
                                            .parse_mode(teloxide::types::ParseMode::Html)
                                            .reply_markup(keyboard)
                                            .await?;
                                    }
                                }
                            },
                            Err(_) => {
                                bot.answer_callback_query(&query.id)
                                    .text("❌ Error refreshing file list. Please try /list_files again.")
                                    .await?;
                            }
                        }
                    },
                    Err(e) => {
                        bot.answer_callback_query(&query.id)
                            .text(&format!("❌ Failed to delete file. Error: {}", e))
                            .await?;
                    }
                }
            }
        } else if data == "clear_all_files" {
            use quark_backend::ai::delete_vector_store;
            use teloxide::types::{InlineKeyboardMarkup, InlineKeyboardButton};
            match delete_vector_store(user_id, &db, &openai_api_key).await {
                Ok(_) => {
                    bot.answer_callback_query(&query.id).await?;
                    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
                        bot.edit_message_text(message.chat.id, message.id, "✅ <b>All files cleared successfully!</b>\n\n🗑️ <i>Your entire document library has been deleted</i>\n\n💡 Use /add_files to start building your library again")
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .reply_markup(InlineKeyboardMarkup::new(vec![] as Vec<Vec<InlineKeyboardButton>>))
                            .await?;
                    }
                },
                Err(e) => {
                    bot.answer_callback_query(&query.id)
                        .text(&format!("❌ Failed to clear files. Error: {}", e))
                        .await?;
                }
            }
        }
    }
    Ok(())
} 