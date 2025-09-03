use open_ai_rust_responses_by_sshift::Verbosity;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPreferences {
    // Regular chat models (for /c command)
    pub chat_model: ChatModel,

    // GPT-5 specific preferences (unified chat flow)
    pub reasoning_enabled: bool,
    pub verbosity: VerbosityLevel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChatModel {
    GPT5,
    GPT5Mini,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum VerbosityLevel {
    Normal,
    Chatty,
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            chat_model: ChatModel::GPT5Mini,
            reasoning_enabled: false,
            verbosity: VerbosityLevel::Normal,
        }
    }
}

impl ChatModel {
    pub fn to_display_string(&self) -> &'static str {
        match self {
            ChatModel::GPT5 => "GPT-5",
            ChatModel::GPT5Mini => "GPT-5-Mini",
        }
    }

    pub fn to_openai_model(&self) -> open_ai_rust_responses_by_sshift::Model {
        match self {
            ChatModel::GPT5 => open_ai_rust_responses_by_sshift::Model::GPT5,
            ChatModel::GPT5Mini => open_ai_rust_responses_by_sshift::Model::GPT5Mini,
        }
    }
}

impl VerbosityLevel {
    pub fn to_display_string(&self) -> &'static str {
        match self {
            VerbosityLevel::Normal => "Normal",
            VerbosityLevel::Chatty => "Chatty",
        }
    }

    pub fn to_openai_verbosity(&self) -> Verbosity {
        match self {
            VerbosityLevel::Normal => Verbosity::Low,
            VerbosityLevel::Chatty => Verbosity::Medium,
        }
    }
}
