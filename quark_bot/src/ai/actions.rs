use std::env;

use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use quark_core::helpers::dto::{PayUsersRequest, PayUsersVersion};
use sled::Tree;
use teloxide::types::Message;

use crate::{
    credentials::helpers::get_credentials, panora::handler::Panora, services::handler::Services,
};

/// Execute trending pools fetch from GeckoTerminal
pub async fn execute_trending_pools(arguments: &serde_json::Value) -> String {
    // Parse arguments
    let network = arguments
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("aptos");

    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .min(20) as u32;

    let page = arguments
        .get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as u32;

    let duration = arguments
        .get("duration")
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
                        let result =
                            format_trending_pools_response(&data, network, limit, duration);
                        // Ensure we never return an empty string to prevent Telegram error
                        if result.trim().is_empty() {
                            format!(
                                "📊 No trending pools found for {} network. The API returned valid data but no pools matched the criteria.",
                                network
                            )
                        } else {
                            result
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse trending pools API response: {}", e);
                        format!("❌ Error parsing API response: {}", e)
                    }
                }
            } else if response.status() == 404 {
                log::error!("Network '{}' not found in trending pools API", network);
                format!(
                    "❌ Network '{}' not found. Please check the network name and try again.",
                    network
                )
            } else if response.status() == 429 {
                log::error!("Rate limit exceeded for trending pools API");
                "⚠️ Rate limit exceeded. GeckoTerminal allows 30 requests per minute. Please try again later.".to_string()
            } else {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                log::error!(
                    "Trending pools API request failed with status: {} - {}",
                    status,
                    error_text
                );
                format!(
                    "❌ API request failed with status: {} - {}",
                    status, error_text
                )
            }
        }
        Err(e) => {
            log::error!(
                "Network error when calling trending pools GeckoTerminal API: {}",
                e
            );
            format!("❌ Network error when calling GeckoTerminal API: {}", e)
        }
    };

    // Final safety check to prevent empty responses
    if result.trim().is_empty() {
        format!(
            "🔧 Debug: Function completed but result was empty. Network: {}, URL attempted",
            network
        )
    } else {
        result
    }
}

/// Format the trending pools API response into a readable string
fn format_trending_pools_response(
    data: &serde_json::Value,
    network: &str,
    limit: u32,
    duration: &str,
) -> String {
    let mut result = format!(
        "🔥 **Trending Pools on {} ({})**\n\n",
        network.to_uppercase(),
        duration
    );

    // Build lookup maps for tokens and DEXes from included array
    let mut token_map = std::collections::HashMap::new();
    let mut dex_map = std::collections::HashMap::new();
    if let Some(included) = data.get("included").and_then(|d| d.as_array()) {
        for item in included {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                match item.get("type").and_then(|v| v.as_str()) {
                    Some("token") => {
                        token_map.insert(id, item);
                    }
                    Some("dex") => {
                        dex_map.insert(id, item);
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(pools) = data.get("data").and_then(|d| d.as_array()) {
        let pools_to_show: Vec<_> = pools.iter().take(limit as usize).collect();
        for (index, pool) in pools_to_show.iter().enumerate() {
            if let Some(attributes) = pool.get("attributes") {
                let name = attributes
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Pool");
                let pool_address = attributes
                    .get("address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let pool_created_at = attributes
                    .get("pool_created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let fdv_usd = attributes
                    .get("fdv_usd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let market_cap_usd = attributes
                    .get("market_cap_usd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let reserve_usd = attributes
                    .get("reserve_in_usd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let base_token_price = attributes
                    .get("base_token_price_usd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let quote_token_price = attributes
                    .get("quote_token_price_usd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let price_changes = if let Some(pcp) = attributes.get("price_change_percentage") {
                    let h1 = pcp.get("h1").and_then(|v| v.as_str()).unwrap_or("0");
                    let h6 = pcp.get("h6").and_then(|v| v.as_str()).unwrap_or("0");
                    let h24 = pcp.get("h24").and_then(|v| v.as_str()).unwrap_or("0");
                    format!("1h: {}% | 6h: {}% | 24h: {}%", h1, h6, h24)
                } else {
                    "No data".to_string()
                };
                let volumes = if let Some(vol) = attributes.get("volume_usd") {
                    let h5m = vol.get("h5m").and_then(|v| v.as_str()).unwrap_or("0");
                    let h1 = vol.get("h1").and_then(|v| v.as_str()).unwrap_or("0");
                    let h6 = vol.get("h6").and_then(|v| v.as_str()).unwrap_or("0");
                    let h24 = vol.get("h24").and_then(|v| v.as_str()).unwrap_or("0");
                    format!(
                        "5m: ${} | 1h: ${} | 6h: ${} | 24h: ${}",
                        format_large_number(h5m),
                        format_large_number(h1),
                        format_large_number(h6),
                        format_large_number(h24)
                    )
                } else {
                    "No data".to_string()
                };
                let transactions = if let Some(txns) = attributes.get("transactions") {
                    let h5m = txns
                        .get("h5m")
                        .and_then(|v| v.get("buys"))
                        .and_then(|b| b.as_u64())
                        .unwrap_or(0)
                        + txns
                            .get("h5m")
                            .and_then(|v| v.get("sells"))
                            .and_then(|s| s.as_u64())
                            .unwrap_or(0);
                    let h1 = txns
                        .get("h1")
                        .and_then(|v| v.get("buys"))
                        .and_then(|b| b.as_u64())
                        .unwrap_or(0)
                        + txns
                            .get("h1")
                            .and_then(|v| v.get("sells"))
                            .and_then(|s| s.as_u64())
                            .unwrap_or(0);
                    let h24 = txns
                        .get("h24")
                        .and_then(|v| v.get("buys"))
                        .and_then(|b| b.as_u64())
                        .unwrap_or(0)
                        + txns
                            .get("h24")
                            .and_then(|v| v.get("sells"))
                            .and_then(|s| s.as_u64())
                            .unwrap_or(0);
                    format!("5m: {} | 1h: {} | 24h: {}", h5m, h1, h24)
                } else {
                    "No data".to_string()
                };
                let main_change_24h = attributes
                    .get("price_change_percentage")
                    .and_then(|v| v.get("h24"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let price_change_formatted = if let Ok(change) = main_change_24h.parse::<f64>() {
                    if change >= 0.0 {
                        format!("📈 +{:.2}%", change)
                    } else {
                        format!("📉 {:.2}%", change)
                    }
                } else {
                    "➡️ 0.00%".to_string()
                };
                let liquidity_formatted = format_large_number(reserve_usd);
                let base_price_formatted = format_price(base_token_price);
                let quote_price_formatted = format_price(quote_token_price);
                let fdv_formatted = format_large_number(fdv_usd);
                let mcap_formatted = format_large_number(market_cap_usd);
                let created_date = if pool_created_at != "Unknown" {
                    pool_created_at.split('T').next().unwrap_or(pool_created_at)
                } else {
                    "Unknown"
                };

                // --- ENRICH WITH TOKEN & DEX INFO ---
                let (base_token_info, quote_token_info, dex_info) =
                    if let Some(relationships) = pool.get("relationships") {
                        let base_token_id = relationships
                            .get("base_token")
                            .and_then(|r| r.get("data"))
                            .and_then(|d| d.get("id"))
                            .and_then(|v| v.as_str());
                        let quote_token_id = relationships
                            .get("quote_token")
                            .and_then(|r| r.get("data"))
                            .and_then(|d| d.get("id"))
                            .and_then(|v| v.as_str());
                        let dex_id = relationships
                            .get("dex")
                            .and_then(|r| r.get("data"))
                            .and_then(|d| d.get("id"))
                            .and_then(|v| v.as_str());
                        (
                            base_token_id.and_then(|id| token_map.get(id)),
                            quote_token_id.and_then(|id| token_map.get(id)),
                            dex_id.and_then(|id| dex_map.get(id)),
                        )
                    } else {
                        (None, None, None)
                    };

                // Base token details
                let (base_name, base_symbol, base_addr, base_dec, base_cg) =
                    if let Some(token) = base_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("address").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("decimals")
                                .and_then(|v| v.as_u64())
                                .map(|d| d.to_string())
                                .unwrap_or("?".to_string()),
                            attr.get("coingecko_coin_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-"),
                        )
                    } else {
                        ("?", "?", "?", "?".to_string(), "-")
                    };
                // Quote token details
                let (quote_name, quote_symbol, quote_addr, quote_dec, quote_cg) =
                    if let Some(token) = quote_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("address").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("decimals")
                                .and_then(|v| v.as_u64())
                                .map(|d| d.to_string())
                                .unwrap_or("?".to_string()),
                            attr.get("coingecko_coin_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("-"),
                        )
                    } else {
                        ("?", "?", "?", "?".to_string(), "-")
                    };
                // DEX details
                let dex_name = if let Some(dex) = dex_info {
                    dex.get("attributes")
                        .and_then(|a| a.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown DEX")
                } else {
                    attributes
                        .get("dex_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown DEX")
                };

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
                    base_name,
                    base_symbol,
                    base_addr,
                    base_dec,
                    base_cg,
                    quote_name,
                    quote_symbol,
                    quote_addr,
                    quote_dec,
                    quote_cg,
                    dex_name,
                    base_price_formatted,
                    quote_price_formatted,
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
            result.push_str(&format!(
                "📈 Data from GeckoTerminal • Updates every 30 seconds\n"
            ));
            result.push_str(&format!(
                "🌐 Network: {} • Showing {}/{} pools",
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

/// Execute search pools fetch from GeckoTerminal
pub async fn execute_search_pools(arguments: &serde_json::Value) -> String {
    // Parse arguments
    let query = match arguments.get("query").and_then(|v| v.as_str()) {
        Some(q) if !q.trim().is_empty() => q,
        _ => {
            log::error!("Pool search called without required query parameter");
            return "❌ Error: 'query' is required for pool search.".to_string();
        }
    };
    let network = arguments.get("network").and_then(|v| v.as_str());
    let page = arguments
        .get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1);

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
                            format!(
                                "🔍 No pools found for query '{}'. The API returned valid data but no pools matched the criteria.",
                                query
                            )
                        } else {
                            result
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse search pools API response: {}", e);
                        format!("❌ Error parsing API response: {}", e)
                    }
                }
            } else if response.status() == 404 {
                log::error!("No pools found for query '{}' (404 response)", query);
                format!("❌ No pools found for query '{}'.", query)
            } else if response.status() == 429 {
                log::error!("Rate limit exceeded for search pools API");
                "⚠️ Rate limit exceeded. GeckoTerminal allows 30 requests per minute. Please try again later.".to_string()
            } else {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                log::error!(
                    "Search pools API request failed with status: {} - {}",
                    status,
                    error_text
                );
                format!(
                    "❌ API request failed with status: {} - {}",
                    status, error_text
                )
            }
        }
        Err(e) => {
            log::error!(
                "Network error when calling search pools GeckoTerminal API: {}",
                e
            );
            format!("❌ Network error when calling GeckoTerminal API: {}", e)
        }
    };
    if result.trim().is_empty() {
        format!(
            "🔧 Debug: Function completed but result was empty. Query: {}",
            query
        )
    } else {
        result
    }
}

/// Format the search pools API response into a readable string
fn format_search_pools_response(
    data: &serde_json::Value,
    query: &str,
    network: Option<&str>,
) -> String {
    let mut result = String::new();
    result.push_str(&format!(
        "🔍 **Search Results for '{}'{}**\n\n",
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
                    Some("token") => {
                        token_map.insert(id, item);
                    }
                    Some("dex") => {
                        dex_map.insert(id, item);
                    }
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
                    let name = attributes
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Pool");
                    let pool_address = attributes
                        .get("address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let pool_created_at = attributes
                        .get("pool_created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let reserve_usd = attributes
                        .get("reserve_in_usd")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    let base_token_price = attributes
                        .get("base_token_price_usd")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    let quote_token_price = attributes
                        .get("quote_token_price_usd")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    // --- ENRICH WITH TOKEN & DEX INFO ---
                    let (base_token_info, quote_token_info, dex_info) =
                        if let Some(relationships) = pool.get("relationships") {
                            let base_token_id = relationships
                                .get("base_token")
                                .and_then(|r| r.get("data"))
                                .and_then(|d| d.get("id"))
                                .and_then(|v| v.as_str());
                            let quote_token_id = relationships
                                .get("quote_token")
                                .and_then(|r| r.get("data"))
                                .and_then(|d| d.get("id"))
                                .and_then(|v| v.as_str());
                            let dex_id = relationships
                                .get("dex")
                                .and_then(|r| r.get("data"))
                                .and_then(|d| d.get("id"))
                                .and_then(|v| v.as_str());
                            (
                                base_token_id.and_then(|id| token_map.get(id)),
                                quote_token_id.and_then(|id| token_map.get(id)),
                                dex_id.and_then(|id| dex_map.get(id)),
                            )
                        } else {
                            (None, None, None)
                        };
                    // Base token details
                    let (base_name, base_symbol, base_addr) = if let Some(token) = base_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("address").and_then(|v| v.as_str()).unwrap_or("?"),
                        )
                    } else {
                        ("?", "?", "?")
                    };
                    // Quote token details
                    let (quote_name, quote_symbol, quote_addr) =
                        if let Some(token) = quote_token_info {
                            let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                            (
                                attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                                attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                                attr.get("address").and_then(|v| v.as_str()).unwrap_or("?"),
                            )
                        } else {
                            ("?", "?", "?")
                        };
                    // DEX details
                    let dex_name = if let Some(dex) = dex_info {
                        dex.get("attributes")
                            .and_then(|a| a.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown DEX")
                    } else {
                        attributes
                            .get("dex_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown DEX")
                    };
                    let created_date = if pool_created_at != "Unknown" {
                        pool_created_at.split('T').next().unwrap_or(pool_created_at)
                    } else {
                        "Unknown"
                    };
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
            result.push_str(&format!(
                "🌐 Network: {} • Showing {}/{} pools",
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

/// Execute new pools fetch from GeckoTerminal
pub async fn execute_new_pools(arguments: &serde_json::Value) -> String {
    // Parse arguments
    let network = arguments
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("aptos");

    let page = arguments
        .get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(10) as u32;

    // Construct GeckoTerminal API URL
    let mut url = format!(
        "https://api.geckoterminal.com/api/v2/networks/{}/new_pools?page={}",
        network, page
    );
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
                        let result = format_new_pools_response(&data, network);
                        // Ensure we never return an empty string to prevent Telegram error
                        if result.trim().is_empty() {
                            format!(
                                "✨ No new pools found for {} network. The API returned valid data but no pools matched the criteria.",
                                network
                            )
                        } else {
                            result
                        }
                    }
                    Err(e) => {
                        format!("❌ Error parsing API response: {}", e)
                    }
                }
            } else if response.status() == 404 {
                format!(
                    "❌ Network '{}' not found. Please check the network name and try again.",
                    network
                )
            } else if response.status() == 429 {
                "⚠️ Rate limit exceeded. GeckoTerminal allows 30 requests per minute. Please try again later.".to_string()
            } else {
                format!(
                    "❌ API request failed with status: {} - {}",
                    response.status(),
                    response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string())
                )
            }
        }
        Err(e) => {
            format!("❌ Network error when calling GeckoTerminal API: {}", e)
        }
    };

    // Final safety check to prevent empty responses
    if result.trim().is_empty() {
        format!(
            "🔧 Debug: Function completed but result was empty. Network: {}, URL attempted: {}",
            network, url
        )
    } else {
        result
    }
}

/// Format the new pools API response into a readable string
fn format_new_pools_response(data: &serde_json::Value, network: &str) -> String {
    let mut result = format!("✨ **Newest Pools on {}**\n\n", network.to_uppercase());

    // Build lookup maps for tokens and DEXes from included array
    let mut token_map = std::collections::HashMap::new();
    let mut dex_map = std::collections::HashMap::new();
    if let Some(included) = data.get("included").and_then(|d| d.as_array()) {
        for item in included {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                match item.get("type").and_then(|v| v.as_str()) {
                    Some("token") => {
                        token_map.insert(id, item);
                    }
                    Some("dex") => {
                        dex_map.insert(id, item);
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(pools) = data.get("data").and_then(|d| d.as_array()) {
        if pools.is_empty() {
            result.push_str("No new pools found for this network.\n");
        } else {
            for (index, pool) in pools.iter().enumerate() {
                if let Some(attributes) = pool.get("attributes") {
                    let name = attributes
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown Pool");
                    let pool_address = attributes
                        .get("address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let pool_created_at = attributes
                        .get("pool_created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let reserve_usd = attributes
                        .get("reserve_in_usd")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    let base_token_price = attributes
                        .get("base_token_price_usd")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");

                    let main_change_24h = attributes
                        .get("price_change_percentage")
                        .and_then(|v| v.get("h24"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    let price_change_formatted = if let Ok(change) = main_change_24h.parse::<f64>()
                    {
                        if change >= 0.0 {
                            format!("📈 +{:.2}%", change)
                        } else {
                            format!("📉 {:.2}%", change)
                        }
                    } else {
                        "➡️ 0.00%".to_string()
                    };

                    let liquidity_formatted = format_large_number(reserve_usd);
                    let base_price_formatted = format_price(base_token_price);

                    let created_date = if pool_created_at != "Unknown" {
                        pool_created_at.split('T').next().unwrap_or(pool_created_at)
                    } else {
                        "Unknown"
                    };

                    // --- ENRICH WITH TOKEN & DEX INFO ---
                    let (base_token_info, quote_token_info, dex_info) =
                        if let Some(relationships) = pool.get("relationships") {
                            let base_token_id = relationships
                                .get("base_token")
                                .and_then(|r| r.get("data"))
                                .and_then(|d| d.get("id"))
                                .and_then(|v| v.as_str());
                            let quote_token_id = relationships
                                .get("quote_token")
                                .and_then(|r| r.get("data"))
                                .and_then(|d| d.get("id"))
                                .and_then(|v| v.as_str());
                            let dex_id = relationships
                                .get("dex")
                                .and_then(|r| r.get("data"))
                                .and_then(|d| d.get("id"))
                                .and_then(|v| v.as_str());
                            (
                                base_token_id.and_then(|id| token_map.get(id)),
                                quote_token_id.and_then(|id| token_map.get(id)),
                                dex_id.and_then(|id| dex_map.get(id)),
                            )
                        } else {
                            (None, None, None)
                        };

                    let (_, base_symbol) = if let Some(token) = base_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                        )
                    } else {
                        ("?", "?")
                    };
                    let (_, quote_symbol) = if let Some(token) = quote_token_info {
                        let attr = token.get("attributes").unwrap_or(&serde_json::Value::Null);
                        (
                            attr.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                            attr.get("symbol").and_then(|v| v.as_str()).unwrap_or("?"),
                        )
                    } else {
                        ("?", "?")
                    };
                    let dex_name = if let Some(dex) = dex_info {
                        dex.get("attributes")
                            .and_then(|a| a.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown DEX")
                    } else {
                        attributes
                            .get("dex_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown DEX")
                    };

                    result.push_str(&format!(
                        "**{}. {} ({})** {}\n\
🔹 **Pair:** {} / {}\n\
🏦 **DEX:** {}\n\
💰 **Price:** ${}\n\
💧 **Liquidity:** ${}\n\
📅 **Created:** {}\n\
🔗 [View on GeckoTerminal](https://www.geckoterminal.com/{}/pools/{})\n\n",
                        index + 1,
                        name,
                        dex_name,
                        price_change_formatted,
                        base_symbol,
                        quote_symbol,
                        dex_name,
                        base_price_formatted,
                        liquidity_formatted,
                        created_date,
                        network,
                        pool_address
                    ));
                }
            }
            result.push_str(&format!(
                "📈 Data from GeckoTerminal • Showing {}/{} pools",
                pools.len(),
                pools.len()
            ));
        }
    } else {
        result.push_str("❌ No pool data found in API response.");
    }
    result
}

/// Execute get time fetch from WorldTimeAPI
pub async fn execute_get_time(arguments: &serde_json::Value) -> String {
    let timezone = arguments
        .get("timezone")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Europe/London");

    // Use TIME_API_BASE_URL from env if set, otherwise default to just the base
    let base_url =
        std::env::var("TIME_API_BASE_URL").unwrap_or_else(|_| "https://timeapi.io/api".to_string());
    let url = format!("{}/Time/current/zone?timeZone={}", base_url, timezone);

    let client = reqwest::Client::new();
    match client
        .get(&url)
        .header("User-Agent", "QuarkBot/1.0")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => format_time_response_timeapi(&data),
                    Err(e) => {
                        log::error!("Failed to parse time API response: {}", e);
                        format!("❌ Error parsing time API response: {}", e)
                    }
                }
            } else {
                format!(
                    "❌ Error fetching time for timezone '{}'. Please check the timezone name (e.g., 'Europe/London').",
                    timezone
                )
            }
        }
        Err(e) => {
            log::error!("Network error when calling timeapi.io: {}", e);
            format!("❌ Network error when calling timeapi.io: {}", e)
        }
    }
}

/// Helper for formatting timeapi.io response
fn format_time_response_timeapi(data: &serde_json::Value) -> String {
    let timezone = data
        .get("timeZone")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let date = data.get("date").and_then(|v| v.as_str()).unwrap_or("");
    let time = data.get("time").and_then(|v| v.as_str()).unwrap_or("");
    let day_of_week = data.get("dayOfWeek").and_then(|v| v.as_str()).unwrap_or("");
    let dst_active = data
        .get("dstActive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if time.is_empty() {
        log::error!("Could not extract time from timeapi.io response");
        return "❌ Could not extract the time from the API response.".to_string();
    }

    format!(
        "🕰️ The current time in **{}** is **{}** on **{}** (Date: {}, DST: {}).",
        timezone,
        time,
        day_of_week,
        date,
        if dst_active { "active" } else { "inactive" }
    )
}

/// Execute Fear & Greed Index fetch from Alternative.me
pub async fn execute_fear_and_greed_index(arguments: &serde_json::Value) -> String {
    let limit = arguments.get("days").and_then(|v| v.as_u64()).unwrap_or(1);

    // Use date_format=world to get DD-MM-YYYY dates instead of unix timestamps
    let url = format!(
        "https://api.alternative.me/fng/?limit={}&date_format=world",
        limit
    );

    let client = reqwest::Client::new();
    match client
        .get(&url)
        .header("User-Agent", "QuarkBot/1.0")
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => format_fear_and_greed_response(&data),
                    Err(e) => {
                        log::error!("Failed to parse Fear & Greed API response: {}", e);
                        format!("❌ Error parsing Fear & Greed API response: {}", e)
                    }
                }
            } else {
                format!(
                    "❌ Error fetching Fear & Greed Index. Status: {}",
                    response.status()
                )
            }
        }
        Err(e) => {
            log::error!("Network error when calling Fear & Greed API: {}", e);
            format!("❌ Network error when calling Fear & Greed API: {}", e)
        }
    }
}

/// Format the Fear & Greed Index API response into a readable string
fn format_fear_and_greed_response(data: &serde_json::Value) -> String {
    if let Some(index_data_array) = data.get("data").and_then(|d| d.as_array()) {
        if index_data_array.is_empty() {
            log::error!("No Fear & Greed Index data found in API response");
            return "❌ No Fear & Greed Index data could be found.".to_string();
        }

        // Handle single-day response (latest)
        if index_data_array.len() == 1 {
            let index_data = &index_data_array[0];
            let value = index_data
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A");
            let classification = index_data
                .get("value_classification")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let time_until_update = index_data
                .get("time_until_update")
                .and_then(|v| v.as_str())
                .unwrap_or("0");

            let emoji = match classification {
                "Extreme Fear" => "😨",
                "Fear" => "😟",
                "Neutral" => "😐",
                "Greed" => "😊",
                "Extreme Greed" => "🤑",
                _ => "📊",
            };

            let hours_until_update = time_until_update.parse::<f64>().unwrap_or(0.0) / 3600.0;

            return format!(
                "**Crypto Market Sentiment: Fear & Greed Index**\n\n\
                {} **{} - {}**\n\n\
                The current sentiment in the crypto market is **{}**.\n\
                *Next update in {:.1} hours.*",
                emoji, value, classification, classification, hours_until_update
            );
        } else {
            // Handle historical data response
            let mut result = format!(
                "**Fear & Greed Index - Last {} Days**\n\n",
                index_data_array.len()
            );
            for item in index_data_array {
                let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("N/A");
                let classification = item
                    .get("value_classification")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let date_str = item
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Date");

                let emoji = match classification {
                    "Extreme Fear" => "😨",
                    "Fear" => "😟",
                    "Neutral" => "😐",
                    "Greed" => "😊",
                    "Extreme Greed" => "🤑",
                    _ => "📊",
                };

                result.push_str(&format!(
                    "{} **{}**: {} ({})\n",
                    emoji, date_str, value, classification
                ));
            }
            return result;
        }
    } else {
        log::error!("❌ Could not retrieve Fear & Greed Index data from the API response");
        "❌ Could not retrieve Fear & Greed Index data from the API response.".to_string()
    }
}

pub async fn execute_pay_users(
    arguments: &serde_json::Value,
    msg: Message,
    services: Services,
    tree: Tree,
    panora: Panora,
) -> String {
    let mut version = PayUsersVersion::V1;

    let user = msg.from;

    if user.is_none() {
        log::error!("❌ User not found");
        return "❌ User not found".to_string();
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        log::error!("❌ Username not found");
        return "❌ Username not found".to_string();
    }

    let username = username.unwrap();

    let amount = arguments
        .get("amount")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let symbol = arguments
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("APT");
    let empty_vec = Vec::new();
    let users_array = arguments
        .get("users")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_vec);

    let users = users_array
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();

    let is_emojicoin = arguments
        .get("is_emojicoin")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_native = arguments
        .get("is_native")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_meme = arguments
        .get("is_meme")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_bridged = arguments
        .get("is_bridged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let (token_type, decimals) =
        if symbol.to_lowercase() == "apt" || symbol.to_lowercase() == "aptos" {
            version = PayUsersVersion::V1;
            ("0x1::aptos_coin::AptosCoin".to_string(), 8u8)
        } else {
            let tokens = panora
                .get_panora_token_list(is_emojicoin, is_native, is_meme, is_bridged)
                .await;

            if tokens.is_err() {
                log::error!(
                    "❌ Error getting tokens: {}",
                    tokens.as_ref().err().unwrap()
                );
                return format!("❌ Error getting tokens: {}", tokens.err().unwrap());
            }

            let tokens = tokens.unwrap();

            let token = tokens
                .iter()
                .find(|t| t.panora_symbol.to_lowercase() == symbol.to_lowercase() && !t.is_banned);

            if token.is_none() {
                log::error!("❌ Token not found: {}", symbol);
                return format!("❌ Token not found: {}", symbol);
            }

            let token = token.unwrap();

            let token_type_result = if token.token_address.as_ref().is_some() {
                token.token_address.as_ref().unwrap().to_string()
            } else {
                token.fa_address.clone()
            };

            (token_type_result, token.decimals)
        };

    // Convert amount to blockchain format using token decima
    let blockchain_amount = (amount as f64 * 10_f64.powi(decimals as i32)) as u64;

    let user_addresses = users
        .iter()
        .map(|u| {
            let user_data = get_credentials(u.as_str(), tree.clone());

            if user_data.is_none() {
                log::error!("❌ User not found");
                return None;
            }

            user_data
        })
        .filter(|u| u.is_some())
        .map(|u| u.unwrap().resource_account_address)
        .collect::<Vec<_>>();

    if user_addresses.is_empty() {
        log::error!("❌ No users found");
        return "❌ No users found".to_string();
    }

    let user_credentials = get_credentials(&username, tree.clone());

    if user_credentials.is_none() {
        log::error!("❌ User not found");
        return "❌ User not found".to_string();
    }

    let user_credentials = user_credentials.unwrap();

    let result = services
        .pay_users(
            user_credentials.jwt,
            PayUsersRequest {
                amount: blockchain_amount,
                users: user_addresses,
                coin_type: token_type,
                version,
            },
        )
        .await;

    if result.is_err() {
        log::error!(
            "❌ Error sending payments: {}",
            result.as_ref().err().unwrap()
        );
        return format!("❌ Error sending payments: {}", result.err().unwrap());
    }

    let result = result.unwrap();

    let network = env::var("APTOS_NETWORK")
        .unwrap_or("mainnet".to_string())
        .to_lowercase();

    format!(
        "Payments sent successfully: https://explorer.aptoslabs.com/txn/{}?network={}",
        result.hash, network
    )
}

pub async fn execute_get_wallet_address(msg: Message, tree: Tree) -> String {
    let user = msg.from;

    if user.is_none() {
        log::error!("❌ User not found");
        return "Unable to retrieve wallet address: User not found. Please try again.".to_string();
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        log::error!("❌ Username not found");
        return "Unable to retrieve wallet address: You need to set a Telegram username first. Please go to Telegram Settings and set a username, then try again.".to_string();
    }

    let username = username.unwrap();

    let user_credentials = get_credentials(&username, tree.clone());

    if user_credentials.is_none() {
        log::error!("❌ User credentials not found for username: {}", username);
        return "Unable to retrieve wallet address: No wallet found for your account. Please create a wallet first using the create wallet tool.".to_string();
    }

    let user_credentials = user_credentials.unwrap();

    let wallet_address = user_credentials.account_address;

    log::info!("✅ Successfully retrieved wallet address for user: {}", username);
    
    // Format the wallet address as an HTML code block for better presentation
    format!("Your wallet address is:\n\n<code>{}</code>", wallet_address)
}

pub async fn execute_get_balance(
    arguments: &serde_json::Value,
    msg: Message,
    tree: Tree,
    node: AptosFullnodeClient,
    panora: Panora,
) -> String {
    let user = msg.from;

    if user.is_none() {
        return "❌ User not found".to_string();
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        log::error!("❌ Username not found");
        return "❌ Username not found".to_string();
    }

    let username = username.unwrap();

    let user_credentials = get_credentials(&username, tree.clone());

    if user_credentials.is_none() {
        log::error!("❌ User not found");
        return "❌ User not found".to_string();
    }

    let symbol = arguments
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("APT");

    let (token_type, decimals, token_symbol) =
        if symbol.to_lowercase() == "apt" || symbol.to_lowercase() == "aptos" {
            (
                "0x1::aptos_coin::AptosCoin".to_string(),
                8u8,
                "APT".to_string(),
            )
        } else {
            let tokens = panora
                .get_panora_token_list(false, false, false, false)
                .await;

            if tokens.is_err() {
                log::error!(
                    "❌ Error getting tokens: {}",
                    tokens.as_ref().err().unwrap()
                );
                return format!("❌ Error getting tokens: {}", tokens.err().unwrap());
            }

            let tokens = tokens.unwrap();

            let token = tokens
                .iter()
                .find(|t| t.panora_symbol.to_lowercase() == symbol.to_lowercase() && !t.is_banned);

            if token.is_none() {
                log::error!("❌ Token not found: {}", symbol);
                return format!("❌ Token not found: {}", symbol);
            }

            let token = token.unwrap();
            println!("token: {:?}", token);

            let token_type = if token.token_address.as_ref().is_some() {
                token.token_address.as_ref().unwrap().to_string()
            } else {
                token.fa_address.clone()
            };

            (token_type, token.decimals, token.symbol.clone())
        };

    let user_credentials = user_credentials.unwrap();

    let balance = node
        .get_account_balance(
            user_credentials.resource_account_address,
            token_type.to_string(),
        )
        .await;

    if balance.is_err() {
        log::error!(
            "❌ Error getting balance: {}",
            balance.as_ref().err().unwrap()
        );
        return format!("❌ Error getting balance: {}", balance.err().unwrap());
    }

    let raw_balance = balance.unwrap().into_inner();

    let balance_i64 = raw_balance.as_i64();

    if balance_i64.is_none() {
        log::error!("❌ Balance not found");
        return "❌ Balance not found".to_string();
    }

    let raw_balance = balance_i64.unwrap();

    // Convert raw balance to human readable format using decimals
    let human_balance = raw_balance as f64 / 10_f64.powi(decimals as i32);

    println!(
        "Raw balance: {}, Human balance: {}",
        raw_balance, human_balance
    );

    format!("💰 **Balance**: {:.6} {}", human_balance, token_symbol)
}

pub async fn execute_withdraw_funds(
    arguments: &serde_json::Value,
    msg: Message,
    tree: Tree,
    node: AptosFullnodeClient,
    panora: Panora,
) -> String {
    let app_url = env::var("APP_URL");

    if app_url.is_err() {
        return "❌ APP_URL not found".to_string();
    }

    let app_url = app_url.unwrap();

    let chat = msg.chat;

    if chat.is_group() || chat.is_supergroup() || !chat.is_private() || chat.is_channel() {
        return "❌ This command is only available in private chats".to_string();
    }

    let user = msg.from;

    if user.is_none() {
        return "❌ User not found".to_string();
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        return "❌ Username not found".to_string();
    }

    let username = username.unwrap();

    let user_credentials = get_credentials(&username, tree.clone());

    if user_credentials.is_none() {
        return "❌ User not found".to_string();
    }

    let user_credentials = user_credentials.unwrap();

    let symbol = arguments
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("APT");

    let amount = arguments
        .get("amount")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let tokens = panora
        .get_panora_token_list(false, false, false, false)
        .await;

    if tokens.is_err() {
        return "❌ Error getting token type".to_string();
    }

    let tokens = tokens.unwrap();

    let token = tokens
        .iter()
        .find(|t| t.symbol.to_lowercase() == symbol.to_lowercase());

    if token.is_none() {
        return "❌ Token not found".to_string();
    }

    let token = token.unwrap();

    let token_type = if token.token_address.as_ref().is_some() {
        token.token_address.as_ref().unwrap().to_string()
    } else {
        token.fa_address.clone()
    };

    let balance = node
        .get_account_balance(
            user_credentials.resource_account_address,
            token_type.to_string(),
        )
        .await;

    if balance.is_err() {
        return "❌ Error getting balance".to_string();
    }

    let balance = balance.unwrap().into_inner();

    let balance_i64 = balance.as_i64();

    if balance_i64.is_none() {
        return "❌ Balance not found".to_string();
    }

    let balance_i64 = balance_i64.unwrap();

    if balance_i64 < amount as i64 {
        return "❌ Insufficient balance".to_string();
    }

    let url = format!("{}/withdraw?coin={}&amount={}", app_url, symbol, amount);

    url
}

pub async fn execute_fund_account(
    arguments: &serde_json::Value,
    msg: Message,
    tree: Tree,
    node: AptosFullnodeClient,
    panora: Panora,
) -> String {
    let app_url = env::var("APP_URL");

    if app_url.is_err() {
        return "❌ APP_URL not found".to_string();
    }

    let app_url = app_url.unwrap();

    let chat = msg.chat;

    if chat.is_group() || chat.is_supergroup() || !chat.is_private() || chat.is_channel() {
        return "❌ This command is only available in private chats".to_string();
    }

    let user = msg.from;

    if user.is_none() {
        return "❌ User not found".to_string();
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        return "❌ Username not found".to_string();
    }

    let username = username.unwrap();

    let user_credentials = get_credentials(&username, tree.clone());

    if user_credentials.is_none() {
        return "❌ User not found".to_string();
    }

    let user_credentials = user_credentials.unwrap();

    let symbol = arguments
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("APT");

    let amount = arguments
        .get("amount")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let tokens = panora
        .get_panora_token_list(false, false, false, false)
        .await;

    if tokens.is_err() {
        return "❌ Error getting token type".to_string();
    }

    let tokens = tokens.unwrap();

    let token = tokens
        .iter()
        .find(|t| t.symbol.to_lowercase() == symbol.to_lowercase());

    if token.is_none() {
        return "❌ Token not found".to_string();
    }

    let token = token.unwrap();

    let token_type = if token.token_address.as_ref().is_some() {
        token.token_address.as_ref().unwrap().to_string()
    } else {
        token.fa_address.clone()
    };

    // Get balance from user's main wallet (not resource account)
    let balance = node
        .get_account_balance(user_credentials.account_address, token_type.to_string())
        .await;

    if balance.is_err() {
        return "❌ Error getting balance".to_string();
    }

    let balance = balance.unwrap().into_inner();

    let balance_i64 = balance.as_i64();

    if balance_i64.is_none() {
        return "❌ Balance not found".to_string();
    }

    let balance_i64 = balance_i64.unwrap();

    if balance_i64 < amount as i64 {
        return "❌ Insufficient balance".to_string();
    }

    let url = format!("{}/fund?coin={}&amount={}", app_url, symbol, amount);

    url
}
