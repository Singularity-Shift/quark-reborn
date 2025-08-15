use crate::dependencies::BotDependencies;
use anyhow::Result;
use quark_core::helpers::bot_commands::{Command, QuarkState};
use teloxide::{
    Bot,
    dispatching::{DpHandlerDescription, HandlerExt, UpdateFilterExt, dialogue::InMemStorage},
    dptree::{self, Handler},
    prelude::Requester,
    types::{Message, Update},
};

use crate::{
    bot::{answers::answers, handler::handle_message, handler::handle_web_app_data},
    callbacks::handle_callback_query,
    message_history::handler::{store_message, MessageEntry},
};

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

pub fn handler_tree() -> Handler<'static, Result<()>, DpHandlerDescription> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, InMemStorage<QuarkState>, QuarkState>()
                // Record messages with text to message history buffer (groups only, passthrough)
                .inspect_async(|bot_deps: BotDependencies, msg: Message| async move {
                    if let Some(text) = msg.text() {
                        // Only store messages from group chats, never DMs for privacy
                        if !msg.chat.is_private() {
                            let sender_name = msg.from.as_ref().map(|u| u.first_name.clone());
                            let entry = MessageEntry {
                                sender: sender_name,
                                text: text.to_string(),
                            };
                            store_message(msg.chat.id, entry, bot_deps.history_storage.clone()).await;
                        }
                    }
                })
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| {
                            msg.web_app_data().is_some() && msg.chat.is_private()
                        })
                        .endpoint(
                            |bot: Bot, msg: Message, bot_deps: BotDependencies| async move {
                                handle_web_app_data(bot, msg, bot_deps).await
                            }
                        ),
                )
                // 0. Intercept media-group photo messages early so we can aggregate
                //    all images (important for multi-image vision prompts). This
                //    branch must be first so it runs before command parsing.
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| {
                            msg.media_group_id().is_some() && msg.photo().is_some()
                        })
                        .endpoint(
                            |bot_deps: BotDependencies, msg: Message| async move {
                                let media_aggregator = bot_deps.media_aggregator.clone();
                                media_aggregator.add_message(msg, bot_deps.clone())
                                    .await;
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
                                    | Command::Prices
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
                                    | Command::WalletAddress
                                    | Command::Balance(_)
                                    | Command::AddFiles
                                    | Command::ListFiles
                                    | Command::NewChat
                                    | Command::PromptExamples
                                    | Command::Announcement(_)
                            )
                        })
                        .filter_async(|msg: Message, bot_deps: BotDependencies| async move {
                            bot_deps.auth.verify(msg).await
                        })
                        .endpoint(answers),
                )
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .filter(|cmd| {
                            matches!(
                                cmd,
                                Command::G(_) | Command::Groupsettings
                                    | Command::Mod | Command::GroupBalance(_) | Command::GroupWalletAddress | Command::ModerationRules | Command::SchedulePrompt | Command::ListScheduled
                            )
                        })
                        .filter_async(|msg: Message, bot_deps: BotDependencies| async move {
                            bot_deps.group.verify(msg).await
                        })
                        .endpoint(answers),
                )
                .branch(
                    // DM-only authenticated commands
                    dptree::entry()
                        .filter_command::<Command>()
                        .filter(|cmd| { matches!(cmd, Command::Usersettings) })
                        .filter(|msg: Message| msg.chat.is_private())
                        .filter_async(|msg: Message, bot_deps: BotDependencies| async move {
                            bot_deps.auth.verify(msg).await
                        })
                        .endpoint(answers),
                )
                .branch(
                    // Handle DM-only commands when used in groups - direct to DMs
                    dptree::entry()
                        .filter_command::<Command>()
                        .filter(|cmd| { matches!(cmd, Command::Usersettings) })
                        .filter(|msg: Message| !msg.chat.is_private())
                        .endpoint(|bot: Bot, msg: Message| async move {
                            bot.send_message(
                                msg.chat.id,
                                "❌ This command is only available in direct messages (DMs).\n\n💬 Please send me a private message to use this feature."
                            )
                            .await?;
                            Ok(())
                        }),
                )
                // Handle group messages for sentinel auto-moderation
                .branch(
                    dptree::entry()
                        
                        .endpoint(handle_message),
                        )
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .endpoint(handle_unauthenticated),
                ),
        )
        .branch(Update::filter_callback_query().endpoint(
            |bot: Bot,
             query: teloxide::types::CallbackQuery,
             bot_deps: BotDependencies| async move {
                handle_callback_query(bot, query, bot_deps).await
            },
        ))
}
