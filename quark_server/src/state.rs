use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::api_types::{address::AccountAddress, chain_id::ChainId};
use redis::aio::MultiplexedConnection;

#[derive(Clone)]
pub struct ServerState {
    node: AptosFullnodeClient,
    chain_id: ChainId,
    contract_address: AccountAddress,
    redis_client: MultiplexedConnection,
}

impl
    From<(
        AptosFullnodeClient,
        ChainId,
        AccountAddress,
        MultiplexedConnection,
    )> for ServerState
{
    fn from(
        states: (
            AptosFullnodeClient,
            ChainId,
            AccountAddress,
            MultiplexedConnection,
        ),
    ) -> Self {
        let (node, chain_id, contract_address, redis_client) = states;
        Self {
            node,
            chain_id,
            contract_address,
            redis_client,
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

    pub fn redis_client(&self) -> &MultiplexedConnection {
        &self.redis_client
    }
}
