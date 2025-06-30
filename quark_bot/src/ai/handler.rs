use crate::ai::dto::AIResponse;
use crate::ai::gcs::GcsImageUploader;
use crate::ai::prompt::get_prompt;
use crate::ai::tools::{execute_custom_tool, get_all_custom_tools};
use crate::panora::handler::Panora;
use crate::services::handler::Services;
use crate::user_conversation::handler::UserConversations;
use aptos_rust_sdk::client::builder::AptosClientBuilder;
use aptos_rust_sdk::client::config::AptosNetwork;
use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::chain_id::ChainId;
use base64::{Engine as _, engine::general_purpose};
use open_ai_rust_responses_by_sshift::types::{
    Container, Include, InputItem, ReasoningParams, Response, ResponseItem, Tool, ToolChoice,
};
use open_ai_rust_responses_by_sshift::{Client as OAIClient, FunctionCallInfo, Model, Request, RecoveryPolicy};
use serde_json;
use sled::{Db, Tree};
use teloxide::types::Message;

#[derive(Clone)]
pub struct AI {
    openai_client: OAIClient,
    node: AptosFullnodeClient,
    system_prompt: String,
    cloud: GcsImageUploader,
    panora: Panora,
    service: Services,
}

impl AI {
    pub fn new(openai_api_key: String, cloud: GcsImageUploader, aptos_network: String) -> Self {
        let (builder, _chain_id) = match aptos_network.as_str() {
            "mainnet" => (
                AptosClientBuilder::new(AptosNetwork::mainnet()),
                ChainId::Mainnet,
            ),
            "testnet" => (
                AptosClientBuilder::new(AptosNetwork::testnet()),
                ChainId::Testnet,
            ),
            "devnet" => (
                AptosClientBuilder::new(AptosNetwork::devnet()),
                ChainId::Testing,
            ),
            _ => (
                AptosClientBuilder::new(AptosNetwork::testnet()),
                ChainId::Testnet,
            ),
        };
        let node = builder.build();

        let system_prompt = get_prompt();

        // Use default recovery policy for container expiration handling
        // This provides automatic retry with 1 attempt for seamless experience
        let recovery_policy = RecoveryPolicy::default();
        let openai_client = OAIClient::new_with_recovery(&openai_api_key, recovery_policy)
            .expect("Failed to create OpenAI client with recovery policy");

        let panora = Panora::new();
        let service = Services::new();

        Self {
            openai_client,
            system_prompt,
            node,
            cloud,
            panora,
            service,
        }
    }

    pub fn get_client(&self) -> &OAIClient {
        &self.openai_client
    }

    pub async fn upload_user_images(
        &self,
        image_paths: Vec<(String, String)>,
    ) -> Result<Vec<String>, anyhow::Error> {
        if image_paths.is_empty() {
            return Ok(vec![]);
        }

        let mut urls = Vec::new();

        for (path, extension) in image_paths {
            let bytes = tokio::fs::read(&path).await?;
            let base64_data = general_purpose::STANDARD.encode(&bytes);
            match self
                .cloud
                .upload_base64_image(&base64_data, &extension, "quark/user_uploads")
                .await
            {
                Ok(url) => urls.push(url),
                Err(e) => log::error!("Failed to upload a user image {}: {}", path, e),
            }
            let _ = tokio::fs::remove_file(&path).await;
        }

        Ok(urls)
    }

    pub async fn generate_response(
        &self,
        msg: Message,
        user_id: i64,
        input: &str,
        db: &Db,
        tree: Tree,
        image_url_from_reply: Option<String>,
        user_uploaded_image_urls: Vec<String>,
        model: Model,
        max_tokens: u32,
        temperature: Option<f32>,
        reasoning: Option<ReasoningParams>,
    ) -> Result<AIResponse, anyhow::Error> {
        log::info!(
            "AI generate_response called for user {} with input: '{}'",
            user_id,
            input
        );
        let user_convos = UserConversations::new(db)?;
        let previous_response_id = user_convos.get_response_id(user_id);
        let mut tool_called: Vec<FunctionCallInfo> = Vec::new();
        let mut was_container_recovery = false;

        let vector_store_id = user_convos.get_vector_store_id(user_id);

        // Enhanced tools: built-in tools + custom function tools
        let mut tools = vec![];

        // Add image generation only for non-O-series models (O-series don't support it)
        if !matches!(
            model,
            Model::O3 | Model::O4Mini | Model::O1 | Model::O1Mini | Model::O1Preview
        ) {
            tools.push(Tool::image_generation());
        }

        // Add web search for all models
        tools.push(Tool::web_search_preview());

        if let Some(vs_id) = vector_store_id.clone() {
            if !vs_id.is_empty() {
                tools.push(Tool::file_search(vec![vs_id]));
            }
        }

        // Add custom function tools (get_balance, withdraw_funds, etc.)
        tools.extend(get_all_custom_tools());

        let mut request_builder = Request::builder()
            .model(model.clone())
            .instructions(self.system_prompt.clone())
            .tools(tools.clone())
            .tool_choice(ToolChoice::auto())
            .parallel_tool_calls(true) // Enable parallel execution for efficiency
            .max_output_tokens(max_tokens)
            .user(&format!("user-{}", user_id))
            .store(true);

        if let Some(temp) = temperature {
            request_builder = request_builder.temperature(temp);
        }

        if let Some(reasoning_params) = reasoning.clone() {
            request_builder = request_builder.reasoning(reasoning_params);
        }

        // ---- Attach vision inputs using the SDK helper (0.2.1) ----
        // Collect all image URLs we want GPT-4o to see
        let mut image_urls: Vec<String> = Vec::new();
        if let Some(url) = image_url_from_reply {
            image_urls.push(url);
        }
        image_urls.extend(user_uploaded_image_urls.clone());

        if !image_urls.is_empty() {
            let mut content = Vec::new();
            // Add all images to the content block with detail level 'high'
            for url in image_urls {
                content.push(InputItem::content_image_with_detail(&url, "high"));
            }
            // Add the text prompt to the content block
            if !input.trim().is_empty() {
                content.push(InputItem::content_text(input));
            }
            // Manually construct the message with multiple content items
            request_builder =
                request_builder.input_items(vec![InputItem::message("user", content)]);
        } else {
            // No images ⇒ plain text input as before
            request_builder = request_builder.input(input);
            // With no vision payload we can safely include file-search results if user has a vector store
            if vector_store_id.is_some() {
                request_builder = request_builder.include(vec![Include::FileSearchResults]);
            }
        }

        if let Some(prev_id) = previous_response_id.clone() {
            request_builder = request_builder.previous_response_id(prev_id);
        }

        let request = request_builder.build();
        log::info!("Making OpenAI API call with {} tools", tools.len());
        for tool in &tools {
            if let Some(func) = &tool.function {
                log::info!("Tool available: {}", func.name);
            }
        }

        log::info!("About to call OpenAI API...");
        let mut current_response: Response = match self
            .openai_client
            .responses
            .create(request)
            .await
        {
            Ok(response) => {
                log::info!("OpenAI API call successful, response ID: {}", response.id());
                response
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::error!("OpenAI API call failed: {}", error_msg);

                // Handle vector store not found errors
                if error_msg.contains("Vector store") && error_msg.contains("not found") {
                    log::warn!("Vector store not found, clearing orphaned reference for user {}", user_id);
                    // Centralized cleanup
                    if let Err(clear_err) = user_convos.cleanup_orphaned_vector_store(user_id) {
                        log::error!("Failed to clean up orphaned vector store: {}", clear_err);
                    }
                    // Return a user-friendly error with suggestion to upload files
                    return Err(anyhow::anyhow!(
                        "Your document library is no longer available (vector store deleted). Please upload files again using /add_files to create a new document library."
                    ));
                }

                // Handle container expiry and previous response not found errors  
                let error_lower = error_msg.to_lowercase();
                if error_lower.contains("container") && (error_lower.contains("expired") || error_lower.contains("not found")) 
                    || error_msg.contains("Previous response") && error_msg.contains("not found") {
                    log::warn!("Container expired for user {}, continuing conversation without code interpreter", user_id);
                    user_convos.clear_response_id(user_id)?;
                    was_container_recovery = true;
                    
                    // Rebuild request without code interpreter tool
                    let mut tools_without_code = tools.clone();
                    tools_without_code.retain(|tool| {
                        !matches!(tool.function.as_ref().map(|f| f.name.as_str()), Some("code_interpreter"))
                    });
                    
                    let fallback_request = Request::builder()
                        .model(model.clone())
                        .instructions(self.system_prompt.clone())
                        .tools(tools_without_code) // No code interpreter
                        .tool_choice(ToolChoice::auto())
                        .parallel_tool_calls(true)
                        .max_output_tokens(max_tokens)
                        .user(&format!("user-{}", user_id))
                        .store(true)
                        .input(&format!("Note: Python code execution is temporarily unavailable. {}", input))
                        .build();

                    log::info!("Retrying request without code interpreter for user {}", user_id);
                    match self.openai_client.responses.create(fallback_request).await {
                        Ok(response) => {
                            log::info!("Fallback successful for user {}", user_id);
                            response
                        }
                        Err(retry_err) => {
                            log::error!("Fallback also failed: {}", retry_err);
                            return Err(retry_err.into());
                        }
                    }
                } else {
                    return Err(e.into());
                }
            }
        };

        // Enhanced function calling loop (following demo script pattern)
        let mut iteration = 1;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops

        log::info!(
            "Initial response has {} tool calls",
            current_response.tool_calls().len()
        );

        while !current_response.tool_calls().is_empty() && iteration <= MAX_ITERATIONS {
            let tool_calls = current_response.tool_calls();
            log::info!(
                "AI Response has {} tool calls in iteration {}",
                tool_calls.len(),
                iteration
            );

            // Log all tool calls first
            for tc in &tool_calls {
                log::info!("Tool call found: {} with call_id: {}", tc.name, tc.call_id);
            }

            // Filter for custom function calls (get_balance, get_wallet_address, withdraw_funds, fund_account, get_trending_pools, search_pools, get_current_time, get_fear_and_greed_index, get_pay_users)
            let custom_tool_calls: Vec<_> = tool_calls
                .iter()
                .filter(|tc| {
                    tc.name == "get_balance"
                        || tc.name == "get_wallet_address"
                        || tc.name == "withdraw_funds"
                        || tc.name == "fund_account"
                        || tc.name == "get_trending_pools"
                        || tc.name == "search_pools"
                        || tc.name == "get_new_pools"
                        || tc.name == "get_current_time"
                        || tc.name == "get_fear_and_greed_index"
                        || tc.name == "get_pay_users"
                })
                .collect();

            tool_called.extend(custom_tool_calls.iter().map(|tc| (*tc).clone()));

            log::info!(
                "Found {} custom tool calls out of {} total",
                custom_tool_calls.len(),
                tool_calls.len()
            );

            if !custom_tool_calls.is_empty() {
                // Handle parallel custom function calls
                let mut function_outputs = Vec::new();
                for tool_call in &custom_tool_calls {
                    log::info!(
                        "Executing custom tool: {} with call_id: {}",
                        tool_call.name,
                        tool_call.call_id
                    );

                    // Parse arguments as JSON Value for execute_custom_tool
                    let args_value: serde_json::Value = serde_json::from_str(&tool_call.arguments)
                        .unwrap_or_else(|e| {
                            log::error!("Failed to parse tool arguments: {}", e);
                            serde_json::json!({})
                        });

                    let result = execute_custom_tool(
                        &tool_call.name,
                        &args_value,
                        msg.clone(),
                        self.service.clone(),
                        tree.clone(),
                        self.node.clone(),
                        self.panora.clone(),
                    )
                    .await;

                    log::info!(
                        "Tool {} executed successfully, result length: {}",
                        tool_call.name,
                        result.len()
                    );

                    // Ensure result is not empty
                    let final_result = if result.trim().is_empty() {
                        log::warn!(
                            "Tool {} returned empty result, providing default",
                            tool_call.name
                        );
                        format!("Tool '{}' completed but returned no output", tool_call.name)
                    } else {
                        result
                    };

                    function_outputs.push((tool_call.call_id.clone(), final_result));
                }

                // Submit tool outputs using Responses API pattern (with_function_outputs)
                let mut continuation_builder = Request::builder()
                    .model(model.clone())
                    .with_function_outputs(current_response.id(), function_outputs)
                    .tools(tools.clone()) // Keep tools available for follow-ups
                    .instructions(self.system_prompt.clone())
                    .parallel_tool_calls(true)
                    .max_output_tokens(max_tokens)
                    .user(&format!("user-{}", user_id))
                    .store(true);

                if let Some(temp) = temperature {
                    continuation_builder = continuation_builder.temperature(temp);
                }

                if let Some(reasoning_params) = reasoning.clone() {
                    continuation_builder = continuation_builder.reasoning(reasoning_params);
                }

                let continuation_request = continuation_builder.build();

                log::info!("Making continuation request to OpenAI");
                current_response = self
                    .openai_client
                    .responses
                    .create(continuation_request)
                    .await?;
                log::info!("Continuation request completed");
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
        
        // Append container expiry explanation if this was a recovery
        if was_container_recovery {
            reply = format!("{}\n\n---\n*Note: The previous code execution environment expired after 20 minutes of inactivity, so this is a fresh conversation session.*", reply);
        }

        // Save response ID for future conversation context (remove O-series container check)
        user_convos.set_response_id(user_id, &response_id)?;
        log::info!(
            "Saved response ID {} for future conversation context",
            response_id
        );

        let mut image_data: Option<Vec<u8>> = None;
        for item in &current_response.output {
            if let ResponseItem::ImageGenerationCall { result, .. } = item {
                // Decode the base64 string to image bytes
                match general_purpose::STANDARD.decode(result) {
                    Ok(bytes) => {
                        image_data = Some(bytes);

                        // Upload to GCS and append URL to reply
                        match self
                            .cloud
                            .upload_base64_image(result, "png", "quark/images")
                            .await
                        {
                            Ok(url) => {
                                reply = format!("{}\n\nImage URL: {}", reply, url);
                            }
                            Err(e) => log::error!("Failed to upload image to GCS: {}", e),
                        }

                        // We found our image, no need to look further
                        break;
                    }
                    Err(e) => {
                        // Log the error but don't fail the entire response
                        log::error!("Error decoding base64 image: {}", e);
                    }
                }
            }

            // Handle code interpreter calls
            if let ResponseItem::CodeInterpreterCall {
                id,
                container_id,
                status,
            } = item
            {
                log::info!(
                    "Code interpreter executed: ID={}, Container={}, Status={}",
                    id,
                    container_id,
                    status
                );
                // The code execution result is already included in the response text
                // Additional processing could be added here if needed (e.g., file handling)
            }
        }

        Ok(AIResponse::from((reply, image_data, Some(tool_called))))
    }
}
