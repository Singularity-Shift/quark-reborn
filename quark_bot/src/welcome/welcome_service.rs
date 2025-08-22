use anyhow::Result;
use sled::Tree;
use teloxide::{
    prelude::*,
    types::{ChatId, ChatPermissions, UserId, InlineKeyboardButton, InlineKeyboardMarkup},
};
use tokio::time::{sleep, Duration};

use crate::welcome::{
    dto::{PendingVerification, WelcomeSettings, WelcomeStats},
    helpers::{get_custom_welcome_message, get_verification_expiry_time, is_verification_expired},
};

#[derive(Clone)]
pub struct WelcomeService {
    settings_db: Tree,
    verifications_db: Tree,
    stats_db: Tree,
}

impl WelcomeService {
    pub fn new(db: sled::Db) -> Self {
        let settings_db = db.open_tree("welcome_settings").expect("Failed to open welcome settings tree");
        let verifications_db = db.open_tree("welcome_verifications").expect("Failed to open welcome verifications tree");
        let stats_db = db.open_tree("welcome_stats").expect("Failed to open welcome stats tree");
        
        Self {
            settings_db,
            verifications_db,
            stats_db,
        }
    }

    pub fn get_settings(&self, chat_id: ChatId) -> WelcomeSettings {
        let key = chat_id.0.to_string();
        
        if let Ok(Some(bytes)) = self.settings_db.get(key.as_bytes()) {
            if let Ok(settings) = serde_json::from_slice(&bytes) {
                return settings;
            }
        }
        
        WelcomeSettings::default()
    }

    pub fn save_settings(&self, chat_id: ChatId, settings: WelcomeSettings) -> Result<()> {
        let key = chat_id.0.to_string();
        let bytes = serde_json::to_vec(&settings)?;
        
        self.settings_db.insert(key.as_bytes(), bytes)?;
        Ok(())
    }

    pub fn is_enabled(&self, chat_id: ChatId) -> bool {
        self.get_settings(chat_id).enabled
    }

    pub async fn handle_new_member(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        user_id: UserId,
        username: Option<String>,
        first_name: String,
    ) -> Result<()> {
        if !self.is_enabled(chat_id) {
            return Ok(());
        }

        let settings = self.get_settings(chat_id);
        
        // Mute the new member immediately
        let restricted_permissions = ChatPermissions::empty();
        bot.restrict_chat_member(chat_id, user_id, restricted_permissions).await?;
        
        // Get chat title for welcome message
        let chat = bot.get_chat(chat_id).await?;
        let group_name = chat.title().unwrap_or("this group").to_string();
        
        // Create verification button
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback(
                "✅ Prove You're Human",
                format!("welcome_verify:{}:{}", chat_id.0, user_id.0),
            ),
        ]]);
        
        // Send welcome message with verification button
        let welcome_text = get_custom_welcome_message(&settings, &first_name, &group_name);
        let message = bot.send_message(chat_id, welcome_text)
            .reply_markup(keyboard)
            .await?;
        
        // Store pending verification
        let verification = PendingVerification {
            user_id,
            username,
            first_name,
            chat_id,
            joined_at: chrono::Utc::now().timestamp(),
            expires_at: get_verification_expiry_time(settings.verification_timeout),
            verification_message_id: message.id.0,
        };
        
        let key = format!("{}:{}", chat_id.0, user_id.0);
        let bytes = serde_json::to_vec(&verification)?;
        self.verifications_db.insert(key.as_bytes(), bytes)?;
        
        // Schedule cleanup task
        let bot_clone = bot.clone();
        let service_clone = self.clone();
        let chat_id_clone = chat_id;
        let user_id_clone = user_id;
        
        tokio::spawn(async move {
            sleep(Duration::from_secs(settings.verification_timeout)).await;
            if let Err(e) = service_clone.cleanup_expired_verification(
                &bot_clone,
                chat_id_clone,
                user_id_clone,
            ).await {
                log::error!("Failed to cleanup expired verification: {}", e);
            }
        });
        
        Ok(())
    }

    pub async fn handle_verification(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        user_id: UserId,
    ) -> Result<()> {
        log::info!("Starting verification for user {} in chat {}", user_id.0, chat_id.0);
        let key = format!("{}:{}", chat_id.0, user_id.0);
        
        // Get verification record
        let verification = if let Ok(Some(bytes)) = self.verifications_db.get(key.as_bytes()) {
            if let Ok(verification) = serde_json::from_slice::<PendingVerification>(&bytes) {
                log::info!("Found verification record for user {}: expires at {}", user_id.0, verification.expires_at);
                verification
            } else {
                log::error!("Failed to deserialize verification data for user {}", user_id.0);
                return Err(anyhow::anyhow!("Invalid verification data"));
            }
        } else {
            log::error!("No verification record found for user {} in chat {}", user_id.0, chat_id.0);
            return Err(anyhow::anyhow!("Verification not found"));
        };
        
        // Check if verification is expired
        if is_verification_expired(verification.expires_at) {
            log::warn!("Verification expired for user {} in chat {}", user_id.0, chat_id.0);
            return Err(anyhow::anyhow!("Verification expired"));
        }
        
        log::info!("Attempting to unmute user {} in chat {}", user_id.0, chat_id.0);
        
        // Check if bot has admin permissions (only once)
        let bot_info = bot.get_me().await?;
        let chat_member = bot.get_chat_member(chat_id, bot_info.id).await?;
        
        if !chat_member.is_privileged() {
            log::error!("Bot is not an admin in chat {}, cannot perform verification actions", chat_id.0);
            return Err(anyhow::anyhow!("Bot is not an admin in this chat"));
        }
        
        // Unmute the user
        let full_permissions = ChatPermissions::all();
        match bot.restrict_chat_member(chat_id, user_id, full_permissions).await {
            Ok(_) => log::info!("Successfully unmuted user {} in chat {}", user_id.0, chat_id.0),
            Err(e) => {
                log::error!("Failed to unmute user {} in chat {}: {}", user_id.0, chat_id.0, e);
                return Err(anyhow::anyhow!("Failed to unmute user: {}", e));
            }
        }
        
        log::info!("Updating verification message for user {} in chat {}", user_id.0, chat_id.0);
        
        // Update verification message
        let success_text = format!(
            "✅ Welcome to the group, {}! You've been verified and can now participate.",
            verification.first_name
        );
        
        match bot.edit_message_text(chat_id, teloxide::types::MessageId(verification.verification_message_id), success_text).await {
            Ok(_) => log::info!("Successfully updated verification message for user {} in chat {}", user_id.0, chat_id.0),
            Err(e) => {
                log::error!("Failed to update verification message for user {} in chat {}: {}", user_id.0, chat_id.0, e);
                return Err(anyhow::anyhow!("Failed to update message: {}", e));
            }
        }
        
        log::info!("Removing verification record for user {} in chat {}", user_id.0, chat_id.0);
        
        // Remove verification record
        if let Err(e) = self.verifications_db.remove(key.as_bytes()) {
            log::error!("Failed to remove verification record for user {} in chat {}: {}", user_id.0, chat_id.0, e);
            return Err(anyhow::anyhow!("Failed to remove verification record: {}", e));
        }
        
        log::info!("Updating statistics for user {} in chat {}", user_id.0, chat_id.0);
        
        // Update statistics
        if let Err(e) = self.update_stats(chat_id, true) {
            log::error!("Failed to update stats for user {} in chat {}: {}", user_id.0, chat_id.0, e);
            return Err(anyhow::anyhow!("Failed to update stats: {}", e));
        }
        
        log::info!("Verification completed successfully for user {} in chat {}", user_id.0, chat_id.0);
        Ok(())
    }

    pub async fn cleanup_expired_verification(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        user_id: UserId,
    ) -> Result<()> {
        let key = format!("{}:{}", chat_id.0, user_id.0);
        
        // Get verification record
        let verification = if let Ok(Some(bytes)) = self.verifications_db.get(key.as_bytes()) {
            if let Ok(verification) = serde_json::from_slice::<PendingVerification>(&bytes) {
                verification
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        };
        
        // Check if verification is expired
        if !is_verification_expired(verification.expires_at) {
            return Ok(());
        }
        
        // Remove user from group
        if let Err(e) = bot.ban_chat_member(chat_id, user_id).await {
            log::error!("Failed to ban expired verification user: {}", e);
        }
        
        // Update verification message
        let expired_text = format!(
            "⏰ Verification expired for {}. User has been removed from the group.",
            verification.first_name
        );
        
        if let Err(e) = bot.edit_message_text(chat_id, teloxide::types::MessageId(verification.verification_message_id), expired_text).await {
            log::error!("Failed to update expired verification message: {}", e);
        }
        
        // Remove verification record
        self.verifications_db.remove(key.as_bytes())?;
        
        // Update statistics
        self.update_stats(chat_id, false)?;
        
        Ok(())
    }

    fn update_stats(&self, chat_id: ChatId, success: bool) -> Result<()> {
        let key = format!("{}_stats", chat_id.0);
        
        let mut stats = if let Ok(Some(bytes)) = self.stats_db.get(key.as_bytes()) {
            if let Ok(stats) = serde_json::from_slice::<WelcomeStats>(&bytes) {
                stats
            } else {
                WelcomeStats::default()
            }
        } else {
            WelcomeStats::default()
        };
        
        stats.total_verifications += 1;
        if success {
            stats.successful_verifications += 1;
        } else {
            stats.failed_verifications += 1;
        }
        
        stats.success_rate = (stats.successful_verifications as f64 / stats.total_verifications as f64) * 100.0;
        stats.last_verification = Some(chrono::Utc::now().timestamp());
        
        let bytes = serde_json::to_vec(&stats)?;
        self.stats_db.insert(key.as_bytes(), bytes)?;
        
        Ok(())
    }

    pub fn get_stats(&self, chat_id: ChatId) -> WelcomeStats {
        let key = format!("{}_stats", chat_id.0);
        
        if let Ok(Some(bytes)) = self.stats_db.get(key.as_bytes()) {
            if let Ok(stats) = serde_json::from_slice::<WelcomeStats>(&bytes) {
                return stats;
            }
        }
        
        WelcomeStats::default()
    }

    pub fn cleanup_all_expired_verifications(&self) -> Result<()> {
        let mut expired_keys = Vec::new();
        
        for result in self.verifications_db.iter() {
            if let Ok((key, value)) = result {
                if let Ok(verification) = serde_json::from_slice::<PendingVerification>(&value) {
                    if is_verification_expired(verification.expires_at) {
                        expired_keys.push(key);
                    }
                }
            }
        }
        
        for key in expired_keys {
            self.verifications_db.remove(&key)?;
        }
        
        Ok(())
    }

    pub fn reset_stats(&self, chat_id: ChatId) -> Result<()> {
        let key = format!("{}_stats", chat_id.0);
        self.stats_db.remove(key.as_bytes())?;
        Ok(())
    }

    pub async fn store_input_state(&self, key: String, chat_id: ChatId) -> Result<()> {
        let input_state = serde_json::json!({
            "chat_id": chat_id.0,
            "timestamp": chrono::Utc::now().timestamp(),
            "type": "custom_message_input"
        });
        let bytes = serde_json::to_vec(&input_state)?;
        self.settings_db.insert(key.as_bytes(), bytes)?;
        Ok(())
    }

    pub fn get_input_state(&self, key: &str) -> Option<serde_json::Value> {
        if let Ok(Some(bytes)) = self.settings_db.get(key.as_bytes()) {
            if let Ok(value) = serde_json::from_slice(&bytes) {
                return Some(value);
            }
        }
        None
    }

    pub fn clear_input_state(&self, key: &str) -> Result<()> {
        self.settings_db.remove(key.as_bytes())?;
        Ok(())
    }

    pub fn cleanup_expired_input_states(&self) -> Result<()> {
        let mut expired_keys = Vec::new();
        let now = chrono::Utc::now().timestamp();
        
        for result in self.settings_db.iter() {
            if let Ok((key, value)) = result {
                let key_str = String::from_utf8_lossy(&key);
                if key_str.starts_with("welcome_custom_msg_input:") {
                    if let Ok(state) = serde_json::from_slice::<serde_json::Value>(&value) {
                        if let Some(timestamp) = state["timestamp"].as_i64() {
                            // Clear input states older than 10 minutes
                            if now - timestamp > 600 {
                                expired_keys.push(key);
                            }
                        }
                    }
                }
            }
        }
        
        for key in expired_keys {
            self.settings_db.remove(&key)?;
        }
        
        Ok(())
    }
}
