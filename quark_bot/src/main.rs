mod ai;
mod aptos;
mod assets;
mod bot;
mod callbacks;
mod credentials;
mod dao;
mod db;
mod group;
mod job;
mod message_history;
mod panora;
mod services;
mod user_conversation;
mod user_model_preferences;
mod utils;

mod dependencies;

use crate::{
    ai::{gcs::GcsImageUploader, handler::AI},
    aptos::handler::Aptos,
    credentials::handler::Auth,
    dao::dao::Dao,
    dependencies::BotDependencies,
    group::handler::Group,
    job::job_scheduler::schedule_jobs,
    message_history::handler::MessageHistory,
    panora::handler::Panora,
    services::handler::Services,
    user_conversation::handler::UserConversations,
    user_model_preferences::handler::UserModelPreferences,
};
use quark_core::helpers::bot_commands::QuarkState;
use std::env;
use std::sync::Arc;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use teloxide::types::BotCommand;

use crate::assets::command_image_collector;
use crate::assets::media_aggregator;
use crate::bot::handler_tree::handler_tree;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    log::info!("Starting quark_bot...");

    let bot = Bot::from_env();
    let db = db::init_tree();
    let auth_db = db.open_tree("auth").expect("Failed to open auth tree");
    let group_db = db.open_tree("group").expect("Failed to open group tree");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let gcs_creds = env::var("STORAGE_CREDENTIALS").expect("STORAGE_CREDENTIALS not set");
    let bucket_name = env::var("GCS_BUCKET_NAME").expect("GCS_BUCKET_NAME not set");
    let aptos_network = env::var("APTOS_NETWORK").expect("APTOS_NETWORK not set");
    let contract_address = env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS not set");
    let aptos_api_key = env::var("APTOS_API_KEY").unwrap_or_default();

    let google_cloud = GcsImageUploader::new(&gcs_creds, bucket_name)
        .await
        .expect("Failed to create GCS image uploader");

    let aptos = Aptos::new(aptos_network, contract_address, aptos_api_key);

    let min_deposit = env::var("MIN_DEPOSIT")
        .expect("MIN_DEPOSIT not set")
        .parse::<f64>()
        .expect("MIN_DEPOSIT must be a number");

    let panora = Panora::new(&db, aptos, min_deposit).expect("Failed to create Panora");

    // Create clone for dispatcher early to avoid move issues
    let panora_for_dispatcher = panora.clone();

    let auth = Auth::new(auth_db);
    let group = Group::new(group_db);

    // Execute token list updates immediately on startup
    let panora_startup = panora.clone();
    log::info!("Executing initial token list update on startup...");
    match panora_startup.set_panora_token_list().await {
        Ok(_) => log::info!("Successfully updated Panora token list on startup"),
        Err(e) => log::error!("Failed to update Panora token list on startup: {}", e),
    }

    // Execute token AI fees update immediately on startup
    let panora_startup2 = panora.clone();
    let token_address = panora_startup2.aptos.get_token_address().await;
    match token_address {
        Ok(token_address) => match panora_startup2.set_token_ai_fees(&token_address).await {
            Ok(_) => log::info!("Successfully updated Panora token AI fees on startup"),
            Err(e) => log::error!("Failed to update Panora token AI fees on startup: {}", e),
        },
        Err(e) => log::error!("Failed to get token address: {}", e),
    }

    let dao_db = db.open_tree("dao").expect("Failed to open dao tree");
    let dao = Dao::new(dao_db);

    schedule_jobs(panora.clone(), bot.clone(), dao.clone())
        .await
        .expect("Failed to schedule jobs");

    let ai = AI::new(openai_api_key.clone(), google_cloud);

    let user_convos = UserConversations::new(&db).unwrap();
    let user_model_prefs = UserModelPreferences::new(&db).unwrap();
    let service = Services::new();

    let cmd_collector = Arc::new(command_image_collector::CommandImageCollector::new(
        bot.clone(),
    ));

    let media_aggregator = Arc::new(media_aggregator::MediaGroupAggregator::new(
        bot.clone(),
        ai.clone(),
        auth.clone(),
        user_model_prefs.clone(),
    ));

    let commands = vec![
        BotCommand::new("aptosconnect", "Open the Aptos Connect app."),
        BotCommand::new("help", "Display this text."),
        BotCommand::new("loginuser", "Log in as a user (DM only)."),
        BotCommand::new("logingroup", "Group login (under development)."),
        BotCommand::new("addfiles", "Upload files to your vector store (DM only)."),
        BotCommand::new("listfiles", "List files in your vector store (DM only)."),
        BotCommand::new("newchat", "Start a new conversation thread."),
        BotCommand::new("c", "prompt to chat AI with the bot."),
        BotCommand::new("r", "prompt to chat AI with the bot with reasoning."),
        BotCommand::new(
            "g",
            "prompt to chat AI with the bot in a group. (only admins can use this command)",
        ),
        BotCommand::new("walletaddress", "Get your wallet address."),
        BotCommand::new(
            "selectreasoningmodel",
            "Select reasoning model (O-series) and effort level.",
        ),
        BotCommand::new(
            "selectmodel",
            "Select chat model (4-series) and temperature.",
        ),
        BotCommand::new(
            "mysettings",
            "View your current model preferences (DM only).",
        ),
        BotCommand::new("sentinel", "Monitor system status (on/off)."),
        BotCommand::new("mod", "Moderate content (reply to message)."),
        BotCommand::new(
            "moderationrules",
            "Display the moderation rules to avoid getting muted.",
        ),
        BotCommand::new("balance", "Get your balance of a token."),
        BotCommand::new("groupwalletaddress", "Get the group's wallet address."),
        BotCommand::new("groupbalance", "Get the group's balance of a token."),
        BotCommand::new("proposalpreferences", "Set proposal preferences."),
        BotCommand::new("prices", "Display model pricing information."),
    ];

    let history_storage = InMemStorage::<MessageHistory>::new();

    bot.set_my_commands(commands).await.unwrap();

    let bot_deps = BotDependencies {
        db: db.clone(),
        auth: auth.clone(),
        service: service.clone(),
        user_convos: user_convos.clone(),
        user_model_prefs: user_model_prefs.clone(),
        ai: ai.clone(),
        cmd_collector: cmd_collector.clone(),
        panora: panora_for_dispatcher.clone(),
        group: group.clone(),
        dao: dao.clone(),
        media_aggregator: media_aggregator.clone(),
        history_storage: history_storage.clone(),
    };

    Dispatcher::builder(bot.clone(), handler_tree())
        .dependencies(dptree::deps![InMemStorage::<QuarkState>::new(), bot_deps])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
