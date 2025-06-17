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
    pub chainId: u64,
    pub panoraId: String,
    pub tokenAddress: Option<String>,
    pub faAddress: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub bridge: Option<String>,
    pub panoraSymbol: String,
    pub usdPrice: String,
    pub logoUrl: Option<String>,
    pub websiteUrl: Option<String>,
    pub panoraUI: bool,
    pub panoraTags: Vec<String>,
    pub panoraIndex: u64,
    pub coinGeckoId: Option<String>,
    pub coinMarketCapId: Option<String>,
    pub isInPanoraTokenList: bool,
    pub isBanned: bool,
}
