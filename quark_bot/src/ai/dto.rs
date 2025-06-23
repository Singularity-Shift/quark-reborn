use open_ai_rust_responses_by_sshift::FunctionCallInfo;

/// Represents the AI's response, which can include text and/or an image.
#[derive(Debug)]
pub struct AIResponse {
    pub text: String,
    pub image_data: Option<Vec<u8>>,
    pub tool_calls: Option<Vec<FunctionCallInfo>>,
}

impl From<(String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>)> for AIResponse {
    fn from(value: (String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>)) -> Self {
        let (text, image_data, tool_calls) = value;

        Self {
            text,
            image_data,
            tool_calls,
        }
    }
}
