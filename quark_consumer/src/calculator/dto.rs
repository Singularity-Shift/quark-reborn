use serde::Deserialize;
use std::fmt;

#[derive(Debug, Deserialize)]
pub struct Price {
    pub model: Vec<ModelEntry>,
    pub tool: Vec<ToolEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ModelEntry {
    pub name: ModelName,
    pub price: f64,
}

#[derive(Debug, Deserialize)]
pub struct ToolEntry {
    pub name: ToolName,
    pub price: f64,
}

#[derive(Debug, Deserialize)]
pub enum ModelName {
    O3,
    O4Mini,
    GPT4o,
    GPT5,
    GPT41,
    GPT41Mini,
    GPT5Mini,
    GPT5Nano,
}

impl fmt::Display for ModelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelName::O3 => write!(f, "o3"),
            ModelName::O4Mini => write!(f, "o4-mini"),
            ModelName::GPT4o => write!(f, "gpt-4o"),
            ModelName::GPT5 => write!(f, "gpt-5"),
            ModelName::GPT41 => write!(f, "gpt-4.1"),
            ModelName::GPT41Mini => write!(f, "gpt-4.1-mini"),
            ModelName::GPT5Mini => write!(f, "gpt-5-mini"),
            ModelName::GPT5Nano => write!(f, "gpt-5-nano"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum ToolName {
    FileSearch,
    ImageGeneration,
    WebSearchPreview,
}

impl fmt::Display for ToolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolName::FileSearch => write!(f, "FileSearch"),
            ToolName::ImageGeneration => write!(f, "ImageGeneration"),
            ToolName::WebSearchPreview => write!(f, "WebSearchPreview"),
        }
    }
}
