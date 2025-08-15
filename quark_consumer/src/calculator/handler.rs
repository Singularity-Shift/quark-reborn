use super::dto::{Price, ToolName};
use crate::error::ConsumerError;
use quark_core::helpers::dto::{AITool, CoinVersion, PriceCoin, ToolUsage};
use reqwest::Client;
use ron::de::from_str;
use std::fs;

fn ai_tool_to_tool_name(ai_tool: &AITool) -> ToolName {
    match ai_tool {
        AITool::FileSearch => ToolName::FileSearch,
        AITool::ImageGeneration => ToolName::ImageGeneration,
        AITool::WebSearchPreview => ToolName::WebSearchPreview,
    }
}

pub async fn get_price(
    path: &str,
    panora_url: &str,
    panora_api_key: &str,
    model_name: &str,
    token_address: &str,
    version: CoinVersion,
    total_tokens: u64,
    tool_usage: Vec<ToolUsage>,
    client: &Client,
) -> Result<(u64, String), ConsumerError> {
    println!("model_name: {}", model_name);
    println!("total_tokens: {}", total_tokens);
    println!("tool_usage: {:?}", tool_usage);

    let prices_file = fs::read_to_string(path)
        .map_err(|e| ConsumerError::InvalidMessage(format!("Failed to read prices file: {}", e)))?;

    let price: Price = from_str(&prices_file).map_err(|e| {
        ConsumerError::InvalidMessage(format!("Failed to parse prices file: {}", e))
    })?;

    let price_model = price
        .model
        .iter()
        .find(|model| model.name.to_string() == model_name)
        .ok_or_else(|| ConsumerError::InvalidMessage(format!("Model not found: {}", model_name)))?;

    let price_coins_response_query = client
        .get(format!("{}/prices", panora_url))
        .header("x-api-key", panora_api_key)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json");

    let price_coins_response = if version == CoinVersion::V1 {
        price_coins_response_query
            .query(&[("tokenAddress", &token_address)])
            .send()
            .await?
    } else {
        price_coins_response_query
            .query(&[("faAddress", &token_address)])
            .send()
            .await?
    };

    if !price_coins_response.status().is_success() {
        let error_text = price_coins_response.text().await?;
        return Err(ConsumerError::InvalidMessage(format!(
            "API error: {}",
            error_text
        )));
    }

    let price_coins: Vec<PriceCoin> = price_coins_response.json().await.map_err(|e| {
        ConsumerError::InvalidMessage(format!("Failed to parse price response: {}", e))
    })?;

    let coin = price_coins
        .iter()
        .find(|token| token.token_address.as_ref() == Some(&token_address.to_string()))
        .ok_or_else(|| {
            ConsumerError::InvalidMessage(format!("Token address not found: {}", token_address))
        })?;

    let price_coin_f64 = coin
        .usd_price
        .as_ref()
        .unwrap_or(&"0.0".to_string())
        .parse::<f64>()
        .unwrap_or(0.0);

    let price_tokens = ((price_model.price * total_tokens as f64) / 1000 as f64) / price_coin_f64;

    let price_tools: f64 = tool_usage
        .iter()
        .filter_map(|tool| {
            let tool_name = ai_tool_to_tool_name(&tool.tool);
            price
                .tool
                .iter()
                .find(|t| t.name.to_string() == tool_name.to_string())
                .map(|t| (t.price / price_coin_f64) * tool.calls as f64)
        })
        .sum();

    let total_price = price_tokens + price_tools;

    let total_price_blockchain =
        (total_price * 10_f64.powi(coin.decimals.unwrap_or(8) as i32)) as u64;

    Ok((total_price_blockchain, token_address.to_string()))
}
