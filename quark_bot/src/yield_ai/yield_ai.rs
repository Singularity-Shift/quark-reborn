use std::env;

use crate::yield_ai::dto::PortfolioSnapshot;
use anyhow::{Result, anyhow};

#[derive(Clone)]
pub struct YieldAI {
    base_url: String,
    api_key: String,
}

impl YieldAI {
    pub fn new() -> Self {
        let base_url = env::var("YIELD_AI_URL").expect("YIELD_AI_URL must be set");
        let api_key = env::var("YIELD_AI_API_KEY").expect("YIELD_AI_API_KEY must be set");

        Self { base_url, api_key }
    }

    pub async fn get_portfolio_snapshot(&self, address: String) -> Result<PortfolioSnapshot> {
        let url = format!("{}/wallet/{}/balance", self.base_url, address);
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Content-Type", "application/json")
            .query(&[("api_key", &self.api_key)])
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get portfolio snapshot: {}", e))?;

        log::info!("Yield AI response: {:?}", response);

        let body = response
            .json::<PortfolioSnapshot>()
            .await
            .map_err(|e| anyhow!("Failed to parse portfolio snapshot: {}", e))?;
        Ok(body)
    }
}
