use std::{env, str::FromStr, sync::Arc};

use aptos_rust_sdk::client::{builder::AptosClientBuilder, config::AptosNetwork};
use aptos_rust_sdk_types::api_types::{address::AccountAddress, chain_id::ChainId};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};

use crate::{
    docs::{dto::ApiDoc, handler::api_docs},
    info::handler::info,
    middlewares::handler::auth,
    pay_users::handler::pay_users,
    purchase::handler::purchase,
    state::ServerState,
};

pub async fn router() -> Router {
    let network = env::var("APTOS_NETWORK").expect("APTOS_NETWORK environment variable not set");
    let contract_address =
        env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS environment variable not set");

    let token_payment_address = env::var("TOKEN_PAYMENT_ADDRESS")
        .expect("TOKEN_PAYMENT_ADDRESS environment variable not set");

    let (builder, chain_id) = match network.as_str() {
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

    let contract_address = AccountAddress::from_str(&contract_address)
        .expect("CONTRACT_ADDRESS is not a valid account address");

    let state = Arc::new(ServerState::from((
        node,
        chain_id,
        contract_address,
        token_payment_address,
    )));

    let doc = ApiDoc::openapi();

    let auth_router = Router::new()
        .route("/pay-users", post(pay_users))
        .route("/purchase", post(purchase))
        .route_layer(middleware::from_fn(auth));

    Router::new()
        .merge(Redoc::with_url("/redoc", doc))
        .merge(auth_router)
        .route("/", get(info))
        .route("/docs", get(api_docs))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
