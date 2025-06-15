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
    bot::answers::{handle_message, handle_web_app_data},
    callbacks::handle_callback_query,
    middleware::auth::auth,
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
            dptree::entry()
                .filter(|msg: Message| msg.web_app_data().is_some() && msg.chat.is_private())
                .endpoint(handle_web_app_data),
        )
        .branch(
            dptree::entry()
                .filter(|msg: Message| msg.media_group_id().is_some() && msg.photo().is_some())
                .endpoint(
                    |media_aggregator: std::sync::Arc<
                        crate::assets::media_aggregator::MediaGroupAggregator,
                    >,
                     ai: quark_core::ai::handler::AI,
                     msg: Message| async move {
                        media_aggregator.add_message(msg, ai).await;
                        Ok(())
                    },
                ),
        )
        .branch(
            // 2. Branch for public commands for new users
            dptree::entry()
                .filter_command::<Command>()
                .filter(|cmd| {
                    matches!(
                        cmd,
                        Command::Help
                            | Command::LoginUser
                            | Command::LoginGroup
                            | Command::AptosConnect
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
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_unauthenticated),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback_query))
        .branch(
            dptree::entry()
                .filter(|msg: Message| msg.chat.is_private())
                .endpoint(handle_message),
        )
}
