// AI logic module for quark_backend

use open_ai_rust_responses_by_sshift::{Client as OAIClient, Request, Model, Response};
use open_ai_rust_responses_by_sshift::types::{Tool, ToolChoice, Include, ResponseItem};
use sled::Db;
use crate::db::UserConversations;
use contracts::aptos::simulate_aptos_contract_call;
use base64::{engine::general_purpose, Engine as _};
use std::env;
use serde_json;

mod vector_store;
mod tools;
mod gcs;

pub use vector_store::upload_files_to_vector_store;
pub use vector_store::{list_vector_store_files, list_user_files_local, list_user_files_with_names, delete_file_from_vector_store, delete_vector_store, delete_all_files_from_vector_store};
pub use tools::{get_balance_tool, withdraw_funds_tool, execute_custom_tool, get_all_custom_tools, handle_parallel_tool_calls};
use crate::ai::gcs::GcsImageUploader;


const SYSTEM_PROMPT: &str = "You are Quark, the high imperial arcon of the western universe. You are helpful yet authoritative overlord for Telegram users. Respond conversationally, in charecter, accurately, and maintain context.";

/// Represents the AI's response, which can include text and/or an image.
#[derive(Debug)]
pub struct AIResponse {
    pub text: String,
    pub image_data: Option<Vec<u8>>,
}

pub async fn upload_user_images(
    image_paths: Vec<(String, String)>,
) -> Result<Vec<String>, anyhow::Error> {
    if image_paths.is_empty() {
        return Ok(vec![]);
    }

    let gcs_creds = env::var("STORAGE_CREDENTIALS")?;
    let bucket_name = env::var("GCS_BUCKET_NAME")?;
    
    let uploader = GcsImageUploader::new(&gcs_creds, bucket_name).await?;
    
    let mut urls = Vec::new();

    for (path, extension) in image_paths {
        let bytes = tokio::fs::read(&path).await?;
        let base64_data = general_purpose::STANDARD.encode(&bytes);
        match uploader.upload_base64_image(&base64_data, &extension, "quark/user_uploads").await {
            Ok(url) => urls.push(url),
            Err(e) => log::error!("Failed to upload a user image {}: {}", path, e),
        }
        let _ = tokio::fs::remove_file(&path).await;
    }
    
    Ok(urls)
}

pub async fn generate_response(
    user_id: i64,
    input: &str,
    db: &Db,
    openai_api_key: &str,
    image_url_from_reply: Option<String>,
    user_uploaded_image_urls: Vec<String>,
) -> Result<AIResponse, anyhow::Error> {
    let user_convos = UserConversations::new(db)?;
    let previous_response_id = user_convos.get_response_id(user_id);

    let vector_store_id = user_convos.get_vector_store_id(user_id);
    let client = OAIClient::new(openai_api_key)?;
    


    // Simulate contract call (logging is handled in the contract module)
    let _ = simulate_aptos_contract_call(user_id);

    // Enhanced tools: built-in tools + custom function tools
    let mut tools = vec![
        Tool::web_search_preview(),
        Tool::image_generation(),
    ];
    if let Some(vs_id) = vector_store_id.clone() {
        tools.push(Tool::file_search(vec![vs_id]));
    }
    // Add custom function tools (get_balance, withdraw_funds, etc.)
    tools.extend(get_all_custom_tools());

    let mut request_builder = Request::builder()
        .model(Model::GPT4o)
        .instructions(SYSTEM_PROMPT)
        .tools(tools.clone())
        .tool_choice(ToolChoice::auto())
        .parallel_tool_calls(true) // Enable parallel execution for efficiency
        .max_output_tokens(1000)
        .temperature(0.5)
        .user(&format!("user-{}", user_id))
        .store(true);

    // ---- Attach vision inputs using the SDK helper (0.2.1) ----
    // Collect all image URLs we want GPT-4o to see
    let mut image_urls: Vec<String> = Vec::new();
    if let Some(url) = image_url_from_reply {
        image_urls.push(url);
    }
    image_urls.extend(user_uploaded_image_urls);

    // Also include any previously generated images that haven't been used yet
    {
        let mut cached = user_convos.take_last_image_urls(user_id);
        image_urls.append(&mut cached);
    }

    if !image_urls.is_empty() {
        // New helper in v0.2.2 supports multiple images in one call
        request_builder = request_builder.input_image_urls(&image_urls);

        // Include accompanying text (if any) as instructions
        if !input.trim().is_empty() {
            request_builder = request_builder.instructions(input);
        }
    } else {
        // No images ⇒ plain text input as before
        request_builder = request_builder.input(input);
        // With no vision payload we can safely include file-search results
        request_builder = request_builder.include(vec![Include::FileSearchResults]);
    }

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
        
        // Filter for custom function calls (get_balance, withdraw_funds, get_trending_pools, search_pools, get_current_time, get_fear_and_greed_index)
        let custom_tool_calls: Vec<_> = tool_calls.iter()
            .filter(|tc| tc.name == "get_balance"
                || tc.name == "withdraw_funds"
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
                let result = execute_custom_tool(&tool_call.name, &args_value).await;
                function_outputs.push((tool_call.call_id.clone(), result));
            }
            
            // Submit tool outputs using Responses API pattern (with_function_outputs)
            let continuation_request = Request::builder()
                .model(Model::GPT4o)
                .with_function_outputs(current_response.id(), function_outputs)
                .tools(tools.clone()) // Keep tools available for follow-ups
                .instructions(SYSTEM_PROMPT)
                .parallel_tool_calls(true)
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
    
    // Extract text and potentially image data from the final response
    let mut reply = current_response.output_text();
    let response_id = current_response.id().to_string();
    user_convos.set_response_id(user_id, &response_id)?;

    let mut image_data: Option<Vec<u8>> = None;
    for item in &current_response.output {
        if let ResponseItem::ImageGenerationCall { result, .. } = item {
            // Decode the base64 string to image bytes
            match general_purpose::STANDARD.decode(result) {
                Ok(bytes) => {
                    image_data = Some(bytes);
                    
                    // Upload to GCS and append URL to reply
                    if let (Ok(gcs_creds), Ok(bucket_name)) = (env::var("STORAGE_CREDENTIALS"), env::var("GCS_BUCKET_NAME")) {
                        if let Ok(uploader) = GcsImageUploader::new(&gcs_creds, bucket_name).await {
                            match uploader.upload_base64_image(result, "png", "quark/images").await {
                                Ok(url) => {
                                    reply = format!("{}\n\nImage URL: {}", reply, url);
                                    let _ = user_convos.set_last_image_urls(user_id, &[url.clone()]);
                                }
                                Err(e) => log::error!("Failed to upload image to GCS: {}", e),
                            }
                        }
                    } else {
                        log::warn!("STORAGE_CREDENTIALS or GCS_BUCKET_NAME not set. Skipping image upload.");
                    }

                    // We found our image, no need to look further
                    break; 
                }
                Err(e) => {
                    // Log the error but don't fail the entire response
                    eprintln!("Error decoding base64 image: {}", e);
                }
            }
        }
    }

    Ok(AIResponse {
        text: reply,
        image_data,
    })
} 