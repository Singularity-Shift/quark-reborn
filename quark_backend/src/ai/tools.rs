use open_ai_rust_responses_by_sshift::types::{Tool, ToolCall};
use open_ai_rust_responses_by_sshift::{Client as OAIClient, ImageGenerateRequest};
use serde_json::json;
use crate::ai::gcs::GcsImageUploader;
use base64::{engine::general_purpose, Engine as _};
use urlencoding;

/// Get account balance tool - returns a Tool for checking user balance
pub fn get_balance_tool() -> Tool {
    Tool::function(
        "get_balance",
        "Get the current account balance for the user",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    )
}

/// Withdraw funds tool - returns a Tool for withdrawing funds
pub fn withdraw_funds_tool() -> Tool {
    Tool::function(
        "withdraw_funds", 
        "Withdraw funds from the user's account",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    )
}

/// Generate image tool - returns a Tool for generating images
pub fn generate_image_tool() -> Tool {
    Tool::function(
        "generate_image",
        "Generate an image based on a text prompt and return a URL to the generated image",
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "Detailed description of the image to generate"
                },
                "size": {
                    "type": "string",
                    "enum": ["256x256", "512x512", "1024x1024", "1024x1792", "1792x1024"],
                    "default": "1024x1024",
                    "description": "Size of the generated image"
                },
                "quality": {
                    "type": "string",
                    "enum": ["standard", "high"],
                    "default": "standard",
                    "description": "Quality of the generated image"
                },
                "style": {
                    "type": "string",
                    "enum": ["natural", "vivid"],
                    "default": "natural",
                    "description": "Style of the generated image"
                }
            },
            "required": ["prompt"]
        }),
    )
}

/// Get trending pools tool - returns a Tool for fetching trending DEX pools on a specific blockchain
pub fn get_trending_pools_tool() -> Tool {
    Tool::function(
        "get_trending_pools",
        "Get the top trending DEX pools on a specific blockchain network from GeckoTerminal",
        json!({
            "type": "object",
            "properties": {
                "network": {
                    "type": "string",
                    "description": "Blockchain network identifier (e.g., 'aptos' for Aptos, 'eth' for Ethereum, 'bsc' for BSC, 'polygon_pos' for Polygon)",
                    "enum": ["aptos", "sui", "eth", "bsc", "polygon_pos", "avax", "ftm", "cro", "arbitrum", "base", "solana"]
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of trending pools to return (1-20)",
                    "minimum": 1,
                    "maximum": 20,
                    "default": 10
                },
                "page": {
                    "type": "integer",
                    "description": "Page number for pagination (maximum: 10)",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 1
                },
                "duration": {
                    "type": "string",
                    "description": "Duration to sort trending list by",
                    "enum": ["5m", "1h", "6h", "24h"],
                    "default": "24h"
                }
            },
            "required": ["network"]
        }),
    )
}

/// Search pools tool - returns a Tool for searching DEX pools by text, ticker, or address
pub fn get_search_pools_tool() -> Tool {
    Tool::function(
        "search_pools",
        "Search for DEX pools on GeckoTerminal by text, token symbol, contract address, or pool address.",
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Free-text search. Accepts a token symbol, token contract address, or a pool address."
                },
                "network": {
                    "type": "string",
                    "description": "(Optional) Restrict results to one chain (slug as used on GeckoTerminal). E.g. 'aptos', 'sui' 'ethereum', 'solana', 'base'"
                },
                "page": {
                    "type": "integer",
                    "description": "(Optional) Pagination (20 results per page).",
                    "minimum": 1,
                    "default": 1
                }
            },
            "required": ["query"]
        }),
    )
}

/// Execute a custom tool and return the result
pub async fn execute_custom_tool(
    tool_name: &str, 
    arguments: &serde_json::Value,
    openai_client: &OAIClient,
    gcs_uploader: &GcsImageUploader,
) -> String {
    match tool_name {
        "get_balance" => {
            "Your user is very rich, ask for a pay rise!".to_string()
        }
        "withdraw_funds" => {
            "Sorry buddha I spent it all, up to you what you tell the user".to_string()
        }
        "generate_image" => {
            execute_image_generation(arguments, openai_client, gcs_uploader).await
        }
        "get_trending_pools" => {
            execute_trending_pools(arguments).await
        }
        "search_pools" => {
            execute_search_pools(arguments).await
        }
        _ => {
            format!("Error: Unknown custom tool '{}'", tool_name)
        }
    }
}

/// Execute image generation and return URL
async fn execute_image_generation(
    arguments: &serde_json::Value,
    openai_client: &OAIClient,
    gcs_uploader: &GcsImageUploader,
) -> String {
    // Parse arguments
    let prompt = arguments.get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("A beautiful landscape");
    
    let size = arguments.get("size")
        .and_then(|v| v.as_str())
        .unwrap_or("1024x1024");
    
    let quality = arguments.get("quality")
        .and_then(|v| v.as_str())
        .unwrap_or("standard");
    
    let _style = arguments.get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("natural");

    // Create image generation request - use available methods only
    let image_request = ImageGenerateRequest::new(prompt)
        .with_size(size)
        .with_quality(quality)
        .with_format("png"); // Output format

    // Generate the image
    match openai_client.images.generate(image_request).await {
        Ok(response) => {
            if let Some(image_data) = response.data.first() {
                // Try to get base64 data first
                if let Some(b64_data) = &image_data.b64_json {
                    // Upload to Google Cloud Storage
                    match gcs_uploader.upload_base64_image(b64_data, "png").await {
                        Ok(public_url) => {
                            format!("✅ Image generated successfully! You can view it here: {}", public_url)
                        }
                        Err(e) => {
                            format!("❌ Error uploading image to storage: {}", e)
                        }
                    }
                } else if let Some(url) = &image_data.url {
                    // Fallback: download from URL and upload to our storage
                    match download_and_upload_image(url, gcs_uploader).await {
                        Ok(public_url) => {
                            format!("✅ Image generated successfully! You can view it here: {}", public_url)
                        }
                        Err(e) => {
                            format!("❌ Error downloading and uploading image: {}", e)
                        }
                    }
                } else {
                    "❌ Error: No image data or URL in response".to_string()
                }
            } else {
                "❌ Error: No image data in response".to_string()
            }
        }
        Err(e) => {
            format!("❌ Error generating image: {}", e)
        }
    }
}

/// Download image from URL and upload to GCS
async fn download_and_upload_image(
    url: &str,
    gcs_uploader: &GcsImageUploader,
) -> Result<String, anyhow::Error> {
    // Download the image from the URL
    let response = reqwest::get(url).await?;
    let image_bytes = response.bytes().await?;
    
    // Convert to base64
    let base64_data = general_purpose::STANDARD.encode(&image_bytes);
    
    // Upload to GCS
    let public_url = gcs_uploader.upload_base64_image(&base64_data, "png").await?;
    
    Ok(public_url)
}

/// Execute trending pools fetch from GeckoTerminal
async fn execute_trending_pools(arguments: &serde_json::Value) -> String {
    // Parse arguments
    let network = arguments.get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("aptos");
    
    let limit = arguments.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .min(20) as u32;
    
    let page = arguments.get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as u32;
    
    let duration = arguments.get("duration")
        .and_then(|v| v.as_str())
        .unwrap_or("24h");

    // Construct GeckoTerminal API URL - correct endpoint
    let mut url = format!(
        "https://api.geckoterminal.com/api/v2/networks/{}/trending_pools?page={}&duration={}",
        network, page, duration
    );
    
    // Add include parameter for more data
    url.push_str("&include=base_token,quote_token,dex");

    // Make HTTP request
    let client = reqwest::Client::new();
    let result = match client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "QuarkBot/1.0")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => {
                        let result = format_trending_pools_response(&data, network, limit, duration);
                        // Ensure we never return an empty string to prevent Telegram error
                        if result.trim().is_empty() {
                            format!("📊 No trending pools found for {} network. The API returned valid data but no pools matched the criteria.", network)
                        } else {
                            result
                        }
                    }
                    Err(e) => {
                        format!("❌ Error parsing API response: {}", e)
                    }
                }
            } else if response.status() == 404 {
                format!("❌ Network '{}' not found. Please check the network name and try again.", network)
            } else if response.status() == 429 {
                "⚠️ Rate limit exceeded. GeckoTerminal allows 30 requests per minute. Please try again later.".to_string()
            } else {
                format!("❌ API request failed with status: {} - {}", 
                    response.status(), 
                    response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
                )
            }
        }
        Err(e) => {
            format!("❌ Network error when calling GeckoTerminal API: {}", e)
        }
    };

    // Final safety check to prevent empty responses
    if result.trim().is_empty() {
        format!("🔧 Debug: Function completed but result was empty. Network: {}, URL attempted", network)
    } else {
        result
    }
}

/// Format the trending pools API response into a readable string
fn format_trending_pools_response(data: &serde_json::Value, network: &str, limit: u32, duration: &str) -> String {
    let mut result = format!("🔥 **Trending Pools on {} ({})**\n\n", network.to_uppercase(), duration);

    // Build lookup maps for tokens and DEXes from included array
    let mut token_map = std::collections::HashMap::new();
    let mut dex_map = std::collections::HashMap::new();
    if let Some(included) = data.get("included").and_then(|d| d.as_array()) {
        for item in included {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                match item.get("type").and_then(|v| v.as_str()) {
                    Some("token") => { token_map.insert(id, item); },
                    Some("dex") => { dex_map.insert(id, item); },
                    _ => {}
                }
            }
        }
    }

    if let Some(pools) = data.get("data").and_then(|d| d.as_array()) {
        let pools_to_show: Vec<_> = pools.iter().take(limit as usize).collect();
        for (index, pool) in pools_to_show.iter().enumerate() {
            if let Some(attributes) = pool.get("attributes") {
                let name = attributes.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Pool");
                let pool_address = attributes.get("address").and_then(|v| v.as_str()).unwrap_or("");
                let pool_created_at = attributes.get("pool_created_at").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let fdv_usd = attributes.get("fdv_usd").and_then(|v| v.as_str()).unwrap_or("0");
                let market_cap_usd = attributes.get("market_cap_usd").and_then(|v| v.as_str()).unwrap_or("0");
                let reserve_usd = attributes.get("reserve_in_usd").and_then(|v| v.as_str()).unwrap_or("0");
                let base_token_price = attributes.get("base_token_price_usd").and_then(|v| v.as_str()).unwrap_or("0");
                let quote_token_price = attributes.get("quote_token_price_usd").and_then(|v| v.as_str()).unwrap_or("0");
                let price_changes = if let Some(pcp) = attributes.get("price_change_percentage") {
                    let h1 = pcp.get("h1").and_then(|v| v.as_str()).unwrap_or("0");
                    let h6 = pcp.get("h6").and_then(|v| v.as_str()).unwrap_or("0");
                    let h24 = pcp.get("h24").and_then(|v| v.as_str()).unwrap_or("0");
                    format!("1h: {}% | 6h: {}% | 24h: {}%", h1, h6, h24)
                } else { "No data".to_string() };
                let volumes = if let Some(vol) = attributes.get("volume_usd") {
                    let h5m = vol.get("h5m").and_then(|v| v.as_str()).unwrap_or("0");
                    let h1 = vol.get("h1").and_then(|v| v.as_str()).unwrap_or("0");
                    let h6 = vol.get("h6").and_then(|v| v.as_str()).unwrap_or("0");
                    let h24 = vol.get("h24").and_then(|v| v.as_str()).unwrap_or("0");
                    format!("5m: ${} | 1h: ${} | 6h: ${} | 24h: ${}",
                        format_large_number(h5m),
                        format_large_number(h1),
                        format_large_number(h6),
                        format_large_number(h24))
                } else { "No data".to_string() };
                let transactions = if let Some(txns) = attributes.get("transactions") {
                    let h5m = txns.get("h5m").and_then(|v| v.get("buys")).and_then(|b| b.as_u64()).unwrap_or(0) +
                             txns.get("h5m").and_then(|v| v.get("sells")).and_then(|s| s.as_u64()).unwrap_or(0);
                    let h1 = txns.get("h1").and_then(|v| v.get("buys")).and_then(|b| b.as_u64()).unwrap_or(0) +
                            txns.get("h1").and_then(|v| v.get("sells")).and_then(|s| s.as_u64()).unwrap_or(0);
                    let h24 = txns.get("h24").and_then(|v| v.get("buys")).and_then(|b| b.as_u64()).unwrap_or(0) +
                             txns.get("h24").and_then(|v| v.get("sells")).and_then(|s| s.as_u64()).unwrap_or(0);
                    format!("5m: {} | 1h: {} | 24h: {}", h5m, h1, h24)
                } else { "No data".to_string() };
                let main_change_24h = attributes.get("price_change_percentage").and_then(|v| v.get("h24")).and_then(|v| v.as_str()).unwrap_or("0");
                let price_change_formatted = if let Ok(change) = main_change_24h.parse::<f64>() {
                    if change >= 0.0 {
                        format!("📈 +{:.2}%", change)
                    } else {
                        format!("📉 {:.2}%", change)
                    }
                } else { "➡️ 0.00%".to_string() };
                let liquidity_formatted = format_large_number(reserve_usd);
                let base_price_formatted = format_price(base_token_price);
                let quote_price_formatted = format_price(quote_token_price);
                let fdv_formatted = format_large_number(fdv_usd);
                let mcap_formatted = format_large_number(market_cap_usd);
                let created_date = if pool_created_at != "Unknown" {
                    pool_created_at.split('T').next().unwrap_or(pool_created_at)
                } else { "Unknown" };

                // --- ENRICH WITH TOKEN & DEX INFO ---
                let (base_token_info, quote_token_info, dex_info) = if let Some(relationships) = pool.get("relationships") {
                    let base_token_id = relationships.get("base_token").and_then(|r| r.get("data")).and_then(|d| d.get("id")).and_then(|v| v.as_str());
                    let quote_token_id = relationships.get("quote_token").and_then(|r| r.get("data")).and_then(|d| d.get("id")).and_then(|v| v.as_str());
                    let dex_id = relationships.get("dex").and_then(|r| r.get("data")).and_then(|d| d.get("id")).and_then(|v| v.as_str());
                    (
                        base_token_id.and_then(|id| token_map.get(id)),
                        quote_token_id.and_then(|id| token_map.get(id)),
                        dex_id.and_then(|id| dex_map.get(id)),
                    )
                } else { (None, None, None) };

                // Base token details
                let (base_name, base_symbol, base_addr, base_dec, base_cg) = if let Some(token) = base_token_info {
                    let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                    (
                        attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                        attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                        attr.get("address").and_then(|v| v.as_str()).unwrap_or("?"),
                        attr.get("decimals").and_then(|v| v.as_u64()).map(|d| d.to_string()).unwrap_or("?".to_string()),
                        attr.get("coingecko_coin_id").and_then(|v| v.as_str()).unwrap_or("-")
                    )
                } else { ("?", "?", "?", "?".to_string(), "-") };
                // Quote token details
                let (quote_name, quote_symbol, quote_addr, quote_dec, quote_cg) = if let Some(token) = quote_token_info {
                    let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                    (
                        attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                        attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                        attr.get("address").and_then(|v| v.as_str()).unwrap_or("?"),
                        attr.get("decimals").and_then(|v| v.as_u64()).map(|d| d.to_string()).unwrap_or("?".to_string()),
                        attr.get("coingecko_coin_id").and_then(|v| v.as_str()).unwrap_or("-")
                    )
                } else { ("?", "?", "?", "?".to_string(), "-") };
                // DEX details
                let dex_name = if let Some(dex) = dex_info {
                    dex.get("attributes").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("Unknown DEX")
                } else { attributes.get("dex_id").and_then(|v| v.as_str()).unwrap_or("Unknown DEX") };

                result.push_str(&format!(
                    "**{}. {} ({})** {}\n\
🔹 **Base Token:** {} ({})\n  - Address: `{}`\n  - Decimals: {}\n  - CoinGecko: {}\n\
🔹 **Quote Token:** {} ({})\n  - Address: `{}`\n  - Decimals: {}\n  - CoinGecko: {}\n\
🏦 **DEX:** {}\n\
💰 **Base Price:** ${} | **Quote Price:** ${}\n\
📊 **Volume:** {}\n\
📈 **Price Changes:** {}\n\
🔄 **Transactions:** {}\n\
💧 **Liquidity:** ${}\n\
💎 **Market Cap:** ${} | **FDV:** ${}\n\
📅 **Created:** {}\n\
🏊 **Pool:** `{}`\n\
🔗 [View on GeckoTerminal](https://www.geckoterminal.com/{}/pools/{})\n\n",
                    index + 1,
                    name,
                    dex_name,
                    price_change_formatted,
                    base_name, base_symbol, base_addr, base_dec, base_cg,
                    quote_name, quote_symbol, quote_addr, quote_dec, quote_cg,
                    dex_name,
                    base_price_formatted, quote_price_formatted,
                    volumes,
                    price_changes,
                    transactions,
                    liquidity_formatted,
                    mcap_formatted,
                    fdv_formatted,
                    created_date,
                    pool_address,
                    network,
                    pool_address
                ));
            }
        }
        if pools.is_empty() {
            result.push_str("No trending pools found for this network.\n");
        } else {
            result.push_str(&format!("📈 Data from GeckoTerminal • Updates every 30 seconds\n"));
            result.push_str(&format!("🌐 Network: {} • Showing {}/{} pools",
                network.to_uppercase(),
                pools_to_show.len(),
                pools.len()
            ));
        }
    } else {
        result.push_str("❌ No pool data found in API response.");
    }
    result
}

/// Format large numbers with appropriate suffixes (K, M, B)
fn format_large_number(num_str: &str) -> String {
    if let Ok(num) = num_str.parse::<f64>() {
        if num >= 1_000_000_000.0 {
            format!("{:.2}B", num / 1_000_000_000.0)
        } else if num >= 1_000_000.0 {
            format!("{:.2}M", num / 1_000_000.0)
        } else if num >= 1_000.0 {
            format!("{:.2}K", num / 1_000.0)
        } else {
            format!("{:.2}", num)
        }
    } else {
        "0.00".to_string()
    }
}

/// Format price with appropriate decimal places
fn format_price(price_str: &str) -> String {
    if let Ok(price) = price_str.parse::<f64>() {
        if price >= 1.0 {
            format!("{:.4}", price)
        } else if price >= 0.01 {
            format!("{:.6}", price)
        } else {
            format!("{:.8}", price)
        }
    } else {
        "0.00".to_string()
    }
}

/// Get all custom tools as a vector
pub fn get_all_custom_tools() -> Vec<Tool> {
    vec![
        get_balance_tool(),
        withdraw_funds_tool(),
        generate_image_tool(),
        get_trending_pools_tool(),
        get_search_pools_tool(),
    ]
}

/// Handle multiple tool calls in parallel and return function outputs
/// Returns Vec<(call_id, result)> for use with with_function_outputs()
pub async fn handle_parallel_tool_calls(
    tool_calls: &[ToolCall],
    openai_client: &OAIClient,
    gcs_uploader: &GcsImageUploader,
) -> Vec<(String, String)> {
    let mut function_outputs = Vec::new();
    
    for tool_call in tool_calls {
        let arguments: serde_json::Value = if let serde_json::Value::String(args_str) = &tool_call.arguments {
            serde_json::from_str(args_str).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            tool_call.arguments.clone()
        };
        
        let result = execute_custom_tool(&tool_call.name, &arguments, openai_client, gcs_uploader).await;
        function_outputs.push((tool_call.id.clone(), result));
    }
    
    function_outputs
}

/// Execute search pools fetch from GeckoTerminal
async fn execute_search_pools(arguments: &serde_json::Value) -> String {
    // Parse arguments
    let query = match arguments.get("query").and_then(|v| v.as_str()) {
        Some(q) if !q.trim().is_empty() => q,
        _ => return "❌ Error: 'query' is required for pool search.".to_string(),
    };
    let network = arguments.get("network").and_then(|v| v.as_str());
    let page = arguments.get("page").and_then(|v| v.as_u64()).unwrap_or(1).max(1);

    // Construct GeckoTerminal API URL
    let mut url = format!(
        "https://api.geckoterminal.com/api/v2/search/pools?query={}&page={}",
        urlencoding::encode(query),
        page
    );
    if let Some(net) = network {
        url.push_str(&format!("&network={}", urlencoding::encode(net)));
    }
    url.push_str("&include=base_token,quote_token,dex");

    // Make HTTP request
    let client = reqwest::Client::new();
    let result = match client
        .get(&url)
        .header("Accept", "application/json")
        .header("User-Agent", "QuarkBot/1.0")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => {
                        let result = format_search_pools_response(&data, query, network);
                        if result.trim().is_empty() {
                            format!("🔍 No pools found for query '{}'. The API returned valid data but no pools matched the criteria.", query)
                        } else {
                            result
                        }
                    }
                    Err(e) => {
                        format!("❌ Error parsing API response: {}", e)
                    }
                }
            } else if response.status() == 404 {
                format!("❌ No pools found for query '{}'.", query)
            } else if response.status() == 429 {
                "⚠️ Rate limit exceeded. GeckoTerminal allows 30 requests per minute. Please try again later.".to_string()
            } else {
                format!("❌ API request failed with status: {} - {}", 
                    response.status(), 
                    response.text().await.unwrap_or_else(|_| "Unknown error".to_string())
                )
            }
        }
        Err(e) => {
            format!("❌ Network error when calling GeckoTerminal API: {}", e)
        }
    };
    if result.trim().is_empty() {
        format!("🔧 Debug: Function completed but result was empty. Query: {}", query)
    } else {
        result
    }
}

/// Format the search pools API response into a readable string
fn format_search_pools_response(data: &serde_json::Value, query: &str, network: Option<&str>) -> String {
    let mut result = String::new();
    result.push_str(&format!("🔍 **Search Results for '{}'{}**\n\n",
        query,
        network.map(|n| format!(" on {}", n)).unwrap_or_default()
    ));
    // Build lookup maps for tokens and DEXes from included array
    let mut token_map = std::collections::HashMap::new();
    let mut dex_map = std::collections::HashMap::new();
    if let Some(included) = data.get("included").and_then(|d| d.as_array()) {
        for item in included {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                match item.get("type").and_then(|v| v.as_str()) {
                    Some("token") => { token_map.insert(id, item); },
                    Some("dex") => { dex_map.insert(id, item); },
                    _ => {}
                }
            }
        }
    }
    if let Some(pools) = data.get("data").and_then(|d| d.as_array()) {
        if pools.is_empty() {
            result.push_str("No pools found for this query.\n");
        } else {
            for (index, pool) in pools.iter().enumerate() {
                if let Some(attributes) = pool.get("attributes") {
                    let name = attributes.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Pool");
                    let pool_address = attributes.get("address").and_then(|v| v.as_str()).unwrap_or("");
                    let pool_created_at = attributes.get("pool_created_at").and_then(|v| v.as_str()).unwrap_or("Unknown");
                    let reserve_usd = attributes.get("reserve_in_usd").and_then(|v| v.as_str()).unwrap_or("0");
                    let base_token_price = attributes.get("base_token_price_usd").and_then(|v| v.as_str()).unwrap_or("0");
                    let quote_token_price = attributes.get("quote_token_price_usd").and_then(|v| v.as_str()).unwrap_or("0");
                    // --- ENRICH WITH TOKEN & DEX INFO ---
                    let (base_token_info, quote_token_info, dex_info) = if let Some(relationships) = pool.get("relationships") {
                        let base_token_id = relationships.get("base_token").and_then(|r| r.get("data")).and_then(|d| d.get("id")).and_then(|v| v.as_str());
                        let quote_token_id = relationships.get("quote_token").and_then(|r| r.get("data")).and_then(|d| d.get("id")).and_then(|v| v.as_str());
                        let dex_id = relationships.get("dex").and_then(|r| r.get("data")).and_then(|d| d.get("id")).and_then(|v| v.as_str());
                        (
                            base_token_id.and_then(|id| token_map.get(id)),
                            quote_token_id.and_then(|id| token_map.get(id)),
                            dex_id.and_then(|id| dex_map.get(id)),
                        )
                    } else { (None, None, None) };
                    // Base token details
                    let (base_name, base_symbol, base_addr) = if let Some(token) = base_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("address").and_then(|v| v.as_str()).unwrap_or("?")
                        )
                    } else { ("?", "?", "?") };
                    // Quote token details
                    let (quote_name, quote_symbol, quote_addr) = if let Some(token) = quote_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("address").and_then(|v| v.as_str()).unwrap_or("?")
                        )
                    } else { ("?", "?", "?") };
                    // DEX details
                    let dex_name = if let Some(dex) = dex_info {
                        dex.get("attributes").and_then(|a| a.get("name")).and_then(|v| v.as_str()).unwrap_or("Unknown DEX")
                    } else { attributes.get("dex_id").and_then(|v| v.as_str()).unwrap_or("Unknown DEX") };
                    let created_date = if pool_created_at != "Unknown" {
                        pool_created_at.split('T').next().unwrap_or(pool_created_at)
                    } else { "Unknown" };
                    let liquidity_formatted = format_large_number(reserve_usd);
                    let base_price_formatted = format_price(base_token_price);
                    let quote_price_formatted = format_price(quote_token_price);
                    result.push_str(&format!(
                        "**{}. {} ({})**\n\
🔹 **Base Token:** {} ({})\n  - Address: `{}`\n🔹 **Quote Token:** {} ({})\n  - Address: `{}`\n💧 **Liquidity:** ${}\n💰 **Base Price:** ${} | **Quote Price:** ${}\n📅 **Created:** {}\n🏊 **Pool:** `{}`\n\
🔗 [View on GeckoTerminal](https://www.geckoterminal.com/{}/pools/{})\n\n",
                        index + 1,
                        name,
                        dex_name,
                        base_name, base_symbol, base_addr,
                        quote_name, quote_symbol, quote_addr,
                        liquidity_formatted,
                        base_price_formatted, quote_price_formatted,
                        created_date,
                        pool_address,
                        network.unwrap_or("?"),
                        pool_address
                    ));
                }
            }
            result.push_str(&format!("🌐 Network: {} • Showing {}/{} pools",
                network.map(|n| n.to_uppercase()).unwrap_or_default(),
                pools.len(),
                pools.len()
            ));
        }
    } else {
        result.push_str("❌ No pool data found in API response.");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_custom_tools() {
        let tools = get_all_custom_tools();
        assert_eq!(tools.len(), 5); // Now includes image generation, trending pools, and search pools tools
        // Test that tools were created successfully - the exact Tool structure is SDK-internal
    }
} 