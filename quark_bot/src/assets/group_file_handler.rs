//! Group file upload and processing logic for quark_bot.

use crate::ai::group_vector_store::upload_files_to_group_vector_store;
use crate::dependencies::BotDependencies;
use crate::utils::{self, send_message};
use anyhow::Result as AnyResult;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::ChatAction;
use tokio::sync::Mutex;
use tokio::time::sleep;

/// Simple state manager for tracking which groups are awaiting file uploads
#[derive(Clone)]
pub struct GroupFileUploadState {
    awaiting_groups: Arc<Mutex<HashSet<String>>>,
}

impl GroupFileUploadState {
    pub fn new() -> Self {
        Self {
            awaiting_groups: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn set_awaiting(&self, group_id: String) {
        let mut awaiting = self.awaiting_groups.lock().await;
        awaiting.insert(group_id);
    }

    pub async fn clear_awaiting(&self, group_id: String) {
        let mut awaiting = self.awaiting_groups.lock().await;
        awaiting.remove(&group_id);
    }

    pub async fn is_awaiting(&self, group_id: String) -> bool {
        let awaiting = self.awaiting_groups.lock().await;
        awaiting.contains(&group_id)
    }
}

pub async fn handle_group_file_upload(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> AnyResult<()> {
    use tokio::fs::File;
    
    // Ensure this is a group chat
    if msg.chat.is_private() {
        send_message(
            msg,
            bot,
            "❌ Group document uploads can only be used in group chats.".to_string(),
        )
        .await?;
        return Ok(());
    }

    let group_id = msg.chat.id.to_string();

    // Check if user is admin
    let user = msg.from.as_ref();
    if user.is_none() {
        send_message(
            msg,
            bot,
            "❌ Unable to verify permissions.".to_string(),
        )
        .await?;
        return Ok(());
    }

    let user = user.unwrap();
    let is_admin = utils::is_admin(&bot, msg.chat.id, user.id).await;
    if !is_admin {
        send_message(
            msg,
            bot,
            "❌ Only group administrators can upload files to the group document library.".to_string(),
        )
        .await?;
        return Ok(());
    }

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
        let file_path = format!("/tmp/group_{}_{}", group_id, filename);
        let mut file = File::create(&file_path)
            .await
            .map_err(|e| teloxide::RequestError::from(std::sync::Arc::new(e)))?;
        bot.download_file(&file_info.path, &mut file)
            .await
            .map_err(|e| teloxide::RequestError::from(e))?;
        file_paths.push(file_path);
    }

    // Only documents are supported for vector stores
    // Photos, videos, and audio cannot be processed for semantic search

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
            upload_files_to_group_vector_store(group_id.clone(), bot_deps.clone(), file_paths.clone()).await;

        // Stop the typing indicator task
        typing_indicator_handle.abort();

        // Clear the awaiting state after upload attempt
        bot_deps.group_file_upload_state.clear_awaiting(group_id.clone()).await;

        match upload_result {
            Ok(vector_store_id) => {
                let file_count = file_paths.len();
                let files_text = if file_count == 1 { "file" } else { "files" };
                send_message(
                    msg,
                    bot,
                    format!(
                        "✅ {} {} uploaded to group document library! Vector store ID: {}",
                        file_count, files_text, vector_store_id
                    ),
                )
                .await?;
            }
            Err(e) => {
                send_message(msg, bot, format!("[Group upload error]: {}", e).to_string()).await?;
            }
        }
    } else {
        send_message(msg, bot, "❌ No supported files found in your message. Please attach documents (.txt, .md, .py, .js, .pdf, .docx, etc.) that can be processed for semantic search.".to_string()).await?;
    }
    Ok(())
}
