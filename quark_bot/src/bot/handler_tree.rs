use crate::ai::handler::AI;
use crate::user_conversation::handler::UserConversations;
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
    middleware::auth::auth,
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
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| {
                            msg.web_app_data().is_some() && msg.chat.is_private()
                        })
                        .endpoint(
                            |bot: Bot, msg: Message, tree: sled::Tree, db: sled::Db, user_model_prefs: crate::user_model_preferences::handler::UserModelPreferences| async move {
                                handle_web_app_data(bot, msg, tree, db, user_model_prefs).await
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
                            |media_aggregator: std::sync::Arc<
                                crate::assets::media_aggregator::MediaGroupAggregator,
                            >,
                             msg: Message,| async move {
                                media_aggregator.add_message(msg).await;
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
                                    | Command::G(_)
                                    | Command::R(_)
                                    | Command::WalletAddress
                                    | Command::Balance(_)
                                    | Command::AddFiles
                                    | Command::ListFiles
                                    | Command::NewChat
                                    | Command::PromptExamples
                                    | Command::Sentinal(_)
                                    | Command::Mod
                                    | Command::ModerationRules
                            )
                        })
                        .filter_async(auth)
                        .endpoint(answers),
                )
                .branch(
                    // DM-only authenticated commands
                    dptree::entry()
                        .filter_command::<Command>()
                        .filter(|cmd| {
                            matches!(
                                cmd,
                                Command::SelectModel | Command::SelectReasoningModel | Command::MySettings
                            )
                        })
                        .filter(|msg: Message| msg.chat.is_private())
                        .filter_async(auth)
                        .endpoint(answers),
                )
                // Fallback for any non-command message in PRIVATE CHATS.
                // This ensures we still capture file uploads (documents, photos, etc.)
                // sent via DM while ignoring non-command chatter in groups.
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| msg.chat.is_private())
                        .endpoint(handle_message),
                )
                // Handle group messages for sentinal auto-moderation
                .branch(
                    dptree::entry()
                        .filter(|msg: Message| !msg.chat.is_private())
                        .endpoint(handle_message),
                )
                .branch(
                    // Handle DM-only commands when used in groups - direct to DMs
                    dptree::entry()
                        .filter_command::<Command>()
                        .filter(|cmd| {
                            matches!(
                                cmd,
                                Command::SelectModel | Command::SelectReasoningModel | Command::MySettings
                            )
                        })
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
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .endpoint(handle_unauthenticated),
                ),
        )
        .branch(Update::filter_callback_query().endpoint(
            |bot: Bot,
             query: teloxide::types::CallbackQuery,
             db: sled::Db,
             user_convos: UserConversations,
             user_model_prefs: crate::user_model_preferences::handler::UserModelPreferences,
             ai: AI| async move {
                handle_callback_query(bot, query, db, user_convos, user_model_prefs, ai).await
            },
        ))
}
