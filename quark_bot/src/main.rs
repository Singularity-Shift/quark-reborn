use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use std::env;
mod commands;
mod file_handling;
mod callbacks;
mod utils;
mod media_aggregator;
mod command_image_collector;

#[derive(BotCommands, Clone)]
#[command(description = "These commands are supported:")]
enum Command {
    #[command(description = "Log in as a user (DM only).", parse_with = "split")]
    LoginUser,
    #[command(description = "Group login (under development).", parse_with = "split")]
    LoginGroup,
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Upload files to your vector store (DM only).")]
    AddFiles,
    #[command(description = "List files in your vector store (DM only).")]
    ListFiles,
    #[command(description = "Start a new conversation thread.")]
    NewChat,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    log::info!("Starting quark_bot...");

    let bot = Bot::from_env();
    let me = bot.get_me().await.expect("Failed to get bot info");
    let bot_username = me.user.username.expect("Bot has no username");
    let db = sled::open("quark_db").expect("Failed to open sled DB");
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    // Clone for different handlers
    let db_for_messages = db.clone();
    let openai_api_key_for_messages = openai_api_key.clone();
    let db_for_callbacks = db.clone();
    let openai_api_key_for_callbacks = openai_api_key.clone();
    use std::sync::Arc;
    let media_aggregator = Arc::new(media_aggregator::MediaGroupAggregator::new(bot.clone(), db.clone(), openai_api_key.clone()));
    let cmd_collector = Arc::new(command_image_collector::CommandImageCollector::new(bot.clone(), db.clone(), openai_api_key.clone()));

    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let bot_username = bot_username.clone();
        let db = db_for_messages.clone();
        let openai_api_key = openai_api_key_for_messages.clone();
        let media_aggregator = media_aggregator.clone();
        let cmd_collector = cmd_collector.clone();
        async move {
            if msg.media_group_id().is_some() && msg.photo().is_some() {
                media_aggregator.add_message(msg).await;
                return respond(());
            }

            // Photo-only message (no text/caption) may belong to a pending command
            if msg.text().is_none() && msg.caption().is_none() && msg.photo().is_some() {
                cmd_collector.clone().try_attach_photo(msg).await;
                return respond(());
            }

            let maybe_text = msg.text().or_else(|| msg.caption());
            if let Some(text) = maybe_text {
                if (text == "/login_user" || text == format!("/login_user@{}", bot_username)) && msg.chat.is_private() {
                    crate::commands::handle_login_user(bot.clone(), msg.clone(), db.clone()).await?;
                } else if let Some(_stripped) = text.strip_prefix(&format!("/chat@{} ", bot_username))
                    .or_else(|| text.strip_prefix("/chat "))
                    .or_else(|| text.strip_prefix(&format!("/c@{} ", bot_username)))
                    .or_else(|| text.strip_prefix("/c "))
                {
                    crate::commands::handle_chat(bot.clone(), msg.clone(), db.clone(), openai_api_key.clone()).await?;
                } else if text == "/chat" || text == format!("/chat@{}", bot_username) || text == "/c" || text == format!("/c@{}", bot_username) {
                    if msg.photo().is_some() {
                        cmd_collector.clone().add_command(msg.clone()).await;
                    } else {
                        bot.send_message(msg.chat.id, "Usage: /chat <your message> or /c <your message>").await?;
                    }
                }
                if (text == "/add_files" || text == format!("/add_files@{}", bot_username)) && msg.chat.is_private() {
                    crate::commands::handle_add_files(bot.clone(), msg.clone()).await?;
                }
                if text == "/help" || text == format!("/help@{}", bot_username) {
                    crate::commands::handle_help(bot.clone(), msg.clone()).await?;
                }
                if text == "/login_group" || text == format!("/login_group@{}", bot_username) {
                    crate::commands::handle_login_group(bot.clone(), msg.clone()).await?;
                }
                if text.starts_with("/list_files") && msg.chat.is_private() {
                    crate::commands::handle_list_files(bot.clone(), msg.clone(), db.clone(), openai_api_key.clone()).await?;
                }
                if text == "/new_chat" || text == format!("/new_chat@{}", bot_username) {
                    crate::commands::handle_new_chat(bot.clone(), msg.clone(), db.clone()).await?;
                }
            }
            if msg.caption().is_none() && msg.chat.is_private() && (msg.document().is_some() || msg.photo().is_some() || msg.video().is_some() || msg.audio().is_some()) {
                crate::file_handling::handle_file_upload(bot.clone(), msg.clone(), db.clone(), openai_api_key.clone()).await?;
            }
            respond(())
        }
    });

    let callback_handler = Update::filter_callback_query().endpoint(move |bot: Bot, query: teloxide::types::CallbackQuery| {
        let db = db_for_callbacks.clone();
        let openai_api_key = openai_api_key_for_callbacks.clone();
        async move {
            crate::callbacks::handle_callback_query(bot.clone(), query.clone(), db.clone(), openai_api_key.clone()).await?;
            respond(())
        }
    });

    Dispatcher::builder(bot, dptree::entry().branch(handler).branch(callback_handler))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
