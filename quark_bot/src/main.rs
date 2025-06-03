use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use quark_backend::ai::chat_with_ai;
use std::env;

#[derive(BotCommands, Clone)]
#[command(description = "These commands are supported:")]
enum Command {
    #[command(description = "Display this text.")]
    Help,
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

    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let bot_username = bot_username.clone();
        let db = db.clone();
        let openai_api_key = openai_api_key.clone();
        async move {
            if let Some(text) = msg.text() {
                if let Some(stripped) = text.strip_prefix(&format!("/chat@{} ", bot_username))
                    .or_else(|| text.strip_prefix("/chat "))
                {
                    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0) as i64;
                    let reply = match chat_with_ai(user_id, stripped, &db, &openai_api_key).await {
                        Ok(resp) => resp,
                        Err(e) => format!("[AI error]: {}", e),
                    };
                    bot.send_message(msg.chat.id, reply).await?;
                } else if text == "/chat" || text == format!("/chat@{}", bot_username) {
                    bot.send_message(msg.chat.id, "Usage: /chat <your message>").await?;
                }
                if text == "/help" || text == format!("/help@{}", bot_username) {
                    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
                }
            }
            respond(())
        }
    });

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
