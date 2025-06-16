use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::{address::AccountAddress, chain_id::ChainId};

#[derive(Clone)]
pub struct ServerState {
    node: AptosFullnodeClient,
    chain_id: ChainId,
    contract_address: AccountAddress,
    token_payment_address: String,
}

impl From<(AptosFullnodeClient, ChainId, AccountAddress, String)> for ServerState {
    fn from(states: (AptosFullnodeClient, ChainId, AccountAddress, String)) -> Self {
        let (node, chain_id, contract_address, token_payment_address) = states;
        Self {
            node,
            chain_id,
            contract_address,
            token_payment_address,
        }
    }
}

impl ServerState {
    pub fn node(&self) -> &AptosFullnodeClient {
        &self.node
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn contract_address(&self) -> AccountAddress {
        self.contract_address
    }

    pub fn token_payment_address(&self) -> String {
        self.token_payment_address.clone()
    }
}
