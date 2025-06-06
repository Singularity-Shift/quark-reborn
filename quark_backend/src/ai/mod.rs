// AI logic module for quark_backend

use open_ai_rust_responses_by_sshift::{Client as OAIClient, Request, Model, Response};
use open_ai_rust_responses_by_sshift::types::{Tool, ToolChoice, Include};
use sled::Db;
use crate::db::UserConversations;
use contracts::aptos::simulate_aptos_contract_call;

mod vector_store;
mod tools;
mod gcs;

pub use vector_store::upload_files_to_vector_store;
pub use vector_store::{list_vector_store_files, list_user_files_local, list_user_files_with_names, delete_file_from_vector_store, delete_vector_store, delete_all_files_from_vector_store};
pub use tools::{get_balance_tool, withdraw_funds_tool, generate_image_tool, execute_custom_tool, get_all_custom_tools, handle_parallel_tool_calls};
pub use gcs::GcsImageUploader;

const SYSTEM_PROMPT: &str = "You are Quark, the high imperial arcon of the western universe. You are helpful yet authoritative overlord for Telegram users. Respond conversationally, in charecter, accurately, and maintain context.";

pub async fn generate_response(
    user_id: i64,
    input: &str,
    db: &Db,
    openai_api_key: &str,
    storage_credentials: &str,
    bucket_name: &str,
) -> Result<String, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let previous_response_id = user_convos.get_response_id(user_id);
    let vector_store_id = user_convos.get_vector_store_id(user_id);
    let client = OAIClient::new(openai_api_key)?;
    
    // Initialize GCS uploader
    let gcs_uploader = GcsImageUploader::new(storage_credentials, bucket_name.to_string()).await?;

    // Simulate contract call (logging is handled in the contract module)
    let _ = simulate_aptos_contract_call(user_id);

    // Enhanced tools: built-in tools + custom function tools
    let mut tools = vec![Tool::web_search_preview()];
    if let Some(vs_id) = vector_store_id.clone() {
        tools.push(Tool::file_search(vec![vs_id]));
    }
    // Add custom function tools (get_balance, withdraw_funds, generate_image)
    tools.extend(get_all_custom_tools());

    let mut request_builder = Request::builder()
        .model(Model::GPT4o)
        .input(input)
        .instructions(SYSTEM_PROMPT)
        .tools(tools.clone())
        .tool_choice(ToolChoice::auto())
        .parallel_tool_calls(true) // Enable parallel execution for efficiency
        .include(vec![Include::FileSearchResults])
        .max_output_tokens(1000)
        .temperature(0.5)
        .user(&format!("user-{}", user_id))
        .store(true);

    if let Some(prev_id) = previous_response_id.clone() {
        request_builder = request_builder.previous_response_id(prev_id);
    }

    let request = request_builder.build();
    let mut current_response: Response = client.responses.create(request).await?;
    
    // Enhanced function calling loop (following demo script pattern)
    let mut iteration = 1;
    const MAX_ITERATIONS: usize = 5; // Prevent infinite loops
    
    while !current_response.tool_calls().is_empty() && iteration <= MAX_ITERATIONS {
        let tool_calls = current_response.tool_calls();
        
        // Filter for custom function calls (get_balance, withdraw_funds, generate_image, get_trending_pools, search_pools, get_current_time, get_fear_and_greed_index)
        let custom_tool_calls: Vec<_> = tool_calls.iter()
            .filter(|tc| tc.name == "get_balance"
                || tc.name == "withdraw_funds"
                || tc.name == "generate_image"
                || tc.name == "get_trending_pools"
                || tc.name == "search_pools"
                || tc.name == "get_new_pools"
                || tc.name == "get_current_time"
                || tc.name == "get_fear_and_greed_index")
            .collect();
        
        if !custom_tool_calls.is_empty() {
            // Handle parallel custom function calls 
            let mut function_outputs = Vec::new();
            for tool_call in &custom_tool_calls {
                // Parse arguments as JSON Value for execute_custom_tool
                let args_value: serde_json::Value = serde_json::from_str(&tool_call.arguments)
                    .unwrap_or_else(|_| serde_json::json!({}));
                let result = execute_custom_tool(&tool_call.name, &args_value, &client, &gcs_uploader).await;
                function_outputs.push((tool_call.call_id.clone(), result));
            }
            
            // Submit tool outputs using Responses API pattern (with_function_outputs)
            let continuation_request = Request::builder()
                .model(Model::GPT4o)
                .with_function_outputs(current_response.id(), function_outputs)
                .tools(tools.clone()) // Keep tools available for follow-ups
                .instructions(SYSTEM_PROMPT)
                .parallel_tool_calls(true)
                .include(vec![Include::FileSearchResults])
                .max_output_tokens(1000)
                .temperature(0.5)
                .user(&format!("user-{}", user_id))
                .store(true)
                .build();
            
            current_response = client.responses.create(continuation_request).await?;
        } else {
            // No custom function calls, break the loop
            // (Built-in tools like web_search and file_search are handled automatically by OpenAI)
            break;
        }
        
        iteration += 1;
    }
    
    let reply = current_response.output_text();
    let response_id = current_response.id().to_string();
    user_convos.set_response_id(user_id, &response_id)?;
    Ok(reply)
} 