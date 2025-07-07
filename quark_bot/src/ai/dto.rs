use open_ai_rust_responses_by_sshift::{FunctionCallInfo, Model};

/// Represents the AI's response, which can include text and/or an image.
#[derive(Debug)]
pub struct AIResponse {
    pub text: String,
    pub model: Model,
    pub image_data: Option<Vec<u8>>,
    pub tool_calls: Option<Vec<FunctionCallInfo>>,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl
    From<(
        String,
        Model,
        Option<Vec<u8>>,
        Option<Vec<FunctionCallInfo>>,
        u32,
        u32,
        u32,
    )> for AIResponse
{
    fn from(
        value: (
            String,
            Model,
            Option<Vec<u8>>,
            Option<Vec<FunctionCallInfo>>,
            u32,
            u32,
            u32,
        ),
    ) -> Self {
        let (text, model, image_data, tool_calls, prompt_tokens, output_tokens, total_tokens) =
            value;

        Self {
            text,
            model,
            image_data,
            tool_calls,
            prompt_tokens,
            output_tokens,
            total_tokens,
        }
    }
}

// Backward compatibility constructor
impl
    From<(
        String,
        Model,
        Option<Vec<u8>>,
        Option<Vec<FunctionCallInfo>>,
    )> for AIResponse
{
    fn from(
        value: (
            String,
            Model,
            Option<Vec<u8>>,
            Option<Vec<FunctionCallInfo>>,
        ),
    ) -> Self {
        let (text, model, image_data, tool_calls) = value;

        Self {
            text,
            model,
            image_data,
            tool_calls,
            prompt_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
        }
    }
}
