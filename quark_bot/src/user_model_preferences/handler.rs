use super::dto::{
    ChatModel, Gpt5Mode, ModelPreferences, gpt5_effort_to_display_string, gpt5_mode_to_display_string,
    verbosity_to_display_string,
};
use crate::dependencies::BotDependencies;
use anyhow::Result;
use serde_json;
use sled::Db;
use teloxide::prelude::*;
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

    // Removed legacy set_reasoning_preferences
}

pub async fn handle_select_model(bot: Bot, msg: Message) -> Result<()> {
    let user = msg.from.as_ref();
    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ Unable to verify user.")
            .await?;
        return Ok(());
    }

    let username = user.unwrap().username.as_ref();
    if username.is_none() {
        bot.send_message(
            msg.chat.id,
            "❌ Username not found, required for this feature",
        )
        .await?;
        return Ok(());
    }

    // Step 1: Show chat model selection
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "GPT-5 (💰 Expensive)",
            "select_chat_model:GPT5",
        )],
        vec![InlineKeyboardButton::callback(
            "GPT-4.1 (💸 Cheap)",
            "select_chat_model:GPT41",
        )],
        vec![InlineKeyboardButton::callback(
            "GPT-5-Mini (💵 Cheapest)",
            "select_chat_model:GPT5Mini",
        )],
    ]);

    bot.send_message(msg.chat.id, "🤖 <b>Select your chat model:</b>\n\nChoose which model to use for regular chat commands (/c):")
        .reply_markup(keyboard)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

// Removed: handle_select_reasoning_model (unified in /selectmodel)

pub async fn handle_my_settings(bot: Bot, msg: Message, bot_deps: BotDependencies) -> Result<()> {
    let user = msg.from.as_ref();
    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ Unable to verify user.")
            .await?;
        return Ok(());
    }

    let username = user.unwrap().username.as_ref();
    if username.is_none() {
        bot.send_message(
            msg.chat.id,
            "❌ Username not found, required for this feature",
        )
        .await?;
        return Ok(());
    }

    // Get user's current preferences
    let preferences = bot_deps.user_model_prefs.get_preferences(username.unwrap());

    // Format the settings message
    // Build conditional blocks
    let temperature_block = match preferences.chat_model {
        ChatModel::GPT41 | ChatModel::GPT41Mini | ChatModel::GPT4o => {
            format!("        🌡️ Temperature: {}\n\n", preferences.temperature)
        }
        _ => "\n".to_string(),
    };

    let gpt5_block = if matches!(preferences.chat_model, ChatModel::GPT5 | ChatModel::GPT5Mini) {
        let mode = preferences
            .gpt5_mode
            .as_ref()
            .map(gpt5_mode_to_display_string)
            .unwrap_or("Regular");
        let verbosity = preferences
            .gpt5_verbosity
            .as_ref()
            .map(verbosity_to_display_string)
            .unwrap_or("Medium");
        let effort_line = if preferences.gpt5_mode == Some(Gpt5Mode::Reasoning) {
            let eff = preferences
                .gpt5_effort
                .as_ref()
                .map(gpt5_effort_to_display_string)
                .unwrap_or("Medium");
            format!("        ⚡ Reasoning Effort: {}\n", eff)
        } else {
            String::new()
        };
        format!(
            "        🧩 Mode: {}\n        🗣️ Verbosity: {}\n{}",
            mode, verbosity, effort_line
        )
    } else {
        String::new()
    };

    let settings_text = format!(
        "⚙️ <b>Your Current Model Settings</b>\n\n\
        💬 <b>Chat Model (for /c commands):</b>\n\
        🤖 Model: {}\n\
{}{}\
        💡 Use /selectmodel to change these settings.",
        preferences.chat_model.to_display_string(),
        temperature_block,
        if gpt5_block.is_empty() { String::new() } else { format!("{}\n", gpt5_block) }
    );

    bot.send_message(msg.chat.id, settings_text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
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
