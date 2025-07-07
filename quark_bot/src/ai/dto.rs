use open_ai_rust_responses_by_sshift::FunctionCallInfo;
use open_ai_rust_responses_by_sshift::types::{Response, ResponseItem};

/// Enhanced Usage struct with both token counts and tool usage
#[derive(Debug, Clone)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub web_search: Option<u32>,
    pub file_search: Option<u32>,
    pub image_generation: Option<u32>,
    pub code_interpreter: Option<u32>,
}

impl Usage {
    pub fn new(input_tokens: u32, output_tokens: u32, total_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens,
            web_search: None,
            file_search: None,
            image_generation: None,
            code_interpreter: None,
        }
    }

    pub fn with_tool_usage(mut self, web_search: u32, file_search: u32, image_generation: u32, code_interpreter: u32) -> Self {
        self.web_search = if web_search > 0 { Some(web_search) } else { None };
        self.file_search = if file_search > 0 { Some(file_search) } else { None };
        self.image_generation = if image_generation > 0 { Some(image_generation) } else { None };
        self.code_interpreter = if code_interpreter > 0 { Some(code_interpreter) } else { None };
        self
    }
}

/// Represents the AI's response, which can include text and/or an image.
#[derive(Debug)]
pub struct AIResponse {
    pub text: String,
    pub image_data: Option<Vec<u8>>,
    pub tool_calls: Option<Vec<FunctionCallInfo>>,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    // Tool usage fields - optional for backward compatibility
    pub web_search: Option<u32>,
    pub file_search: Option<u32>, 
    pub image_generation: Option<u32>,
    pub code_interpreter: Option<u32>,
}

impl AIResponse {
    /// Calculate tool usage from OpenAI Response by analyzing output array
    pub fn calculate_tool_usage(response: &Response) -> (u32, u32, u32, u32) {
        let mut web_search = 0u32;
        let mut file_search = 0u32;
        let mut image_generation = 0u32;
        let mut code_interpreter = 0u32;

        for item in &response.output {
            match item {
                ResponseItem::WebSearchCall { .. } => web_search += 1,
                ResponseItem::FileSearchCall { .. } => file_search += 1,
                ResponseItem::ImageGenerationCall { .. } => image_generation += 1,
                ResponseItem::CodeInterpreterCall { .. } => code_interpreter += 1,
                _ => {}
            }
        }

        (web_search, file_search, image_generation, code_interpreter)
    }

    /// Returns enhanced Usage object with both token counts and tool usage populated
    pub fn usage_with_tools(&self) -> Option<Usage> {
        if self.prompt_tokens == 0 && self.output_tokens == 0 && self.total_tokens == 0 {
            return None;
        }

        Some(Usage::new(self.prompt_tokens, self.output_tokens, self.total_tokens)
            .with_tool_usage(
                self.web_search.unwrap_or(0),
                self.file_search.unwrap_or(0), 
                self.image_generation.unwrap_or(0),
                self.code_interpreter.unwrap_or(0)
            ))
    }

    /// Returns user-requested format with token and tool statistics
    pub fn format_usage(&self) -> String {
        let mut result = format!(
            "input tokens: {}\noutput tokens: {}\ntotal tokens: {}",
            self.prompt_tokens, self.output_tokens, self.total_tokens
        );

        if let Some(web_search) = self.web_search {
            result.push_str(&format!("\nweb search: {}", web_search));
        } else {
            result.push_str("\nweb search: 0");
        }

        if let Some(file_search) = self.file_search {
            result.push_str(&format!("\nfile search: {}", file_search));
        } else {
            result.push_str("\nfile search: 0");
        }

        if let Some(image_generation) = self.image_generation {
            result.push_str(&format!("\nimage generation: {}", image_generation));
        } else {
            result.push_str("\nimage generation: 0");
        }

        if let Some(code_interpreter) = self.code_interpreter {
            result.push_str(&format!("\ncode interpreter: {}", code_interpreter));
        } else {
            result.push_str("\ncode interpreter: 0");
        }

        result
    }

    /// Get raw counts for custom formatting
    pub fn get_tool_usage_counts(&self) -> (u32, u32, u32, u32) {
        (
            self.web_search.unwrap_or(0),
            self.file_search.unwrap_or(0),
            self.image_generation.unwrap_or(0),
            self.code_interpreter.unwrap_or(0)
        )
    }
}

impl From<(String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>, u32, u32, u32)> for AIResponse {
    fn from(value: (String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>, u32, u32, u32)) -> Self {
        let (text, image_data, tool_calls, prompt_tokens, output_tokens, total_tokens) = value;

        Self {
            text,
            image_data,
            tool_calls,
            prompt_tokens,
            output_tokens,
            total_tokens,
            web_search: None,
            file_search: None,
            image_generation: None,
            code_interpreter: None,
        }
    }
}

// Enhanced constructor with tool usage tracking
impl From<(String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>, u32, u32, u32, u32, u32, u32, u32)> for AIResponse {
    fn from(value: (String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>, u32, u32, u32, u32, u32, u32, u32)) -> Self {
        let (text, image_data, tool_calls, prompt_tokens, output_tokens, total_tokens, web_search, file_search, image_generation, code_interpreter) = value;

        Self {
            text,
            image_data,
            tool_calls,
            prompt_tokens,
            output_tokens,
            total_tokens,
            web_search: if web_search > 0 { Some(web_search) } else { None },
            file_search: if file_search > 0 { Some(file_search) } else { None },
            image_generation: if image_generation > 0 { Some(image_generation) } else { None },
            code_interpreter: if code_interpreter > 0 { Some(code_interpreter) } else { None },
        }
    }
}

// Backward compatibility constructor
impl From<(String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>)> for AIResponse {
    fn from(value: (String, Option<Vec<u8>>, Option<Vec<FunctionCallInfo>>)) -> Self {
        let (text, image_data, tool_calls) = value;

        Self {
            text,
            image_data,
            tool_calls,
            prompt_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            web_search: None,
            file_search: None,
            image_generation: None,
            code_interpreter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_tool_usage_tracking() {
        // Create an AIResponse with tool usage
        let ai_response = AIResponse::from((
            "Test response".to_string(),
            None,
            None,
            150u32,  // input tokens
            75u32,   // output tokens
            225u32,  // total tokens
            1u32,    // web search
            0u32,    // file search
            2u32,    // image generation
            0u32,    // code interpreter
        ));

        // Test get_tool_usage_counts method
        let (web, file, image, code) = ai_response.get_tool_usage_counts();
        assert_eq!(web, 1);
        assert_eq!(file, 0);
        assert_eq!(image, 2);
        assert_eq!(code, 0);

        // Test usage_with_tools method
        if let Some(usage) = ai_response.usage_with_tools() {
            assert_eq!(usage.input_tokens, 150);
            assert_eq!(usage.output_tokens, 75);
            assert_eq!(usage.total_tokens, 225);
            assert_eq!(usage.web_search, Some(1));
            assert_eq!(usage.file_search, None);
            assert_eq!(usage.image_generation, Some(2));
            assert_eq!(usage.code_interpreter, None);
        } else {
            panic!("usage_with_tools should return Some");
        }

        // Test format_usage method
        let formatted = ai_response.format_usage();
        assert!(formatted.contains("input tokens: 150"));
        assert!(formatted.contains("output tokens: 75"));
        assert!(formatted.contains("total tokens: 225"));
        assert!(formatted.contains("web search: 1"));
        assert!(formatted.contains("file search: 0"));
        assert!(formatted.contains("image generation: 2"));
        assert!(formatted.contains("code interpreter: 0"));
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that existing usage tracking continues to work unchanged
        let ai_response = AIResponse::from((
            "Legacy response".to_string(),
            None,
            None,
            100u32,
            50u32,
            150u32,
        ));

        assert_eq!(ai_response.prompt_tokens, 100);
        assert_eq!(ai_response.output_tokens, 50);
        assert_eq!(ai_response.total_tokens, 150);
        assert_eq!(ai_response.web_search, None);
        assert_eq!(ai_response.file_search, None);
        assert_eq!(ai_response.image_generation, None);
        assert_eq!(ai_response.code_interpreter, None);
    }

    #[test]
    fn test_usage_struct() {
        let usage = Usage::new(100, 50, 150)
            .with_tool_usage(1, 0, 2, 0);

        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.web_search, Some(1));
        assert_eq!(usage.file_search, None);
        assert_eq!(usage.image_generation, Some(2));
        assert_eq!(usage.code_interpreter, None);
    }
}
