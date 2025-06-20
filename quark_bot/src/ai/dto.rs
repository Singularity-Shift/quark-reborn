use serde::Deserialize;

/// Represents the AI's response, which can include text and/or an image.
#[derive(Debug)]
pub struct AIResponse {
    pub text: String,
    pub image_data: Option<Vec<u8>>,
}

impl From<(String, Option<Vec<u8>>)> for AIResponse {
    fn from(value: (String, Option<Vec<u8>>)) -> Self {
        let (text, image_data) = value;

        Self { text, image_data }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetBalanceResponse {
    pub balance: u64,
}
