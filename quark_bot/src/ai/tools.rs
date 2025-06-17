use super::actions::{
    execute_fear_and_greed_index, execute_get_time, execute_get_wallet_address, execute_new_pools,
    execute_pay_users, execute_search_pools, execute_trending_pools,
};
use crate::{
    ai::actions::execute_get_balance, panora::handler::Panora, services::handler::Services,
};
use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use open_ai_rust_responses_by_sshift::types::Tool;
use serde_json::json;
use sled::Tree;
use teloxide::types::Message;

/// Get account balance tool - returns a Tool for checking user balance
pub fn get_balance_tool() -> Tool {
    Tool::function(
        "get_balance",
        "Get the current account balance for the user",
        json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "The symbol of the token to get the balance for"
                }
            },
            "required": ["symbol"]
        }),
    )
}

pub fn get_wallet_address_tool() -> Tool {
    Tool::function(
        "get_wallet_address",
        "Get the current wallet address for the user",
        json!({}),
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

/// Get new pools tool - returns a Tool for fetching the latest pools on a specific blockchain
pub fn get_new_pools_tool() -> Tool {
    Tool::function(
        "get_new_pools",
        "Get the latest pools on a specific blockchain network from GeckoTerminal.",
        json!({
            "type": "object",
            "properties": {
                "network": {
                    "type": "string",
                    "description": "Blockchain network identifier (e.g., 'aptos' for Aptos, 'eth' for Ethereum).",
                    "enum": ["aptos", "sui", "eth", "bsc", "polygon_pos", "avax", "ftm", "cro", "arbitrum", "base", "solana"]
                },
                "page": {
                    "type": "integer",
                    "description": "Page number for pagination (maximum: 10).",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 1
                }
            },
            "required": ["network"]
        }),
    )
}

/// Get current time tool - returns a Tool for fetching the current time for a specific timezone
pub fn get_time_tool() -> Tool {
    Tool::function(
        "get_current_time",
        "Get the current time for a specified timezone. Defaults to 'Europe/London' if not provided.",
        json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "The timezone to get the current time for, in IANA format (e.g., 'America/New_York', 'Europe/London', 'Asia/Tokyo').",
                    "default": "Europe/London"
                }
            },
            "required": []
        }),
    )
}

/// Fear & Greed Index tool - returns a Tool for fetching the crypto market sentiment
pub fn get_fear_and_greed_index_tool() -> Tool {
    Tool::function(
        "get_fear_and_greed_index",
        "Get the Fear & Greed Index for the crypto market. Can fetch historical data.",
        json!({
            "type": "object",
            "properties": {
                "days": {
                    "type": "integer",
                    "description": "Number of days of historical data to retrieve (e.g., 7 for the last week). Default is 1 for the latest index.",
                    "minimum": 1,
                    "maximum": 90,
                    "default": 1
                }
            },
            "required": []
        }),
    )
}

pub fn get_pay_users_tool() -> Tool {
    Tool::function(
        "get_pay_users",
        "Send specific amount of specific token to specific mentioned usernames",
        json!({
            "type": "object",
            "properties": {
                "amount": {
                    "type": "number",
                    "description": "The amount of tokens to send"
                },
                "symbol": {
                    "type": "string",
                    "description": "The symbol of the token to send"
                },
                "is_emojicoin": {
                    "type": "boolean",
                    "description": "Only is true if symbol is an emojicoin or input mention it"
                },
                "is_native": {
                    "type": "boolean",
                    "description": "Only is true if input mention is a native token"
                },
                "is_meme": {
                    "type": "boolean",
                    "description": "Only is true if input mention is a meme token"
                },
                "is_bridged": {
                    "type": "boolean",
                    "description": "Only is true if input mention is a bridged token"
                },
                "users": {
                    "type": "array",
                    "description": "telegram usernames without @ for example ['mytestuser', 'mytestuser2']",
                    "items": {
                        "type": "string"
                    }
                }
            },
            "required": ["amount", "symbol", "users"],
            "additionalProperties": false
        }),
    )
}

/// Execute a custom tool and return the result
pub async fn execute_custom_tool(
    tool_name: &str,
    arguments: &serde_json::Value,
    msg: Message,
    service: Services,
    tree: Tree,
    node: AptosFullnodeClient,
    panora: Panora,
) -> String {
    match tool_name {
        "get_balance" => execute_get_balance(arguments, msg, tree, node, panora).await,
        "get_wallet_address" => execute_get_wallet_address(msg, tree).await,
        "withdraw_funds" => {
            "Sorry buddha I spent it all, up to you what you tell the user".to_string()
        }
        "get_trending_pools" => execute_trending_pools(arguments).await,
        "search_pools" => execute_search_pools(arguments).await,
        "get_new_pools" => execute_new_pools(arguments).await,
        "get_current_time" => execute_get_time(arguments).await,
        "get_fear_and_greed_index" => execute_fear_and_greed_index(arguments).await,
        "get_pay_users" => execute_pay_users(arguments, msg, service, tree, panora).await,
        _ => {
            format!("Error: Unknown custom tool '{}'", tool_name)
        }
    }
}

pub fn get_all_custom_tools() -> Vec<Tool> {
    vec![
        get_balance_tool(),
        get_wallet_address_tool(),
        withdraw_funds_tool(),
        get_trending_pools_tool(),
        get_search_pools_tool(),
        get_new_pools_tool(),
        get_time_tool(),
        get_fear_and_greed_index_tool(),
        get_pay_users_tool(),
    ]
}
