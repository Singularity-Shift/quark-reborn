//! File upload and processing logic for quark_bot.

use crate::ai::vector_store::upload_files_to_vector_store;
use crate::dependencies::BotDependencies;
use std::time::Duration;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::ChatAction;
use tokio::time::sleep;

pub async fn handle_file_upload(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> Result<(), teloxide::RequestError> {
    use tokio::fs::File;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let chat_id = msg.chat.id;
    let mut file_paths = Vec::new();
    // Handle document
    if let Some(document) = msg.document() {
        let file_id = &document.file.id;
        let file_info = bot.get_file(file_id.clone()).await?;
        let filename = document
            .file_name
            .clone()
            .unwrap_or_else(|| "document.bin".to_string());
        let file_path = format!("/tmp/{}_{}", user_id, filename);
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file)
            .await
            .map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }
    // Handle all photos, not just the largest size
    if let Some(photos) = msg.photo() {
        // Process all photos in the message
        for photo in photos {
            let file_id = &photo.file.id;
            let file_info = bot.get_file(file_id.clone()).await?;
            let file_path = format!("/tmp/{}_photo_{}.jpg", user_id, photo.file.id);
            let mut file = File::create(&file_path)
                .await
                .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
            bot.download_file(&file_info.path, &mut file)
                .await
                .map_err(|e| teloxide::RequestError::from(e))?;
            file_paths.push(file_path);
        }
    }
    // Handle video
    if let Some(video) = msg.video() {
        let file_id = &video.file.id;
        let file_info = bot.get_file(file_id.clone()).await?;
        let filename = video
            .file_name
            .clone()
            .unwrap_or_else(|| "video.mp4".to_string());
        let file_path = format!("/tmp/{}_{}", user_id, filename);
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file)
            .await
            .map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }
    // Handle audio
    if let Some(audio) = msg.audio() {
        let file_id = &audio.file.id;
        let file_info = bot.get_file(file_id.clone()).await?;
        let filename = audio
            .file_name
            .clone()
            .unwrap_or_else(|| "audio.mp3".to_string());
        let file_path = format!("/tmp/{}_{}", user_id, filename);
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file)
            .await
            .map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }
    if !file_paths.is_empty() {
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

        let upload_result =
            upload_files_to_vector_store(user_id, bot_deps.clone(), file_paths.clone()).await;

        // Stop the typing indicator task
        typing_indicator_handle.abort();

        match upload_result {
            Ok(vector_store_id) => {
                let file_count = file_paths.len();
                let files_text = if file_count == 1 { "file" } else { "files" };
                bot.send_message(
                    chat_id,
                    format!(
                        "✅ {} {} uploaded and indexed! Your vector store ID: {}",
                        file_count, files_text, vector_store_id
                    ),
                )
                .await?;
            }
            Err(e) => {
                bot.send_message(chat_id, format!("[Upload error]: {}", e))
                    .await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, "No supported files found in your message. Please attach documents, photos, videos, or audio files.").await?;
    }
    Ok(())
}
