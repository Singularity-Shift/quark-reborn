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
use open_ai_rust_responses_by_sshift::types::{Include, InputItem, Response, ResponseItem, Tool, ToolChoice, ReasoningParams};
use open_ai_rust_responses_by_sshift::{Client as OAIClient, Model, Request};
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

        let openai_client =
            OAIClient::new(&openai_api_key).expect("Failed to create OpenAI client");

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
        let user_convos = UserConversations::new(db)?;
        let previous_response_id = user_convos.get_response_id(user_id);

        let vector_store_id = user_convos.get_vector_store_id(user_id);

        // Enhanced tools: built-in tools + custom function tools
        let mut tools = vec![Tool::image_generation()];
        if model != Model::O3 {
            tools.push(Tool::web_search_preview());
        }
        
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
            // Add all images to the content block
            for url in image_urls {
                content.push(InputItem::content_image(&url));
            }
            // Add the text prompt to the content block
            if !input.trim().is_empty() {
                content.push(InputItem::content_text(input));
            }
            // Manually construct the message with multiple content items
            request_builder = request_builder.input_items(vec![InputItem::message("user", content)]);
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
        let mut current_response: Response = self.openai_client.responses.create(request).await?;

        // Enhanced function calling loop (following demo script pattern)
        let mut iteration = 1;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops

        while !current_response.tool_calls().is_empty() && iteration <= MAX_ITERATIONS {
            let tool_calls = current_response.tool_calls();

            // Filter for custom function calls (get_balance, withdraw_funds, get_trending_pools, search_pools, get_current_time, get_fear_and_greed_index)
            let custom_tool_calls: Vec<_> = tool_calls
                .iter()
                .filter(|tc| {
                    tc.name == "get_balance"
                        || tc.name == "withdraw_funds"
                        || tc.name == "get_trending_pools"
                        || tc.name == "search_pools"
                        || tc.name == "get_new_pools"
                        || tc.name == "get_current_time"
                        || tc.name == "get_fear_and_greed_index"
                })
                .collect();

            if !custom_tool_calls.is_empty() {
                // Handle parallel custom function calls
                let mut function_outputs = Vec::new();
                for tool_call in &custom_tool_calls {
                    // Parse arguments as JSON Value for execute_custom_tool
                    let args_value: serde_json::Value = serde_json::from_str(&tool_call.arguments)
                        .unwrap_or_else(|_| serde_json::json!({}));
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
                    function_outputs.push((tool_call.call_id.clone(), result));
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

                current_response = self
                    .openai_client
                    .responses
                    .create(continuation_request)
                    .await?;
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
        }

        Ok(AIResponse::from((reply, image_data)))
    }
}
