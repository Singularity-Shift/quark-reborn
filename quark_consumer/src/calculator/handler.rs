use super::dto::{Price, ToolName};
use crate::error::ConsumerError;
use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::{address::AccountAddress, view::ViewRequest};
use quark_core::helpers::dto::{AITool, PriceCoin, TokenAddress, ToolUsage};
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
    total_tokens: u64,
    tool_usage: Vec<ToolUsage>,
    client: &Client,
    contract_address: &AccountAddress,
    node: &AptosFullnodeClient,
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

    let view_request = ViewRequest {
        function: format!("{contract_address}::user::get_token_address"),
        type_arguments: vec![],
        arguments: vec![],
    };

    let token_address_response = node
        .view_function(view_request)
        .await
        .map_err(|e| ConsumerError::InvalidMessage(format!("Failed to view function: {}", e)))?;

    let token_addresses: Vec<TokenAddress> =
        serde_json::from_value(token_address_response.into_inner()).map_err(|e| {
            ConsumerError::InvalidMessage(format!("Failed to parse token address: {}", e))
        })?;

    let token_address = if token_addresses[0].vec[0] == "0x1" {
        "0x1::aptos_coin::AptosCoin".to_string()
    } else {
        token_addresses[0].vec[0].clone()
    };

    let price_coins_response = client
        .get(format!("{}/prices", panora_url))
        .header("x-api-key", panora_api_key)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .query(&[("tokenAddress", &token_address)])
        .send()
        .await?;

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
        .find(|token| token.token_address.as_ref() == Some(&token_address))
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

    Ok((total_price_blockchain, token_address))
}
