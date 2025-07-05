use crate::ai::handler::AI;
use crate::user_conversation::handler::UserConversations;
use crate::user_model_preferences::handler::UserModelPreferences;
use anyhow::Result;
use quark_core::helpers::bot_commands::Command;
use sled::{Db, Tree};
use std::sync::Arc;
use teloxide::{Bot, prelude::*, types::Message};

use super::handler::{
    handle_add_files, handle_chat, handle_help, handle_list_files, handle_login_group,
    handle_login_user, handle_new_chat, handle_reasoning_chat, handle_mod, handle_sentinal,
};
use crate::assets::command_image_collector::CommandImageCollector;
use crate::bot::handler::handle_aptos_connect;
use crate::user_model_preferences::handler::{
    handle_my_settings, handle_select_model, handle_select_reasoning_model,
};

pub async fn answers(
    bot: Bot,
    msg: Message,
    cmd: Command,
    db: Db,
    tree: Tree,
    user_convos: UserConversations,
    user_model_prefs: UserModelPreferences,
    ai: AI,
    cmd_collector: Arc<CommandImageCollector>,
) -> Result<()> {
    match cmd {
        Command::AptosConnect => handle_aptos_connect(bot, msg).await?,
        Command::Help => handle_help(bot, msg).await?,
        Command::LoginUser => handle_login_user(bot, msg).await?,
        Command::LoginGroup => handle_login_group(bot, msg).await?,
        Command::AddFiles => handle_add_files(bot, msg).await?,
        Command::ListFiles => handle_list_files(bot, msg, db, user_convos).await?,
        Command::NewChat => handle_new_chat(bot, msg, user_convos).await?,
        Command::C(prompt) => {
            if prompt.trim().is_empty() && msg.photo().is_some() {
                cmd_collector.add_command(ai, msg, tree).await;
            } else if prompt.trim().is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Please include a message after /c, e.g. /c What is the weather today?",
                )
                .await?;
            } else {
                handle_chat(bot, msg, ai, db, tree, user_model_prefs.clone(), prompt).await?;
            }
        }
        Command::R(prompt) => {
            if prompt.trim().is_empty() && msg.photo().is_some() {
                cmd_collector.add_command(ai, msg, tree).await;
            } else if prompt.trim().is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Please include a message after /r, e.g. /r Explain quantum entanglement.",
                )
                .await?;
            } else {
                handle_reasoning_chat(bot, msg, ai, db, tree, user_model_prefs.clone(), prompt)
                    .await?;
            }
        }
        Command::Sentinal(param) => {
            handle_sentinal(bot, msg, param, db).await?;
        }
        Command::Mod => {
            handle_mod(bot, msg, db).await?;
        }
        Command::PromptExamples => {
            bot.send_message(msg.chat.id, "Here are some example prompts you can use:\n\n💰 Wallet & Balance:\n- /prompt \"What's my wallet address?\" or /p \"What's my wallet address?\"\n- /prompt \"Show my balance\" or /p \"Show my balance\"\n- /prompt \"Check my SUI balance\" or /p \"Check my SUI balance\"\n- /prompt \"How much do I have?\" or /p \"How much do I have?\"\n\n💸 Transactions:\n- /prompt \"Send 10 SUI to @username\" or /p \"Send 10 SUI to @username\"\n- /prompt \"Withdraw 5 SUI\" or /p \"Withdraw 5 SUI\"\n- /prompt \"Send 100 SUI to everyone\" or /p \"Send 100 SUI to everyone\"\n\n❓ General:\n- /prompt \"What can you help me with?\" or /p \"What can you help me with?\"\n- /prompt \"Explain how this bot works\" or /p \"Explain how this bot works\"\n\n💡 Tip: Use /p as a shortcut for /prompt!").await?;
            ()
        }
        Command::SelectModel => handle_select_model(bot, msg, user_model_prefs.clone()).await?,
        Command::SelectReasoningModel => {
            handle_select_reasoning_model(bot, msg, user_model_prefs.clone()).await?
        }
        Command::MySettings => handle_my_settings(bot, msg, user_model_prefs.clone()).await?,
    };
    Ok(())
}
