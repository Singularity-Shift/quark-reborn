// AI logic module for quark_backend

use open_ai_rust_responses_by_sshift::{Client as OAIClient, Request, Model, Response};
use sled::Db;
use crate::db::UserConversations;

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

    let mut request_builder = Request::builder()
        .model(Model::GPT41Mini)
        .input(input)
        .instructions(SYSTEM_PROMPT);

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