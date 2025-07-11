mod admin;
mod calculator;
mod error;
mod purchase;

use aptos_rust_sdk::client::builder::AptosClientBuilder;
use aptos_rust_sdk::client::config::AptosNetwork;
use aptos_rust_sdk_types::api_types::address::AccountAddress;
use aptos_rust_sdk_types::api_types::chain_id::ChainId;
use error::{ConsumerError, ConsumerResult};
use quark_core::helpers::dto::PurchaseMessage;
use redis::{AsyncCommands, Client};
use reqwest::Client as ReqClient;
use serde_json;
use std::env;
use std::str::FromStr;
use std::time::Duration;

use crate::calculator::handler::get_price;
use crate::purchase::dto::{Purchase, PurchaseType};
use crate::purchase::handler::purchase_ai;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> ConsumerResult<()> {
    let consumer_id = env::var("CONSUMER_ID").unwrap_or_else(|_| "consumer".to_string());
    let network = env::var("APTOS_NETWORK").expect("APTOS_NETWORK environment variable not set");
    let contract_address =
        env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS environment variable not set");
    let redis_url = env::var("REDIS_URL").map_err(|_| {
        ConsumerError::ConnectionFailed("REDIS_URL environment variable not set".to_string())
    })?;
    let path = "assets/prices.ron".to_string();
    let panora_url =
        env::var("PANORA_URL").unwrap_or_else(|_| "https://api.panora.exchange".to_string());
    let panora_api_key = env::var("PANORA_API_KEY").unwrap_or_else(|_| "".to_string());

    let (builder, chain_id) = match network.as_str() {
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

    let contract_address = AccountAddress::from_str(&contract_address)
        .expect("CONTRACT_ADDRESS is not a valid account address");

    println!("[{}] Starting Quark Consumer...", consumer_id);
    println!("[{}] Connecting to Redis", consumer_id);

    let redis_client = Client::open(redis_url).map_err(|e| {
        ConsumerError::ConnectionFailed(format!("Failed to create Redis client: {}", e))
    })?;

    let mut redis_connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| {
            ConsumerError::ConnectionFailed(format!("Failed to get Redis connection: {}", e))
        })?;

    println!("[{}] Connected to Redis successfully", consumer_id);
    println!("[{}] Starting consumer loop...", consumer_id);

    loop {
        match redis_connection
            .rpop::<_, Option<String>>("purchase", None)
            .await
        {
            Ok(outcome) => match outcome {
                Some(message) => {
                    let purchase: PurchaseMessage =
                        serde_json::from_str(&message).map_err(|e| {
                            ConsumerError::InvalidMessage(format!("Failed to parse message: {}", e))
                        })?;

                    let model_name = purchase.model.to_string();
                    let total_tokens = purchase.tokens_used;
                    let tool_usage = purchase.tools_used;
                    let client = ReqClient::builder()
                        .user_agent("quark-consumer/1.0")
                        .build()
                        .map_err(|e| {
                            ConsumerError::InvalidMessage(format!(
                                "Failed to create HTTP client: {}",
                                e
                            ))
                        })?;

                    let node_price = node.clone();

                    let price = get_price(
                        &path,
                        &panora_url,
                        &panora_api_key,
                        &model_name,
                        total_tokens as u64,
                        tool_usage,
                        &client,
                        &contract_address,
                        &node_price,
                    )
                    .await;

                    if price.is_err() {
                        eprintln!("[{}] Error getting price: {:?}", consumer_id, price.err());

                        let _: () =
                            redis_connection
                                .lpush("purchase", message)
                                .await
                                .map_err(|e| {
                                    ConsumerError::InvalidMessage(format!(
                                        "Failed to push message to Redis: {}",
                                        e
                                    ))
                                })?;

                        return Err(ConsumerError::InvalidMessage(
                            "Failed to get price".to_string(),
                        ));
                    }

                    let purchase_type = if purchase.group_id.is_some() {
                        PurchaseType::Group(purchase.group_id.unwrap())
                    } else {
                        PurchaseType::User(purchase.account_address)
                    };

                    let (amount, token_address) = price.unwrap();

                    let purchase_query = Purchase::from((
                        purchase_type,
                        contract_address,
                        amount,
                        token_address,
                        node.clone(),
                        chain_id,
                    ));

                    let transaction_response = purchase_ai(purchase_query).await;

                    if transaction_response.is_err() {
                        eprintln!(
                            "[{}] Error purchasing: {:?}",
                            consumer_id,
                            transaction_response.err()
                        );

                        let _: () =
                            redis_connection
                                .lpush("purchase", message)
                                .await
                                .map_err(|e| {
                                    ConsumerError::InvalidMessage(format!(
                                        "Failed to push message to Redis: {}",
                                        e
                                    ))
                                })?;

                        return Err(ConsumerError::InvalidMessage(
                            "Failed to purchase".to_string(),
                        ));
                    }

                    let transaction_response = transaction_response.unwrap();

                    println!(
                        "[{}] Purchased successfully: {:?}",
                        consumer_id, transaction_response
                    );
                }
                None => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            },
            Err(e) => {
                eprintln!(
                    "[{}] Redis error: {}. Retrying in 5 seconds...",
                    consumer_id, e
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
