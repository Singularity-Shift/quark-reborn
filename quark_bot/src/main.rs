use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(description = "These commands are supported:")]
enum Command {
    #[command(description = "Chat with the AI.")]
    Chat,
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

    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let bot_username = bot_username.clone();
        async move {
            if let Some(text) = msg.text() {
                if text == "/chat" || text == format!("/chat@{}", bot_username) {
                    bot.send_message(msg.chat.id, "under development").await?;
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
