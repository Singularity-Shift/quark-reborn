use crate::credentials::handler::Auth;
use crate::group::handler::Group;
use crate::user_model_preferences::handler::UserModelPreferences;
use crate::{ai::handler::AI, services::handler::Services};
use dashmap::DashMap;
use sled::Db;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use crate::bot::handler::{handle_chat, handle_reasoning_chat};

/// Holds an in-flight `/c` command and any trailing photo-only messages
struct PendingCmd {
    first_msg: Message,
    extra_photos: Vec<Message>,
    timer: Option<JoinHandle<()>>, // debounce task
}

pub struct CommandImageCollector {
    // Keyed by (chat_id, user_id)
    pendings: DashMap<(ChatId, i64), PendingCmd>,
    bot: Bot,
    db: Db,
    service: Services,
    user_model_prefs: UserModelPreferences,
    debounce_ms: u64,
}

impl CommandImageCollector {
    pub fn new(
        bot: Bot,
        db: Db,
        service: Services,
        user_model_prefs: UserModelPreferences,
    ) -> Self {
        Self {
            pendings: DashMap::new(),
            bot,
            db,
            service,
            user_model_prefs,
            debounce_ms: 1000, // 1 second default
        }
    }

    /// Entry point for any incoming message that is a `/c` command
    pub async fn add_command(
        self: Arc<Self>,
        ai: AI,
        msg: Message,
        auth: Auth,
        group_id: Option<String>,
        group: Group,
    ) {
        // Cancel any existing pending command for this user/chat
        let key = (
            msg.chat.id,
            msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0),
        );
        if let Some(mut pending) = self.pendings.remove(&key) {
            if let Some(handle) = pending.1.timer.take() {
                handle.abort();
            }
        }

        // Insert new pending
        self.pendings.insert(
            key,
            PendingCmd {
                first_msg: msg.clone(),
                extra_photos: Vec::new(),
                timer: None,
            },
        );

        self.reset_timer(key, ai, msg, auth, group_id, group);
    }

    /// Entry point for photo-only messages that may belong to a pending command
    pub async fn try_attach_photo(
        self: Arc<Self>,
        msg: Message,
        ai: AI,
        auth: Auth,
        group_id: Option<String>,
        group: Group,
    ) {
        let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
        let key = (msg.chat.id, user_id);
        if let Some(mut entry) = self.pendings.get_mut(&key) {
            // Attach photo
            entry.extra_photos.push(msg.clone());
            // restart debounce
            self.reset_timer(key, ai, msg, auth, group_id, group);
        }
    }

    fn reset_timer(
        self: &Arc<Self>,
        key: (ChatId, i64),
        ai: AI,
        msg: Message,
        auth: Auth,
        group_id: Option<String>,
        group: Group,
    ) {
        // Abort any existing timer first
        if let Some(mut entry) = self.pendings.get_mut(&key) {
            if let Some(handle) = entry.timer.take() {
                handle.abort();
            }
        }

        let collector = Arc::clone(self);
        let handle = tokio::spawn(async move {
            sleep(Duration::from_millis(collector.debounce_ms)).await;
            collector
                .finalize(key, ai, msg, auth, group_id, group)
                .await;
        });

        if let Some(mut entry) = self.pendings.get_mut(&key) {
            entry.timer = Some(handle);
        }
    }

    async fn finalize(
        self: &Arc<Self>,
        key: (ChatId, i64),
        ai: AI,
        msg: Message,
        auth: Auth,
        group_id: Option<String>,
        group: Group,
    ) {
        if let Some((_k, pending)) = self.pendings.remove(&key) {
            let mut all_msgs = Vec::new();
            all_msgs.push(pending.first_msg.clone());
            all_msgs.extend(pending.extra_photos);
            let text = msg.text().or_else(|| msg.caption()).unwrap_or("");

            // Check if the original command was /r (reasoning)
            let is_reasoning_command = pending
                .first_msg
                .text()
                .map(|t| t.trim_start().starts_with("/r ") || t.trim() == "/r")
                .unwrap_or(false);

            // Decide whether to call single or grouped handler
            if all_msgs.len() == 1 {
                // Single message path
                let msg = all_msgs.pop().unwrap();

                if is_reasoning_command {
                    // Use reasoning handler for /r commands
                    if let Err(e) = handle_reasoning_chat(
                        self.bot.clone(),
                        msg,
                        self.service.clone(),
                        ai,
                        self.db.clone(),
                        auth,
                        self.user_model_prefs.clone(),
                        text.to_string(),
                        group,
                    )
                    .await
                    {
                        log::error!("Error handling reasoning chat: {}", e);
                    }
                } else {
                    // Use regular chat handler for /c commands
                    if let Err(e) = handle_chat(
                        self.bot.clone(),
                        msg,
                        self.service.clone(),
                        ai,
                        self.db.clone(),
                        auth,
                        self.user_model_prefs.clone(),
                        text.to_string(),
                        group_id,
                        group,
                    )
                    .await
                    {
                        log::error!("Error handling chat: {}", e);
                    }
                }
            } else {
                log::error!("Error handling grouped chat (ungrouped images)");
            }
        }
    }
}
