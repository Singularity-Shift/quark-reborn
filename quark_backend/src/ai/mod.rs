// AI logic module for quark_backend

use open_ai_rust_responses_by_sshift::{Client as OAIClient, Request, Model, Response};
use open_ai_rust_responses_by_sshift::types::{Tool, ToolChoice, Include};
use sled::Db;
use crate::db::UserConversations;
use contracts::aptos::simulate_aptos_contract_call;

const SYSTEM_PROMPT: &str = "You are Quark, a helpful and friendly assistant for Telegram groups. Respond conversationally and maintain context.";

pub async fn generate_response(
    user_id: i64,
    input: &str,
    db: &Db,
    openai_api_key: &str,
) -> Result<String, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let previous_response_id = user_convos.get_response_id(user_id);
    let client = OAIClient::new(openai_api_key)?;

    // Simulate contract call (logging is handled in the contract module)
    let _ = simulate_aptos_contract_call(user_id);

    // Always include the websearch tool
    let tools = vec![Tool::web_search_preview()];

    let mut request_builder = Request::builder()
        .model(Model::GPT41Mini)
        .input(input)
        .instructions(SYSTEM_PROMPT)
        .tools(tools)
        .tool_choice(ToolChoice::auto())
        .include(vec![Include::FileSearchResults])
        .max_output_tokens(300)
        .temperature(0.2)
        .user(&format!("user-{}", user_id))
        .store(true);

    if let Some(prev_id) = previous_response_id.clone() {
        request_builder = request_builder.previous_response_id(prev_id);
    }

    let request = request_builder.build();
    let response: Response = client.responses.create(request).await?;
    let reply = response.output_text();
    let response_id = response.id().to_string();
    user_convos.set_response_id(user_id, &response_id)?;
    Ok(reply)
} 