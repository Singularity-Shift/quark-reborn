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
}

impl Panora {
    pub fn new(db: &Db, aptos: Aptos) -> sled::Result<Self> {
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
        })
    }

    pub async fn set_panora_token_list(&self) -> Result<()> {
        let response = self
            .client
            .get(format!("{}/tokenlist", self.panora_url))
            .query(&[("panoraUI", false)])
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

        Ok(())
    }

    pub async fn set_token_ai_fees(&self, token_address: &str) -> Result<()> {
        let token_address_param = if token_address == "0x1" {
            "0x1::aptos_coin::AptosCoin"
        } else {
            token_address
        };

        let price_coins_response = self
            .client
            .get(format!("{}/prices", self.panora_url))
            .header("x-api-key", self.panora_api_key.clone())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .query(&[("tokenAddress", &token_address_param)])
            .send()
            .await?;

        let price_coins = price_coins_response.json::<Vec<PriceCoin>>().await?;

        let price_coin = price_coins
            .iter()
            .find(|pc| pc.token_address == Some(token_address_param.to_string()));

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

    pub async fn get_token_by_symbol(&self, symbol: &str) -> Result<Token> {
        let list = self.get_panora_token_list().await?;

        let token = list
            .iter()
            .find(|t| t.symbol.to_lowercase() == symbol.to_lowercase() && !t.is_banned);

        if token.is_none() {
            return Err(anyhow::anyhow!("Token not found"));
        }

        let token = token.unwrap();

        Ok(token.clone())
    }

    pub async fn get_token_v1(&self, address: &str) -> Result<Token> {
        let response = self.get_panora_token_list().await?;

        let token = response
            .iter()
            .find(|t| t.token_address.as_ref() == Some(&address.to_string()));

        if token.is_none() {
            return Err(anyhow::anyhow!("Token not found"));
        }

        let token = token.unwrap();

        Ok(token.clone())
    }

    pub async fn get_token_ai_fees(&self) -> Result<PriceCoin> {
        let price_coin = self.tree.get(b"token_ai_fees")?;

        if price_coin.is_none() {
            return Err(anyhow::anyhow!("Token AI fees not found"));
        }

        let price_coin = price_coin.unwrap();

        let price_coin = serde_json::from_slice::<PriceCoin>(&price_coin)?;

        Ok(price_coin)
    }
}
