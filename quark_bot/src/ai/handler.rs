use crate::ai::dto::AIResponse;
use crate::ai::gcs::GcsImageUploader;
use crate::ai::prompt::get_prompt;
use crate::ai::tools::{execute_custom_tool, get_all_custom_tools};
use crate::credentials::helpers::get_credentials;
use crate::panora::handler::Panora;
use crate::services::handler::Services;
use crate::user_conversation::handler::UserConversations;
use base64::{Engine as _, engine::general_purpose};
use open_ai_rust_responses_by_sshift::types::{
    Include, InputItem, ReasoningParams, Response, ResponseItem, Tool, ToolChoice,
};
use open_ai_rust_responses_by_sshift::{
    Client as OAIClient, FunctionCallInfo, Model, RecoveryPolicy, Request,
};
use serde_json;
use sled::{Db, Tree};
use teloxide::types::Message;

#[derive(Clone)]
pub struct AI {
    openai_client: OAIClient,
    system_prompt: String,
    cloud: GcsImageUploader,
    panora: Panora,
    service: Services,
    min_deposit: f64,
}

impl AI {
    pub fn new(
        openai_api_key: String,
        cloud: GcsImageUploader,
        panora: Panora,
        min_deposit: f64,
    ) -> Self {
        let system_prompt = get_prompt();

        // Use default recovery policy for API error handling
        // This provides automatic retry with 1 attempt for seamless experience
        let recovery_policy = RecoveryPolicy::default();
        let openai_client = OAIClient::new_with_recovery(&openai_api_key, recovery_policy)
            .expect("Failed to create OpenAI client with recovery policy");

        let service = Services::new();

        Self {
            openai_client,
            system_prompt,
            cloud,
            panora,
            service,
            min_deposit,
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
        let user = msg.from.clone();

        if user.is_none() {
            return Err(anyhow::anyhow!("User not found"));
        }

        let user = user.unwrap();
        let user_id = user.id.0 as i64;

        log::info!(
            "AI generate_response called for user {} with input: '{}'",
            user_id,
            input
        );

        let username = user.username.clone();

        if username.is_none() {
            return Err(anyhow::anyhow!("Username not found"));
        }

        let username = username.unwrap();

        let user_credentials = get_credentials(&username, tree.clone());

        if user_credentials.is_none() {
            return Err(anyhow::anyhow!("User credentials not found"));
        }

        let user_credentials = user_credentials.unwrap();

        let coin_address = self.panora.aptos.get_token_address().await?;

        let address = user_credentials.resource_account_address;

        let user_balance = self
            .panora
            .aptos
            .get_account_balance(&address, &coin_address)
            .await?;

        let token = self.panora.get_token_ai_fees().await?;

        let token_price = token.usd_price;

        if token_price.is_none() {
            return Err(anyhow::anyhow!("Token price not found"));
        }

        let token_price = token_price.unwrap();

        let token_price = token_price.parse::<f64>();

        if token_price.is_err() {
            return Err(anyhow::anyhow!("Token price not found"));
        }

        let token_price = token_price.unwrap();

        let token_decimals = token.decimals;

        if token_decimals.is_none() {
            return Err(anyhow::anyhow!("Token decimals not found"));
        }

        let token_decimals = token_decimals.unwrap();

        let min_deposit = (self.min_deposit / token_price);

        let min_deposit = (min_deposit as f64 * 10_f64.powi(token_decimals as i32)) as u64;

        if user_balance < min_deposit as i64 {
            let min_deposit_formatted = format!(
                "{:.2}",
                min_deposit as f64 / 10_f64.powi(token_decimals as i32)
            );

            let user_balance_formatted = format!(
                "{:.2}",
                user_balance as f64 / 10_f64.powi(token_decimals as i32)
            );

            return Err(anyhow::anyhow!(format!(
                "User balance is less than the minimum deposit. Please fund your account using /fund_account. Minimum deposit: {} {} (Your balance: {} {})",
                min_deposit_formatted,
                token.symbol.clone().unwrap_or("".to_string()),
                user_balance_formatted,
                token.symbol.unwrap_or("".to_string())
            )));
        }

        let user_convos = UserConversations::new(db)?;
        let previous_response_id = user_convos.get_response_id(user_id);
        let mut tool_called: Vec<FunctionCallInfo> = Vec::new();

        // Track token usage across all API calls
        let mut total_prompt_tokens = 0u32;
        let mut total_output_tokens = 0u32;
        let mut total_tokens_used = 0u32;

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

                // Extract and accumulate token usage
                if let Some(usage) = &response.usage {
                    total_prompt_tokens += usage.input_tokens;
                    total_output_tokens += usage.output_tokens;
                    total_tokens_used += usage.total_tokens;
                    log::info!(
                        "Initial API call tokens: input={}, output={}, total={}",
                        usage.input_tokens,
                        usage.output_tokens,
                        usage.total_tokens
                    );
                }

                response
            }
            Err(e) => {
                let error_msg = e.to_string();
                log::error!("OpenAI API call failed: {}", error_msg);

                // Handle vector store not found errors
                if error_msg.contains("Vector store") && error_msg.contains("not found") {
                    log::warn!(
                        "Vector store not found, clearing orphaned reference for user {}",
                        user_id
                    );
                    // Centralized cleanup
                    if let Err(clear_err) = user_convos.cleanup_orphaned_vector_store(user_id) {
                        log::error!("Failed to clean up orphaned vector store: {}", clear_err);
                    }
                    // Return a user-friendly error with suggestion to upload files
                    return Err(anyhow::anyhow!(
                        "Your document library is no longer available (vector store deleted). Please upload files again using /add_files to create a new document library."
                    ));
                }

                return Err(e.into());
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

                // Extract and accumulate token usage from continuation
                if let Some(usage) = &current_response.usage {
                    total_prompt_tokens += usage.input_tokens;
                    total_output_tokens += usage.output_tokens;
                    total_tokens_used += usage.total_tokens;
                    log::info!(
                        "Continuation API call tokens: input={}, output={}, total={}",
                        usage.input_tokens,
                        usage.output_tokens,
                        usage.total_tokens
                    );
                }
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

        // Save response ID for future conversation context
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
        }

        log::info!(
            "Total conversation tokens: input={}, output={}, total={}",
            total_prompt_tokens,
            total_output_tokens,
            total_tokens_used
        );

        // Calculate tool usage from the final response
        let (web_search_count, file_search_count, image_generation_count, code_interpreter_count) =
            AIResponse::calculate_tool_usage(&current_response);

        log::info!(
            "Tool usage: web_search={}, file_search={}, image_generation={}, code_interpreter={}",
            web_search_count,
            file_search_count,
            image_generation_count,
            code_interpreter_count
        );

        Ok(AIResponse::from((
            reply,
            model,
            image_data,
            Some(tool_called),
            total_tokens_used,
            web_search_count,
            file_search_count,
            image_generation_count,
            code_interpreter_count,
        )))
    }
}
