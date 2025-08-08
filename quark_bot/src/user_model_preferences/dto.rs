use serde::{Deserialize, Serialize};
use open_ai_rust_responses_by_sshift::{ReasoningEffort, Verbosity};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPreferences {
    // Regular chat models (for /c command)
    pub chat_model: ChatModel,
    pub temperature: f32,

    // GPT-5 specific preferences (unified chat flow)
    pub gpt5_mode: Option<Gpt5Mode>,
    pub gpt5_effort: Option<ReasoningEffort>,
    pub gpt5_verbosity: Option<Verbosity>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChatModel {
    GPT4o,
    GPT41,
    GPT41Mini,
    GPT5,
    GPT5Mini,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Gpt5Mode {
    Regular,
    Reasoning,
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            chat_model: ChatModel::GPT5Mini,
            temperature: 0.6,
            // Defaults for GPT-5 unified flow
            gpt5_mode: Some(Gpt5Mode::Regular),
            gpt5_effort: None,
            gpt5_verbosity: Some(Verbosity::Medium),
        }
    }
}

impl ChatModel {
    pub fn to_display_string(&self) -> &'static str {
        match self {
            ChatModel::GPT4o => "GPT-4o",
            ChatModel::GPT41 => "GPT-4.1",
            ChatModel::GPT41Mini => "GPT-4.1-Mini",
            ChatModel::GPT5 => "GPT-5",
            ChatModel::GPT5Mini => "GPT-5-Mini",
        }
    }

    pub fn to_openai_model(&self) -> open_ai_rust_responses_by_sshift::Model {
        match self {
            ChatModel::GPT4o => open_ai_rust_responses_by_sshift::Model::GPT4o,
            ChatModel::GPT41 => open_ai_rust_responses_by_sshift::Model::GPT41,
            ChatModel::GPT41Mini => open_ai_rust_responses_by_sshift::Model::GPT41Mini,
            ChatModel::GPT5 => open_ai_rust_responses_by_sshift::Model::GPT5,
            ChatModel::GPT5Mini => open_ai_rust_responses_by_sshift::Model::GPT5Mini,
        }
    }
}

// Removed legacy O-series reasoning model and effort fields

pub fn gpt5_effort_to_display_string(effort: &ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::Minimal => "Minimal",
        ReasoningEffort::Medium => "Medium",
        ReasoningEffort::High => "High",
    }
}

pub fn gpt5_mode_to_display_string(mode: &Gpt5Mode) -> &'static str {
    match mode {
        Gpt5Mode::Regular => "Regular",
        Gpt5Mode::Reasoning => "Reasoning",
    }
}

pub fn verbosity_to_display_string(v: &Verbosity) -> &'static str {
    match v {
        Verbosity::Low => "Low",
        Verbosity::Medium => "Medium",
        Verbosity::High => "High",
    }
}