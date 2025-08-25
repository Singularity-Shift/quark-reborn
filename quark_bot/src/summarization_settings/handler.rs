use super::dto::{EffectiveSummarizationPrefs, SummarizationPrefs};
use super::helpers::{build_summarization_keyboard, format_summarization_status};
use anyhow::Result;
use sled::{Db, Tree};
use std::env;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

const TREE_NAME: &str = "summarization_prefs";

#[derive(Clone)]
pub struct SummarizationSettings {
    tree: Tree,
}

impl SummarizationSettings {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn get(&self, user_id: i64) -> SummarizationPrefs {
        let key = user_id.to_string();
        match self.tree.get(&key) {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
            _ => SummarizationPrefs::default(),
        }
    }

    pub fn set(&self, user_id: i64, prefs: &SummarizationPrefs) -> sled::Result<()> {
        let key = user_id.to_string();
        let bytes = serde_json::to_vec(prefs).unwrap();
        self.tree.insert(key, bytes)?;
        Ok(())
    }

    pub fn set_enabled(&self, user_id: i64, enabled: bool) -> sled::Result<()> {
        let mut prefs = self.get(user_id);
        prefs.summarizer_enabled = Some(enabled);
        self.set(user_id, &prefs)
    }

    pub fn set_token_limit(&self, user_id: i64, limit: u32) -> sled::Result<()> {
        let mut prefs = self.get(user_id);
        prefs.summarizer_token_limit = Some(limit);
        self.set(user_id, &prefs)
    }

    pub fn get_effective_prefs(&self, user_id: i64) -> EffectiveSummarizationPrefs {
        let prefs = self.get(user_id);
        
        // Resolve enabled: user pref -> env (both spellings) -> default true
        let enabled = prefs.summarizer_enabled.unwrap_or_else(|| {
            env::var("SUMMARIZER_ENABLED")
                .or_else(|_| env::var("SUMMARIZER"))
                .or_else(|_| env::var("summarizer_enabled"))
                .or_else(|_| env::var("summerizer"))
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true)
        });

        // Resolve token limit: user pref -> env (both spellings) -> default 12000
        let token_limit = prefs.summarizer_token_limit.unwrap_or_else(|| {
            env::var("CONVERSATION_TOKEN_LIMIT")
                .or_else(|_| env::var("conversation_token_limit"))
                .unwrap_or_else(|_| "12000".to_string())
                .parse::<u32>()
                .unwrap_or(12000)
        });

        EffectiveSummarizationPrefs {
            enabled,
            token_limit,
        }
    }
}

pub async fn handle_summarization_settings_callback(
    bot: Bot,
    query: CallbackQuery,
    summarization_settings: SummarizationSettings,
) -> Result<()> {
    let data = query.data.as_ref().unwrap();
    let user_id = query.from.id.0 as i64;

    if data == "open_summarization_settings" {
        show_summarization_settings_menu(bot, query, summarization_settings).await?;
    } else if data.starts_with("toggle_summarizer:") {
        let enabled = data == "toggle_summarizer:on";
        if let Err(e) = summarization_settings.set_enabled(user_id, enabled) {
            log::error!("Failed to set summarizer enabled for user {}: {}", user_id, e);
        }
        show_summarization_settings_menu(bot, query, summarization_settings).await?;
    } else if data.starts_with("set_summarizer_threshold:") {
        if let Some(threshold_str) = data.strip_prefix("set_summarizer_threshold:") {
            if let Ok(threshold) = threshold_str.parse::<u32>() {
                // Validate against allowed presets
                if [12000, 14000, 16000, 18000, 20000].contains(&threshold) {
                    if let Err(e) = summarization_settings.set_token_limit(user_id, threshold) {
                        log::error!("Failed to set token limit for user {}: {}", user_id, e);
                    }
                }
            }
        }
        show_summarization_settings_menu(bot, query, summarization_settings).await?;
    } else if data == "summarization_back_to_usersettings" {
        show_user_settings_menu(bot, query).await?;
    }

    Ok(())
}

async fn show_summarization_settings_menu(
    bot: Bot,
    query: CallbackQuery,
    summarization_settings: SummarizationSettings,
) -> Result<()> {
    let user_id = query.from.id.0 as i64;
    let effective_prefs = summarization_settings.get_effective_prefs(user_id);
    
    let status_text = format_summarization_status(&effective_prefs);
    let keyboard = build_summarization_keyboard();

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        bot.edit_message_text(message.chat.id, message.id, status_text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    bot.answer_callback_query(query.id)
        .text("Settings updated!")
        .await?;

    Ok(())
}

async fn show_user_settings_menu(bot: Bot, query: CallbackQuery) -> Result<()> {
    use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

    if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(message)) = &query.message {
        let kb = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::callback(
                "🧠 Select Model",
                "open_select_model",
            )],
            vec![InlineKeyboardButton::callback(
                "💳 Payment Settings",
                "open_payment_settings",
            )],
            vec![InlineKeyboardButton::callback(
                "📋 View My Settings",
                "open_my_settings",
            )],
            vec![InlineKeyboardButton::callback(
                "🧾 Summarization Settings",
                "open_summarization_settings",
            )],
            vec![InlineKeyboardButton::callback(
                "↩️ Close",
                "user_settings_close",
            )],
        ]);

        bot.edit_message_text(
            message.chat.id,
            message.id,
            "⚙️ <b>User Settings</b>\n\n• Manage your model, view current settings, and configure payment.\n\n💡 If no payment token is selected, the on-chain default will be used."
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(kb)
        .await?;
    }

    bot.answer_callback_query(query.id).await?;
    Ok(())
}
