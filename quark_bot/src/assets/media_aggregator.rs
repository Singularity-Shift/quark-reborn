use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use crate::bot::handler::{handle_chat, handle_reasoning_chat};
use crate::ai::handler::AI;
use crate::credentials::handler::Auth;
use crate::group::handler::Group;
use crate::services::handler::Services;
use crate::user_model_preferences::handler::UserModelPreferences;
use sled::Db;

pub struct MediaGroupAggregator {
    // Key: media_group_id
    // Value: (Vec of messages in the group, debounce task handle)
    groups: DashMap<String, (Vec<Message>, tokio::task::JoinHandle<()>)>,
    bot: Bot,
    ai: AI,
    auth: Auth,
    group: Group,
    services: Services,
    user_model_prefs: UserModelPreferences,
    db: Db,
}

impl MediaGroupAggregator {
    pub fn new(
        bot: Bot,
        ai: AI,
        auth: Auth,
        group: Group,
        services: Services,
        user_model_prefs: UserModelPreferences,
        db: Db,
    ) -> Self {
        Self {
            groups: DashMap::new(),
            bot,
            ai,
            auth,
            group,
            services,
            user_model_prefs,
            db,
        }
    }

    pub async fn add_message(self: Arc<Self>, msg: Message) {
        let media_group_id = if let Some(id) = msg.media_group_id() {
            id.to_string()
        } else {
            return;
        };

        let mut entry = self
            .groups
            .entry(media_group_id.clone())
            .or_insert_with(|| (Vec::new(), tokio::spawn(async {})));

        // A new message has arrived for the group, so cancel the previous debounce task.
        entry.value().1.abort();

        // Add the new message to the group.
        entry.value_mut().0.push(msg);

        // Clone the Arc to move it into the new task.
        let aggregator_clone = self.clone();

        // Start a new debounce task.
        let handle = tokio::spawn(async move {
            // Wait for a short period to see if more messages arrive for this group.
            tokio::time::sleep(Duration::from_millis(2000)).await;

            // The timer has elapsed, so we can now process the group.
            if let Some((_, (messages, _))) = aggregator_clone.groups.remove(&media_group_id) {
                aggregator_clone.process_media_group(messages).await;
            }
        });

        // Store the new task's handle.
        entry.value_mut().1 = handle;
    }

    async fn process_media_group(&self, messages: Vec<Message>) {
        if messages.is_empty() {
            return;
        }

        // Find the message with caption (the command)
        let command_msg = messages.iter().find(|msg| msg.caption().is_some());
        
        if let Some(cmd_msg) = command_msg {
            let text = cmd_msg.caption().unwrap_or("");
            
            // Check if it's a reasoning command
            let is_reasoning_command = text.trim_start().starts_with("/r ") || text.trim() == "/r";
            
            // Check if it's a group admin command
            let is_group_command = text.trim_start().starts_with("/g ");
            
            // Only set group_id for /g commands (admin group commands)
            // Regular /c and /r commands should use None even in groups
            let group_id = if is_group_command && !cmd_msg.chat.is_private() {
                Some(cmd_msg.chat.id.to_string())
            } else {
                None
            };

            if is_reasoning_command {
                // Use reasoning handler for /r commands
                if let Err(e) = handle_reasoning_chat(
                    self.bot.clone(),
                    cmd_msg.clone(),
                    self.services.clone(),
                    self.ai.clone(),
                    self.db.clone(),
                    self.auth.clone(),
                    self.user_model_prefs.clone(),
                    text.to_string(),
                    self.group.clone(),
                ).await {
                    log::error!("Error handling reasoning chat with media group: {}", e);
                }
            } else {
                // Use regular chat handler for /c commands
                if let Err(e) = handle_chat(
                    self.bot.clone(),
                    cmd_msg.clone(),
                    self.services.clone(),
                    self.ai.clone(),
                    self.db.clone(),
                    self.auth.clone(),
                    self.user_model_prefs.clone(),
                    text.to_string(),
                    group_id,
                    self.group.clone(),
                ).await {
                    log::error!("Error handling chat with media group: {}", e);
                }
            }
        } else {
            log::warn!("Media group processed but no caption found with command");
        }
    }
}
