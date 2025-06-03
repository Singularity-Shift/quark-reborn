//! File upload and processing logic for quark_bot.

use teloxide::prelude::*;
use sled::Db;
use teloxide::net::Download;

pub async fn handle_file_upload(bot: Bot, msg: Message, db: Db, openai_api_key: String) -> Result<(), teloxide::RequestError> {
    use quark_backend::ai::upload_files_to_vector_store;
    use tokio::fs::File;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
    let mut file_paths = Vec::new();
    // Handle document
    if let Some(document) = msg.document() {
        let file_id = &document.file.id;
        let file_info = bot.get_file(file_id).await?;
        let filename = document.file_name.clone().unwrap_or_else(|| "document.bin".to_string());
        let file_path = format!("/tmp/{}_{}", user_id, filename);
        let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }
    // Handle photo (largest size)
    if let Some(photos) = msg.photo() {
        if let Some(photo) = photos.last() {
            let file_id = &photo.file.id;
            let file_info = bot.get_file(file_id).await?;
            let file_path = format!("/tmp/{}_photo_{}.jpg", user_id, photo.file.id);
            let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
            bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
            file_paths.push(file_path);
        }
    }
    // Handle video
    if let Some(video) = msg.video() {
        let file_id = &video.file.id;
        let file_info = bot.get_file(file_id).await?;
        let filename = video.file_name.clone().unwrap_or_else(|| "video.mp4".to_string());
        let file_path = format!("/tmp/{}_{}", user_id, filename);
        let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }
    // Handle audio
    if let Some(audio) = msg.audio() {
        let file_id = &audio.file.id;
        let file_info = bot.get_file(file_id).await?;
        let filename = audio.file_name.clone().unwrap_or_else(|| "audio.mp3".to_string());
        let file_path = format!("/tmp/{}_{}", user_id, filename);
        let mut file = File::create(&file_path).await.map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file).await.map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }
    if !file_paths.is_empty() {
        match upload_files_to_vector_store(user_id, &db, &openai_api_key, file_paths.clone()).await {
            Ok(vector_store_id) => {
                let file_count = file_paths.len();
                let files_text = if file_count == 1 { "file" } else { "files" };
                bot.send_message(msg.chat.id, format!("✅ {} {} uploaded and indexed! Your vector store ID: {}", file_count, files_text, vector_store_id)).await?;
            },
            Err(e) => {
                bot.send_message(msg.chat.id, format!("[Upload error]: {}", e)).await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, "No supported files found in your message. Please attach documents, photos, videos, or audio files.").await?;
    }
    Ok(())
} 