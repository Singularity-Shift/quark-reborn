// AI logic module for quark_backend

use open_ai_rust_responses_by_sshift::{Client as OAIClient, Request, Model, Response};
use open_ai_rust_responses_by_sshift::types::{Tool, ToolChoice, Include};
use sled::Db;
use crate::db::UserConversations;
use contracts::aptos::simulate_aptos_contract_call;

mod vector_store;
pub use vector_store::upload_files_to_vector_store;
pub use vector_store::{list_vector_store_files, list_user_files_local, list_user_files_with_names, delete_file_from_vector_store, delete_vector_store, delete_all_files_from_vector_store};

const SYSTEM_PROMPT: &str = "You are Quark, the high imperial arcon of the western universe. You are helpful yet authoritative overlord for Telegram users. Respond conversationally, in charecter, accurately, and maintain context.";

pub async fn generate_response(
    user_id: i64,
    input: &str,
    db: &Db,
    openai_api_key: &str,
) -> Result<String, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let previous_response_id = user_convos.get_response_id(user_id);
    let vector_store_id = user_convos.get_vector_store_id(user_id);
    let client = OAIClient::new(openai_api_key)?;

    // Simulate contract call (logging is handled in the contract module)
    let _ = simulate_aptos_contract_call(user_id);

    // Always include the websearch tool, and file search if user has a vector store
    let mut tools = vec![Tool::web_search_preview()];
    if let Some(vs_id) = vector_store_id.clone() {
        tools.push(Tool::file_search(vec![vs_id]));
    }

    let mut request_builder = Request::builder()
        .model(Model::GPT4o)
        .input(input)
        .instructions(SYSTEM_PROMPT)
        .tools(tools)
        .tool_choice(ToolChoice::auto())
        .include(vec![Include::FileSearchResults])
        .max_output_tokens(1000)
        .temperature(0.5)
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