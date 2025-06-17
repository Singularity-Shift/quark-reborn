use anyhow::Result;
use reqwest::Client;
use std::env;

use crate::panora::dto::{PanoraResponse, QueryValue, Token};

#[derive(Clone)]
pub struct Panora {
    client: Client,
}

impl Panora {
    pub fn new() -> Self {
        let client = Client::new();

        Self { client }
    }

    pub async fn get_panora_token_list(
        &self,
        is_emojicoin: bool,
        is_native: bool,
        is_meme: bool,
        is_bridged: bool,
    ) -> Result<Vec<Token>> {
        let panora_url = env::var("PANORA_URL").expect("PANORA_URL must be set");
        let panora_api_key = env::var("PANORA_API_KEY").expect("PANORA_API_KEY must be set");

        let mut tags = vec![];

        if is_emojicoin {
            tags.push("Emojicoin");
        }

        if is_native {
            tags.push("Native");
        }

        if is_meme {
            tags.push("Meme");
        }

        if is_bridged {
            tags.push("Bridged");
        }

        let response = self
            .client
            .get(format!("{}/tokenlist", panora_url))
            .query(&[
                ("panoraUI", QueryValue::Boolean(false)),
                ("tags", QueryValue::String(tags.join(","))),
            ])
            .header("x-api-key", panora_api_key)
            .send()
            .await?;

        let body = response.json::<PanoraResponse>().await?;

        Ok(body.data)
    }
}
