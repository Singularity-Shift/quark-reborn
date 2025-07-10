mod ai;
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
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let gcs_creds = env::var("STORAGE_CREDENTIALS").expect("STORAGE_CREDENTIALS not set");
    let bucket_name = env::var("GCS_BUCKET_NAME").expect("GCS_BUCKET_NAME not set");
    let aptos_network = env::var("APTOS_NETWORK").expect("APTOS_NETWORK not set");

    let media_aggregator = Arc::new(media_aggregator::MediaGroupAggregator::new(
        bot.clone(),
        db.clone(),
    ));

    let google_cloud = GcsImageUploader::new(&gcs_creds, bucket_name)
        .await
        .expect("Failed to create GCS image uploader");

    let ai = AI::new(openai_api_key.clone(), google_cloud, aptos_network);

    let user_convos = UserConversations::new(&db).unwrap();
    let user_model_prefs = UserModelPreferences::new(&db).unwrap();
    
    let cmd_collector = Arc::new(command_image_collector::CommandImageCollector::new(
        bot.clone(),
        db.clone(),
        user_model_prefs.clone(),
    ));

    let auth_db = db.open_tree("auth").unwrap();

    let commands = vec![
        BotCommand::new("aptosconnect", "Open the Aptos Connect app."),
        BotCommand::new("help", "Display this text."),
        BotCommand::new("loginuser", "Log in as a user (DM only)."),
        BotCommand::new("logingroup", "Group login (under development)."),
        BotCommand::new("xlogin", "Login with X (Twitter) account (DM only)."),
        BotCommand::new("addfiles", "Upload files to your vector store (DM only)."),
        BotCommand::new("listfiles", "List files in your vector store (DM only)."),
        BotCommand::new("newchat", "Start a new conversation thread."),
        BotCommand::new("c", "prompt to chat AI with the bot."),
        BotCommand::new("r", "prompt to chat AI with the bot with reasoning."),
        BotCommand::new("selectreasoningmodel", "Select reasoning model (O-series) and effort level."),
        BotCommand::new("selectmodel", "Select chat model (4-series) and temperature."),
        BotCommand::new("mysettings", "View your current model preferences (DM only)."),
        BotCommand::new("sentinal", "Monitor system status (on/off)."),
        BotCommand::new("mod", "Moderate content (reply to message)."),
        BotCommand::new("moderationrules", "Display the moderation rules to avoid getting muted."),
    ];

    bot.set_my_commands(commands).await.unwrap();

    Dispatcher::builder(bot.clone(), handler_tree())
        .dependencies(dptree::deps![
            InMemStorage::<QuarkState>::new(),
            auth_db,
            db,
            user_convos,
            user_model_prefs,
            ai,
            media_aggregator,
            cmd_collector
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
