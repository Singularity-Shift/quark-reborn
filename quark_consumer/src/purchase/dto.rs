use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::{address::AccountAddress, chain_id::ChainId};

pub enum PurchaseType {
    User(String),
    Group(String),
}

pub struct Purchase {
    pub purchase_type: PurchaseType,
    pub contract_address: AccountAddress,
    pub token_address: String,
    pub amount: u64,
    pub node: AptosFullnodeClient,
    pub chain_id: ChainId,
}

impl
    From<(
        PurchaseType,
        AccountAddress,
        u64,
        String,
        AptosFullnodeClient,
        ChainId,
    )> for Purchase
{
    fn from(
        value: (
            PurchaseType,
            AccountAddress,
            u64,
            String,
            AptosFullnodeClient,
            ChainId,
        ),
    ) -> Self {
        let (purchase_type, contract_address, amount, token_address, node, chain_id) = value;

        Self {
            purchase_type,
            contract_address,
            token_address,
            amount,
            node,
            chain_id,
        }
    }
}
