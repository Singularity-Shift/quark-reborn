use open_ai_rust_responses_by_sshift::{Client as OAIClient, Model, Request, Verbosity, ReasoningEffort};

pub fn build_summarization_prompt(
    latest_user_input: &str,
    latest_assistant_reply: &str,
) -> String {
    let prompt = format!(
        "Summarize the conversation so far for future context. Include key facts, decisions, named entities, constraints, and unresolved items. Keep it concise (< 200 words). Avoid pleasantries and repetitive details.\n\nLatest user input: {}\n\nLatest assistant reply: {}\n\nNew summary:",
        latest_user_input,
        latest_assistant_reply
    );

    prompt
}

pub fn get_conversation_summary_key(user_id: i64) -> String {
    format!("user:{}", user_id)
}

pub fn should_summarize(total_tokens: u32, token_limit: u32) -> bool {
    total_tokens > token_limit
}

pub async fn generate_summary(
    openai_client: &OAIClient,
    prompt: &str,
) -> Result<String, anyhow::Error> {
    let full_prompt = format!(
        "You are a conversation summarizer. Generate concise, factual summaries.\n\n{}",
        prompt
    );

    let request = Request::builder()
        .model(Model::GPT5Nano)
        .input(full_prompt)
        .max_output_tokens(300)
        .verbosity(Verbosity::Low)
        .reasoning_effort(ReasoningEffort::Minimal)
        .build();

    let response = openai_client.responses.create(request).await?;
    let summary = response.output_text().trim().to_string();

    if summary.is_empty() {
        return Err(anyhow::anyhow!("Generated summary is empty"));
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_summarization_prompt() {
        let prompt = build_summarization_prompt(
            "User input",
            "Assistant reply"
        );
        
        assert!(prompt.contains("User input"));
        assert!(prompt.contains("Assistant reply"));
        assert!(prompt.contains("New summary:"));
        assert!(!prompt.contains("Previous summary"));
    }

    #[test]
    fn test_get_conversation_summary_key() {
        let key = get_conversation_summary_key(12345);
        assert_eq!(key, "user:12345");
    }

    #[test]
    fn test_should_summarize() {
        assert!(should_summarize(13000, 12000));
        assert!(!should_summarize(11000, 12000));
        assert!(!should_summarize(12000, 12000));
    }
}
