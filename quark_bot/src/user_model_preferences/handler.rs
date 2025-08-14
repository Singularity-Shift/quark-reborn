use super::dto::{ChatModel, ModelPreferences};
use anyhow::Result;
use serde_json;
use sled::Db;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

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
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
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

    pub fn set_chat_preferences(
        &self,
        username: &str,
        model: ChatModel,
        temperature: f32,
    ) -> sled::Result<()> {
        let mut prefs = self.get_preferences(username);
        prefs.chat_model = model;
        prefs.temperature = temperature;
        self.set_preferences(username, &prefs)
    }
}

pub fn get_temperature_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("0.3", "set_temperature:0.3"),
            InlineKeyboardButton::callback("0.6", "set_temperature:0.6"),
        ],
        vec![
            InlineKeyboardButton::callback("0.8", "set_temperature:0.8"),
            InlineKeyboardButton::callback("1.0", "set_temperature:1.0"),
        ],
    ])
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
