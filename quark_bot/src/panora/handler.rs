use anyhow::Result;
use quark_core::helpers::dto::PriceCoin;
use reqwest::Client;
use sled::{Db, Tree};
use std::env;

use crate::{
    aptos::handler::Aptos,
    panora::dto::{PanoraResponse, Token},
};

#[derive(Clone)]
pub struct Panora {
    client: Client,
    tree: Tree,
    panora_url: String,
    panora_api_key: String,
    pub aptos: Aptos,
    pub min_deposit: f64,
}

impl Panora {
    pub fn new(db: &Db, aptos: Aptos, min_deposit: f64) -> sled::Result<Self> {
        let client = Client::new();
        let tree = db.open_tree("panora")?;

        let panora_url = env::var("PANORA_URL").expect("PANORA_URL must be set");
        let panora_api_key = env::var("PANORA_API_KEY").expect("PANORA_API_KEY must be set");

        Ok(Self {
            client,
            tree,
            panora_url,
            panora_api_key,
            aptos,
            min_deposit,
        })
    }

    pub async fn set_panora_token_list(&self) -> Result<()> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 2000; // 2 seconds base delay

        for attempt in 1..=MAX_RETRIES {
            match self.set_panora_token_list_internal().await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("429") && attempt < MAX_RETRIES {
                        // Exponential backoff: 2s, 4s, 8s
                        let delay_ms = BASE_DELAY_MS * (2_u64.pow(attempt - 1));
                        log::warn!(
                            "Rate limited when updating token list (attempt {}/{}), waiting {}ms before retry",
                            attempt,
                            MAX_RETRIES,
                            delay_ms
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to update Panora token list after {} retries",
            MAX_RETRIES
        ))
    }

    async fn set_panora_token_list_internal(&self) -> Result<()> {
        let response = self
            .client
            .get(format!("{}/tokenlist", self.panora_url))
            .header("x-api-key", self.panora_api_key.clone())
            .send()
            .await?;

        if response.status() != 200 {
            let status = response.status();
            let error_text = response.text().await?;
            log::error!(
                "❌ Error getting tokens: status: {}, text: {}",
                status,
                error_text
            );
            return Err(anyhow::anyhow!(
                "❌ Error getting tokens: status: {}, text: {}",
                status,
                error_text,
            ));
        }

        let body = response.json::<PanoraResponse>().await?;

        let serialized_data = serde_json::to_vec(&body.data)?;
        self.tree.insert(b"panora_token_list", serialized_data)?;

        let response_non_bonding_tokens = self
            .client
            .get(format!("{}/tokenlist", self.panora_url))
            .header("x-api-key", self.panora_api_key.clone())
            .query(&[("panoraUI", false)])
            .send()
            .await?;

        if response_non_bonding_tokens.status() != 200 {
            let status = response_non_bonding_tokens.status();
            let error_text = response_non_bonding_tokens.text().await?;
            log::error!(
                "❌ Error getting non-bonding tokens: status: {}, text: {}",
                status,
                error_text
            );
            return Err(anyhow::anyhow!(
                "❌ Error getting non-bonding tokens: status: {}, text: {}",
                status,
                error_text,
            ));
        }

        let body_non_bonding_tokens = response_non_bonding_tokens.json::<PanoraResponse>().await?;

        let serialized_data_non_bonding_tokens = serde_json::to_vec(&body_non_bonding_tokens.data)?;
        self.tree.insert(
            b"panora_token_list_non_bonding",
            serialized_data_non_bonding_tokens,
        )?;

        Ok(())
    }

    pub async fn set_token_ai_fees(&self, token_address: &str) -> Result<()> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 2000; // 2 seconds base delay

        for attempt in 1..=MAX_RETRIES {
            match self.set_token_ai_fees_internal(token_address).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("429") && attempt < MAX_RETRIES {
                        // Exponential backoff: 2s, 4s, 8s
                        let delay_ms = BASE_DELAY_MS * (2_u64.pow(attempt - 1));
                        log::warn!(
                            "Rate limited when updating token AI fees (attempt {}/{}), waiting {}ms before retry",
                            attempt,
                            MAX_RETRIES,
                            delay_ms
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to update token AI fees after {} retries",
            MAX_RETRIES
        ))
    }

    async fn set_token_ai_fees_internal(&self, token_address: &str) -> Result<()> {
        let price_coins_response = self
            .client
            .get(format!("{}/prices", self.panora_url))
            .header("x-api-key", self.panora_api_key.clone())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .query(&[("tokenAddress", &token_address)])
            .send()
            .await?;

        let price_coins = price_coins_response.json::<Vec<PriceCoin>>().await?;

        let price_coin = price_coins
            .iter()
            .find(|pc| pc.token_address == Some(token_address.to_string()));

        if price_coin.is_none() {
            return Err(anyhow::anyhow!("Price coin not found"));
        }

        let price_coin = price_coin.unwrap();

        let serialized_data = serde_json::to_vec(&price_coin)?;
        self.tree.insert(b"token_ai_fees", serialized_data)?;

        Ok(())
    }

    pub async fn get_panora_token_list(&self) -> Result<Vec<Token>> {
        let list = self.tree.get(b"panora_token_list")?;

        if list.is_none() {
            return Err(anyhow::anyhow!("Panora token list not found"));
        }

        let list = list.unwrap();

        let list = serde_json::from_slice::<Vec<Token>>(&list);

        if list.is_err() {
            return Err(anyhow::anyhow!("Error parsing panora token list"));
        }

        Ok(list.unwrap())
    }

    pub async fn get_panora_token_list_non_bonding(&self) -> Result<Vec<Token>> {
        let list = self.tree.get(b"panora_token_list_non_bonding")?;

        if list.is_none() {
            return Err(anyhow::anyhow!("Panora token list not found"));
        }

        let list = list.unwrap();

        let list = serde_json::from_slice::<Vec<Token>>(&list);

        if list.is_err() {
            return Err(anyhow::anyhow!("Error parsing panora token list"));
        }

        Ok(list.unwrap())
    }

    pub async fn get_token_by_symbol(&self, symbol: &str) -> Result<Token> {
        let list = self.get_panora_token_list().await?;

        let clean_symbol = symbol.replace('\u{fe0f}', "");

        let mut token = list
            .iter()
            .find(|t| {
                t.panora_symbol.replace('\u{fe0f}', "").to_lowercase()
                    == clean_symbol.to_lowercase()
                    && !t.is_banned
            })
            .cloned();

        if token.is_none() {
            let list_non_bonding = self.get_panora_token_list_non_bonding().await?;

            token = list_non_bonding
                .iter()
                .find(|t| {
                    t.panora_symbol.replace('\u{fe0f}', "").to_lowercase()
                        == clean_symbol.to_lowercase()
                        && !t.is_banned
                })
                .cloned();

            if token.is_none() {
                return Err(anyhow::anyhow!("Token not found"));
            }
        }

        let token = token.unwrap();

        Ok(token.clone())
    }
}
