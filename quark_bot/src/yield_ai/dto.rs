use serde::{Deserialize, Serialize};

/// A snapshot of an address portfolio at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortfolioSnapshot {
    /// Hex-encoded address string
    pub address: String,
    /// ISO-8601 timestamp string (e.g. "2025-08-09T04:28:10.597Z")
    pub timestamp: String,
    /// List of token holdings for the address
    pub tokens: Vec<TokenHolding>,
}

/// Token balance and valuation details for a single asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenHolding {
    /// Fully-qualified token address or type string
    #[serde(rename = "tokenAddress")]
    pub token_address: String,
    /// Optional ticker symbol (may be null)
    pub symbol: Option<String>,
    /// Optional human-readable token name (may be null)
    pub name: Option<String>,
    /// Optional number of decimals (may be null)
    pub decimals: Option<u8>,
    /// Raw token amount as a string to preserve precision
    pub amount: String,
    /// Optional token price in USD
    #[serde(rename = "priceUSD")]
    pub price_usd: Option<f64>,
    /// Optional position value in USD
    #[serde(rename = "valueUSD")]
    pub value_usd: Option<f64>,
}
