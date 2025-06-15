use crate::ai::dto::AIResponse;
use crate::ai::gcs::GcsImageUploader;
use crate::ai::prompt::get_prompt;
use crate::ai::tools::{execute_custom_tool, get_all_custom_tools};
use crate::user_conversation::handler::UserConversations;
use base64::{engine::general_purpose, Engine as _};
use open_ai_rust_responses_by_sshift::types::{Include, ResponseItem, Tool, ToolChoice};
use open_ai_rust_responses_by_sshift::{Client as OAIClient, Model, Request, Response};
use serde_json;
use sled::Db;

#[derive(Clone)]
pub struct AI {
    openai_client: OAIClient,
    system_prompt: String,
    cloud: GcsImageUploader,
}

impl AI {
    pub fn new(openai_api_key: String, cloud: GcsImageUploader) -> Self {
        let system_prompt = get_prompt();

        let openai_client =
            OAIClient::new(&openai_api_key).expect("Failed to create OpenAI client");

        Self {
            openai_client,
            system_prompt,
            cloud,
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
        user_id: i64,
        input: &str,
        db: &Db,
        image_url_from_reply: Option<String>,
        user_uploaded_image_urls: Vec<String>,
    ) -> Result<AIResponse, anyhow::Error> {
        log::info!("=== GENERATE_RESPONSE START for user {} ===", user_id);
        log::info!("Input text: '{}'", input);
        log::info!("Image URL from reply: {:?}", image_url_from_reply);
        log::info!("User uploaded image URLs: {:?}", user_uploaded_image_urls);

        let user_convos = UserConversations::new(db)?;
        let previous_response_id = user_convos.get_response_id(user_id);

        let vector_store_id = user_convos.get_vector_store_id(user_id);

        // Enhanced tools: built-in tools + custom function tools
        let mut tools = vec![Tool::web_search_preview(), Tool::image_generation()];
        if let Some(vs_id) = vector_store_id.clone() {
            tools.push(Tool::file_search(vec![vs_id]));
        }
        // Add custom function tools (get_balance, withdraw_funds, etc.)
        tools.extend(get_all_custom_tools());

        let mut request_builder = Request::builder()
            .model(Model::GPT4o)
            .instructions(self.system_prompt.clone())
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
            log::info!("Adding image URL from reply: {}", url);
            image_urls.push(url);
        }
        image_urls.extend(user_uploaded_image_urls.clone());
        if !user_uploaded_image_urls.is_empty() {
            log::info!("Added {} user uploaded image URLs", user_uploaded_image_urls.len());
        }

        // Also include any previously generated images that haven't been used yet
        let cached_images = {
            let cached = user_convos.take_last_image_urls(user_id);
            log::info!("Retrieved {} cached image URLs from storage: {:?}", cached.len(), cached);
            if !cached.is_empty() {
                log::info!("CACHE CLEARED: URLs have been removed from storage after retrieval");
            }
            cached
        };
        image_urls.extend(cached_images.clone());

        log::info!("Final image_urls vector: {:?} (total: {})", image_urls, image_urls.len());

        if !image_urls.is_empty() {
            log::info!("VISION MODE: Using input_image_urls with {} images", image_urls.len());
            // New helper in v0.2.2 supports multiple images in one call
            request_builder = request_builder.input_image_urls(&image_urls);

            // Include accompanying text (if any) as instructions
            if !input.trim().is_empty() {
                let user_context = if !cached_images.is_empty() && user_uploaded_image_urls.is_empty() {
                    // Only cached images present = previously generated by AI
                    format!("CONTEXT: The following image(s) were generated by you in a previous response. They may wish to discuss the image in detail, or may simply be acknowledging it. Use your best judgment based on their message to determine the appropriate level of analysis and response. The user is now saying: '{}'.", input)
                } else {
                    // User uploaded images or mixed case - just use the input as-is
                    format!("USER MESSAGE: {}", input)
                };
                
                // Combine system prompt with user context
                let combined_instructions = format!("{}\n\n{}", self.system_prompt, user_context);
                
                log::info!("Adding contextual instructions: '{}'", user_context);
                request_builder = request_builder.instructions(combined_instructions);
            } else {
                log::info!("No text input provided with images");
            }
        } else {
            log::info!("TEXT MODE: No images present, using plain text input");
            // No images ⇒ plain text input as before
            request_builder = request_builder.input(input);
            // With no vision payload we can safely include file-search results
            request_builder = request_builder.include(vec![Include::FileSearchResults]);
            log::info!("Added FileSearchResults to request");
        }

        if let Some(prev_id) = previous_response_id.clone() {
            log::info!("Using previous response ID: {}", prev_id);
            request_builder = request_builder.previous_response_id(prev_id);
        }

        let request = request_builder.build();
        log::info!("Request built, sending to OpenAI...");
        let mut current_response: Response = self.openai_client.responses.create(request).await?;
        log::info!("Received response from OpenAI, ID: {}", current_response.id());

        // Enhanced function calling loop (following demo script pattern)
        let mut iteration = 1;
        const MAX_ITERATIONS: usize = 5; // Prevent infinite loops

        while !current_response.tool_calls().is_empty() && iteration <= MAX_ITERATIONS {
            let tool_calls = current_response.tool_calls();
            log::info!("Processing {} tool calls in iteration {}", tool_calls.len(), iteration);

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
                log::info!("Executing {} custom function calls", custom_tool_calls.len());
                // Handle parallel custom function calls
                let mut function_outputs = Vec::new();
                for tool_call in &custom_tool_calls {
                    log::info!("Executing custom tool: {}", tool_call.name);
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
                    .instructions(self.system_prompt.clone())
                    .parallel_tool_calls(true)
                    .max_output_tokens(1000)
                    .temperature(0.5)
                    .user(&format!("user-{}", user_id))
                    .store(true)
                    .build();

                current_response = self
                    .openai_client
                    .responses
                    .create(continuation_request)
                    .await?;
                log::info!("Received continuation response, ID: {}", current_response.id());
            } else {
                // No custom function calls, break the loop
                // (Built-in tools like web_search and file_search are handled automatically by OpenAI)
                log::info!("No custom function calls found, ending tool execution loop");
                break;
            }

            iteration += 1;
        }

        // Extract text and potentially image data from the final response
        let mut reply = current_response.output_text();
        let response_id = current_response.id().to_string();
        log::info!("Final response text length: {} chars", reply.len());
        log::info!("Storing response ID: {}", response_id);
        user_convos.set_response_id(user_id, &response_id)?;

        let mut image_data: Option<Vec<u8>> = None;
        for item in &current_response.output {
            if let ResponseItem::ImageGenerationCall { result, .. } = item {
                log::info!("Found image generation in response, processing...");
                // Decode the base64 string to image bytes
                match general_purpose::STANDARD.decode(result) {
                    Ok(bytes) => {
                        log::info!("Successfully decoded image bytes: {} bytes", bytes.len());
                        image_data = Some(bytes);

                        // Upload to GCS and append URL to reply
                        match self
                            .cloud
                            .upload_base64_image(result, "png", "quark/images")
                            .await
                        {
                            Ok(url) => {
                                log::info!("Successfully uploaded image to GCS: {}", url);
                                reply = format!("{}\n\nImage URL: {}", reply, url);
                                let cache_result = user_convos.set_last_image_urls(user_id, &[url.clone()]);
                                match cache_result {
                                    Ok(_) => log::info!("CACHE STORED: Image URL cached for user {}: {}", user_id, url),
                                    Err(e) => log::error!("CACHE ERROR: Failed to store image URL: {}", e),
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to upload image to GCS: {}", e);
                            }
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

        log::info!("=== GENERATE_RESPONSE END for user {} ===", user_id);
        Ok(AIResponse::from((reply, image_data)))
    }
}
