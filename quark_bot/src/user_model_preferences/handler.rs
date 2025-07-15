use super::dto::{ChatModel, ModelPreferences, ReasoningModel};
use anyhow::Result;
use open_ai_rust_responses_by_sshift::types::Effort;
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

    pub fn set_reasoning_preferences(
        &self,
        username: &str,
        model: ReasoningModel,
        effort: Effort,
    ) -> sled::Result<()> {
        let mut prefs = self.get_preferences(username);
        prefs.reasoning_model = model;
        prefs.effort = effort;
        self.set_preferences(username, &prefs)
    }
}

pub async fn handle_select_model(
    bot: Bot,
    msg: Message,
    _user_model_prefs: UserModelPreferences,
) -> Result<()> {
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
            "GPT-4o (💰 Expensive)",
            "select_chat_model:GPT4o",
        )],
        vec![InlineKeyboardButton::callback(
            "GPT-4.1 (💸 Cheap)",
            "select_chat_model:GPT41",
        )],
        vec![InlineKeyboardButton::callback(
            "GPT-4.1-Mini (💵 Cheapest)",
            "select_chat_model:GPT41Mini",
        )],
    ]);

    bot.send_message(msg.chat.id, "🤖 **Select your chat model:**\n\nChoose which model to use for regular chat commands (/c):")
        .reply_markup(keyboard)
        .parse_mode(teloxide::types::ParseMode::Markdown)
        .await?;

    Ok(())
}

pub async fn handle_select_reasoning_model(
    bot: Bot,
    msg: Message,
    _user_model_prefs: UserModelPreferences,
) -> Result<()> {
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

    // Step 1: Show reasoning model selection
    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "O3 (💰 Expensive)",
            "select_reasoning_model:O3",
        )],
        vec![InlineKeyboardButton::callback(
            "O4-Mini (💵 Cheapest)",
            "select_reasoning_model:O4Mini",
        )],
    ]);

    bot.send_message(msg.chat.id, "🧠 **Select your reasoning model:**\n\nChoose which model to use for reasoning commands (/r):")
        .reply_markup(keyboard)
        .parse_mode(teloxide::types::ParseMode::Markdown)
        .await?;

    Ok(())
}

pub async fn handle_my_settings(
    bot: Bot,
    msg: Message,
    user_model_prefs: UserModelPreferences,
) -> Result<()> {
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
    let preferences = user_model_prefs.get_preferences(username.unwrap());

    // Format the settings message
    let settings_text = format!(
        "⚙️ **Your Current Model Settings**\n\n\
        💬 **Chat Model (for /c commands):**\n\
        🤖 Model: {}\n\
        🌡️ Temperature: {}\n\n\
        🧠 **Reasoning Model (for /r commands):**\n\
        🤖 Model: {}\n\
        ⚡ Effort: {}\n\n\
        💡 Use /selectmodel or /selectreasoningmodel to change these settings.",
        preferences.chat_model.to_display_string(),
        preferences.temperature,
        preferences.reasoning_model.to_display_string(),
        super::dto::effort_to_display_string(&preferences.effort)
    );

    bot.send_message(msg.chat.id, settings_text)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
            InlineKeyboardButton::callback("1.0", "set_temperature:1.0"),
            InlineKeyboardButton::callback("1.5", "set_temperature:1.5"),
        ],
    ])
}

pub fn get_effort_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "Low (💸 Cheap)",
            "set_effort:Low",
        )],
        vec![InlineKeyboardButton::callback(
            "Medium (💰 Standard)",
            "set_effort:Medium",
        )],
        vec![InlineKeyboardButton::callback(
            "High (💸💸 Very Expensive)",
            "set_effort:High",
        )],
    ])
}

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
