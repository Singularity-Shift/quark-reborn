use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub enum QueryValue {
    Boolean(bool),
    String(String),
}

#[derive(Debug, Deserialize)]
pub struct PanoraResponse {
    pub data: Vec<Token>,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "panoraId")]
    pub panora_id: String,
    #[serde(rename = "tokenAddress")]
    pub token_address: Option<String>,
    #[serde(rename = "faAddress")]
    pub fa_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub bridge: Option<String>,
    #[serde(rename = "panoraSymbol")]
    pub panora_symbol: String,
    #[serde(rename = "usdPrice")]
    pub usd_price: Option<String>,
    #[serde(rename = "logoUrl")]
    pub logo_url: Option<String>,
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    #[serde(rename = "panoraUI")]
    pub panora_ui: bool,
    #[serde(rename = "panoraTags")]
    pub panora_tags: Vec<String>,
    #[serde(rename = "panoraIndex")]
    pub panora_index: u64,
    #[serde(rename = "coinGeckoId")]
    pub coin_gecko_id: Option<String>,
    #[serde(rename = "coinMarketCapId")]
    pub coin_market_cap_id: Option<u64>,
    #[serde(rename = "isInPanoraTokenList")]
    pub is_in_panora_token_list: bool,
    #[serde(rename = "isBanned")]
    pub is_banned: bool,
}
