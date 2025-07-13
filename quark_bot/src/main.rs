mod ai;
mod aptos;
mod assets;
mod bot;
mod callbacks;
mod credentials;
mod db;
mod middleware;
mod panora;
mod services;
mod user_conversation;
mod user_model_preferences;
mod utils;

use crate::{
    ai::{gcs::GcsImageUploader, handler::AI},
    aptos::handler::Aptos,
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
use tokio_cron_scheduler::{Job, JobScheduler};

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
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let gcs_creds = env::var("STORAGE_CREDENTIALS").expect("STORAGE_CREDENTIALS not set");
    let bucket_name = env::var("GCS_BUCKET_NAME").expect("GCS_BUCKET_NAME not set");
    let aptos_network = env::var("APTOS_NETWORK").expect("APTOS_NETWORK not set");
    let contract_address = env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS not set");

    let media_aggregator = Arc::new(media_aggregator::MediaGroupAggregator::new(
        bot.clone(),
        db.clone(),
    ));

    let google_cloud = GcsImageUploader::new(&gcs_creds, bucket_name)
        .await
        .expect("Failed to create GCS image uploader");

    let aptos = Aptos::new(aptos_network, contract_address);

    let panora = Panora::new(&db, aptos).expect("Failed to create Panora");

    // Create clone for dispatcher early to avoid move issues
    let panora_for_dispatcher = panora.clone();

    // Execute token list updates immediately on startup
    let panora_startup = panora.clone();
    log::info!("Executing initial token list update on startup...");
    match panora_startup.set_panora_token_list().await {
        Ok(_) => log::info!("Successfully updated Panora token list on startup"),
        Err(e) => log::error!("Failed to update Panora token list on startup: {}", e),
    }

    // Execute token AI fees update immediately on startup
    let panora_startup2 = panora.clone();
    let token_address = panora_startup2.aptos.get_token_address().await.unwrap();
    match panora_startup2.set_token_ai_fees(&token_address).await {
        Ok(_) => log::info!("Successfully updated Panora token AI fees on startup"),
        Err(e) => log::error!("Failed to update Panora token AI fees on startup: {}", e),
    }

    // Set up cron job for Panora token list updates (runs every hour)
    let panora_clone1 = panora.clone();
    let panora_clone2 = panora.clone();
    let scheduler = JobScheduler::new()
        .await
        .expect("Failed to create job scheduler");

    let job_token_list = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let panora_inner = panora_clone1.clone();
        Box::pin(async move {
            match panora_inner.set_panora_token_list().await {
                Ok(_) => log::info!("Successfully updated Panora token list"),
                Err(e) => log::error!("Failed to update Panora token list: {}", e),
            }
        })
    })
    .expect("Failed to create cron job");

    let job_token_ai_fees = Job::new_async("0 * * * * *", move |_uuid, _l| {
        let panora_inner = panora_clone2.clone();

        Box::pin(async move {
            let token_address = panora_inner.aptos.get_token_address().await.unwrap();
            match panora_inner.set_token_ai_fees(&token_address).await {
                Ok(_) => log::info!("Successfully updated Panora token AI fees"),
                Err(e) => log::error!("Failed to update Panora token AI fees: {}", e),
            }
        })
    })
    .expect("Failed to create cron job");

    scheduler
        .add(job_token_list)
        .await
        .expect("Failed to add job to scheduler");

    scheduler
        .add(job_token_ai_fees)
        .await
        .expect("Failed to add job to scheduler");

    scheduler.start().await.expect("Failed to start scheduler");

    log::info!("Panora token list cron job started (runs every hour)");

    let min_deposit = env::var("MIN_DEPOSIT")
        .expect("MIN_DEPOSIT not set")
        .parse::<f64>()
        .expect("MIN_DEPOSIT must be a number");

    let ai = AI::new(openai_api_key.clone(), google_cloud, panora, min_deposit);

    let user_convos = UserConversations::new(&db).unwrap();
    let user_model_prefs = UserModelPreferences::new(&db).unwrap();
    let service = Services::new();
    let cmd_collector = Arc::new(command_image_collector::CommandImageCollector::new(
        bot.clone(),
        db.clone(),
        service.clone(),
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
    ];

    bot.set_my_commands(commands).await.unwrap();

    Dispatcher::builder(bot.clone(), handler_tree())
        .dependencies(dptree::deps![
            InMemStorage::<QuarkState>::new(),
            auth_db,
            db,
            service,
            user_convos,
            user_model_prefs,
            ai,
            media_aggregator,
            cmd_collector,
            panora_for_dispatcher
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
