use anyhow::Result;
use aptos_rust_sdk::client::{
    builder::AptosClientBuilder, config::AptosNetwork, rest_api::AptosFullnodeClient,
};
use aptos_rust_sdk_types::api_types::{chain_id::ChainId, view::ViewRequest};
use quark_core::helpers::dto::TokenAddress;

#[derive(Clone)]
pub struct Aptos {
    pub node: AptosFullnodeClient,
    pub contract_address: String,
}

impl Aptos {
    pub fn new(network: String, contract_address: String) -> Self {
        let (builder, _chain_id) = match network.as_str() {
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

        Self {
            node,
            contract_address,
        }
    }

    pub async fn get_token_address(&self) -> Result<String> {
        let coin_address_value = self
            .node
            .view_function(ViewRequest {
                function: format!("{}::user::get_token_address", self.contract_address),
                type_arguments: vec![],
                arguments: vec![],
            })
            .await?
            .into_inner();

        let coin_address = serde_json::from_value::<Vec<TokenAddress>>(coin_address_value)?;

        if coin_address[0].vec[0].clone() == "0x1" {
            return Ok(format!("0x1::aptos_coin::AptosCoin"));
        }

        Ok(format!(
            "{}::coin_factory::Emojicoin",
            coin_address[0].vec[0].clone()
        ))
    }

    pub async fn get_account_balance(&self, address: &str, token_address: &str) -> Result<i64> {
        let coin_address_formatted = if token_address == String::from("0x1") {
            format!("0x1::aptos_coin::AptosCoin")
        } else {
            token_address.to_string()
        };

        let balance = self
            .node
            .get_account_balance(address.to_string(), coin_address_formatted)
            .await?;

        let balance = balance.into_inner();

        let balance = balance.as_i64();

        if balance.is_none() {
            return Err(anyhow::anyhow!("Balance not found"));
        }

        Ok(balance.unwrap())
    }
}
