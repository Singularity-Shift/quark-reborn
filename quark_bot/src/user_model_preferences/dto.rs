use serde::{Deserialize, Serialize};
use open_ai_rust_responses_by_sshift::types::Effort;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelPreferences {
    // Regular chat models (for /c command)
    pub chat_model: ChatModel,
    pub temperature: f32,
    
    // Reasoning models (for /r command) 
    pub reasoning_model: ReasoningModel,
    pub effort: Effort,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChatModel {
    GPT4o,
    GPT41,
    GPT41Mini,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReasoningModel {
    O3,
    O4Mini,
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self {
            chat_model: ChatModel::GPT41Mini,
            temperature: 0.6,
            reasoning_model: ReasoningModel::O4Mini,
            effort: Effort::Low,
        }
    }
}

impl ChatModel {
    pub fn to_display_string(&self) -> &'static str {
        match self {
            ChatModel::GPT4o => "GPT-4o",
            ChatModel::GPT41 => "GPT-4.1",
            ChatModel::GPT41Mini => "GPT-4.1-Mini",
        }
    }

    pub fn to_openai_model(&self) -> open_ai_rust_responses_by_sshift::Model {
        match self {
            ChatModel::GPT4o => open_ai_rust_responses_by_sshift::Model::GPT4o,
            ChatModel::GPT41 => open_ai_rust_responses_by_sshift::Model::GPT41,
            ChatModel::GPT41Mini => open_ai_rust_responses_by_sshift::Model::GPT41Mini,
        }
    }
}

impl ReasoningModel {
    pub fn to_display_string(&self) -> &'static str {
        match self {
            ReasoningModel::O3 => "O3",
            ReasoningModel::O4Mini => "O4-Mini",
        }
    }

    pub fn to_openai_model(&self) -> open_ai_rust_responses_by_sshift::Model {
        match self {
            ReasoningModel::O3 => open_ai_rust_responses_by_sshift::Model::O3,
            ReasoningModel::O4Mini => open_ai_rust_responses_by_sshift::Model::O4Mini,
        }
    }
}

pub fn effort_to_display_string(effort: &Effort) -> &'static str {
    match effort {
        Effort::Low => "Low",
        Effort::Medium => "Medium",
        Effort::High => "High",
    }
} 