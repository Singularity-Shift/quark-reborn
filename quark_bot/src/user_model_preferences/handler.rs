use super::dto::{ChatModel, ModelPreferences, VerbosityLevel};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json;
use sled::Db;

// Legacy struct for migration
#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyModelPreferences {
    pub chat_model: LegacyChatModel,
    pub temperature: f32,
    pub gpt5_mode: Option<LegacyGpt5Mode>,
    pub gpt5_effort: Option<open_ai_rust_responses_by_sshift::ReasoningEffort>,
    pub gpt5_verbosity: Option<open_ai_rust_responses_by_sshift::Verbosity>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum LegacyChatModel {
    GPT41,
    GPT5,
    GPT5Mini,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum LegacyGpt5Mode {
    Regular,
    Reasoning,
}

const TREE_NAME: &str = "user_model_preferences";

#[derive(Clone)]
pub struct UserModelPreferences {
    tree: sled::Tree,
}

impl UserModelPreferences {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn get_preferences(&self, username: &str) -> ModelPreferences {
        match self.tree.get(username) {
            Ok(Some(bytes)) => {
                // Try to deserialize as new format first
                if let Ok(prefs) = serde_json::from_slice::<ModelPreferences>(&bytes) {
                    prefs
                } else {
                    // Try to deserialize as legacy format and migrate
                    if let Ok(legacy_prefs) = serde_json::from_slice::<LegacyModelPreferences>(&bytes) {
                        let migrated_prefs = self.migrate_legacy_preferences(legacy_prefs);
                        // Save migrated preferences back to database
                        if let Ok(_) = self.set_preferences(username, &migrated_prefs) {
                            log::info!("Migrated legacy preferences for user: {}", username);
                        }
                        migrated_prefs
                    } else {
                        ModelPreferences::default()
                    }
                }
            }
            _ => ModelPreferences::default(),
        }
    }

    pub fn set_preferences(
        &self,
        username: &str,
        preferences: &ModelPreferences,
    ) -> sled::Result<()> {
        let bytes = serde_json::to_vec(preferences).unwrap();
        self.tree.insert(username, bytes)?;
        Ok(())
    }

    fn migrate_legacy_preferences(&self, legacy: LegacyModelPreferences) -> ModelPreferences {
        // Migrate chat model: only GPT41 was actually used, migrate to GPT5Mini with reasoning off
        let chat_model = match legacy.chat_model {
            LegacyChatModel::GPT41 => ChatModel::GPT5Mini,
            LegacyChatModel::GPT5 => ChatModel::GPT5,
            LegacyChatModel::GPT5Mini => ChatModel::GPT5Mini,
        };

        // Migrate reasoning: if mode was Reasoning, enable reasoning; otherwise disable
        let reasoning_enabled = if let Some(mode) = legacy.gpt5_mode {
            mode == LegacyGpt5Mode::Reasoning
        } else {
            false
        };

        // Migrate verbosity: Low -> Normal, Medium/High -> Chatty
        let verbosity = if let Some(legacy_verbosity) = legacy.gpt5_verbosity {
            match legacy_verbosity {
                open_ai_rust_responses_by_sshift::Verbosity::Low => VerbosityLevel::Normal,
                open_ai_rust_responses_by_sshift::Verbosity::Medium => VerbosityLevel::Chatty,
                open_ai_rust_responses_by_sshift::Verbosity::High => VerbosityLevel::Chatty,
            }
        } else {
            VerbosityLevel::Normal
        };

        ModelPreferences {
            chat_model,
            reasoning_enabled,
            verbosity,
        }
    }
}



// Removed legacy O-series effort keyboard

/// Initialize default preferences for a new user
pub async fn initialize_user_preferences(
    username: &str,
    user_model_prefs: &UserModelPreferences,
) -> Result<()> {
    // Only set if user doesn't already have preferences
    let existing = user_model_prefs.tree.get(username)?;
    if existing.is_none() {
        let default_prefs = ModelPreferences::default();
        user_model_prefs.set_preferences(username, &default_prefs)?;
        log::info!(
            "Initialized default model preferences for user: {}",
            username
        );
    }

    Ok(())
}
