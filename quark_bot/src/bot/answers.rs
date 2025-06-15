use anyhow::Result;
use quark_core::ai::handler::AI;
use quark_core::helpers::bot_commands::Command;
use quark_core::helpers::jwt::JwtManager;
use quark_core::user_conversation::handler::UserConversations;
use sled::{Db, Tree};
use std::sync::Arc;
use teloxide::{Bot, prelude::*, types::Message};

use super::handler::{
    handle_add_files, handle_chat, handle_help, handle_list_files, handle_login_group,
    handle_login_user, handle_new_chat,
};
use crate::assets::command_image_collector::CommandImageCollector;
use crate::assets::handler::handle_file_upload;
use crate::assets::media_aggregator::MediaGroupAggregator;
use crate::bot::handler::handle_aptos_connect;
use crate::credentials::helpers::generate_new_jwt;

pub async fn answers(
    bot: Bot,
    msg: Message,
    cmd: Command,
    db: Db,
    user_convos: UserConversations,
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
                cmd_collector.add_command(ai, msg).await;
            } else {
                handle_chat(bot, msg, ai, db, prompt).await?;
            }
        }
        Command::PromptExamples => {
            bot.send_message(msg.chat.id, "Here are some example prompts you can use:\n\n💰 Wallet & Balance:\n- /prompt \"What's my wallet address?\" or /p \"What's my wallet address?\"\n- /prompt \"Show my balance\" or /p \"Show my balance\"\n- /prompt \"Check my SUI balance\" or /p \"Check my SUI balance\"\n- /prompt \"How much do I have?\" or /p \"How much do I have?\"\n\n💸 Transactions:\n- /prompt \"Send 10 SUI to @username\" or /p \"Send 10 SUI to @username\"\n- /prompt \"Withdraw 5 SUI\" or /p \"Withdraw 5 SUI\"\n- /prompt \"Send 100 SUI to everyone\" or /p \"Send 100 SUI to everyone\"\n\n❓ General:\n- /prompt \"What can you help me with?\" or /p \"What can you help me with?\"\n- /prompt \"Explain how this bot works\" or /p \"Explain how this bot works\"\n\n💡 Tip: Use /p as a shortcut for /prompt!").await?;
            ()
        }
    };
    Ok(())
}

pub async fn handle_web_app_data(bot: Bot, msg: Message, tree: Tree) -> Result<()> {
    let web_app_data = msg.web_app_data().unwrap();
    let account_address = web_app_data.data.clone();

    let user = msg.from;

    if user.is_none() {
        bot.send_message(msg.chat.id, "❌ User not found").await?;
        return Ok(());
    }

    let user = user.unwrap();

    let username = user.username;

    if username.is_none() {
        bot.send_message(msg.chat.id, "❌ Username not found, required for login")
            .await?;
        return Ok(());
    }

    let username = username.unwrap();

    let user_id = user.id;

    let jwt_manager = JwtManager::new();

    generate_new_jwt(username, user_id, account_address, jwt_manager, tree).await;

    return Ok(());
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    ai: AI,
    media_aggregator: Arc<MediaGroupAggregator>,
    cmd_collector: Arc<CommandImageCollector>,
    db: Db,
) -> Result<()> {
    if msg.media_group_id().is_some() && msg.photo().is_some() {
        media_aggregator.add_message(msg, ai).await;
        return Ok(());
    }

    // Photo-only message (no text/caption) may belong to a pending command
    if msg.text().is_none() && msg.caption().is_none() && msg.photo().is_some() {
        cmd_collector.try_attach_photo(msg, ai).await;
        return Ok(());
    }

    if msg.caption().is_none()
        && msg.chat.is_private()
        && (msg.document().is_some()
            || msg.photo().is_some()
            || msg.video().is_some()
            || msg.audio().is_some())
    {
        handle_file_upload(bot, msg, db, ai).await?;
    }
    Ok(())
}
