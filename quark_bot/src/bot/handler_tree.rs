use anyhow::Result;
use quark_core::helpers::bot_commands::{Command, QuarkState};
use teloxide::{
    Bot,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt, dialogue::InMemStorage},
    dptree::{self, Handler},
    prelude::{DependencyMap, Requester},
    types::{Message, Update},
};

use crate::{
    bot::answers::handle_message, callbacks::handle_callback_query, middleware::auth::auth,
};

use super::answers::answers;

async fn handle_unauthenticated(bot: Bot, msg: Message) -> Result<()> {
    bot.send_message(
        msg.chat.id,
        "👋 Welcome to Quark! 

To use commands like `/c` or `/newchat`, you need to authenticate first.

Please use `/login` to authenticate.",
    )
    .await?;
    Ok(())
}

pub fn handler_tree() -> Handler<'static, DependencyMap, Result<()>, DpHandlerDescription> {
    Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<QuarkState>, QuarkState>()
        .branch(
            // 2. Branch for public commands for new users
            dptree::entry()
                .filter_command::<Command>()
                .filter(|cmd| {
                    matches!(
                        cmd,
                        Command::Help | Command::LoginUser | Command::LoginGroup
                    )
                })
                .endpoint(answers),
        )
        .branch(
            // 1. Branch for authenticated users
            dptree::entry()
                .filter_command::<Command>()
                .filter(|cmd| {
                    matches!(
                        cmd,
                        Command::C(_)
                            | Command::AddFiles
                            | Command::ListFiles
                            | Command::NewChat
                            | Command::PromptExamples
                    )
                })
                .filter_async(auth)
                .endpoint(answers),
        )
        // Fallback for any other message that isn't a recognized slash-command.
        // This lets us capture plain attachments (documents, photos, etc.) that
        // are required for the /addfiles flow.
        .branch(
            dptree::entry()
                .endpoint(handle_message),
        )
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_unauthenticated),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback_query))
}
