use anyhow::Result;
use quark_core::helpers::bot_commands::Command;
use teloxide::{Bot, prelude::*, types::Message};

use crate::announcement::handle_announcement;
use super::handler::{
    handle_add_files, handle_chat, handle_help, handle_list_files, handle_login_group,
    handle_login_user, handle_mod, handle_moderation_rules, handle_new_chat, handle_prices,
    handle_reasoning_chat, handle_sentinel,
};

use crate::bot::handler::{
    handle_aptos_connect, handle_balance, handle_group_balance, handle_group_wallet_address,
    handle_migrate_group_id, handle_wallet_address,
};
use crate::dao::handler::handle_dao_preferences;
use crate::dependencies::BotDependencies;
use crate::user_model_preferences::handler::{
    handle_my_settings, handle_select_model, handle_select_reasoning_model,
};

pub async fn answers(
    bot: Bot,
    msg: Message,
    cmd: Command,
    bot_deps: BotDependencies,
) -> Result<()> {
    match cmd {
        Command::AptosConnect => handle_aptos_connect(bot, msg).await?,
        Command::Help => handle_help(bot, msg).await?,
        Command::WalletAddress => handle_wallet_address(bot, msg, bot_deps.clone()).await?,
        Command::Balance(symbol) => handle_balance(bot, msg, &symbol, bot_deps.clone()).await?,
        Command::Prices => handle_prices(bot, msg).await?,
        Command::LoginUser => handle_login_user(bot, msg).await?,
        Command::LoginGroup => handle_login_group(bot, msg, bot_deps.clone()).await?,
        Command::AddFiles => handle_add_files(bot, msg).await?,
        Command::ListFiles => handle_list_files(bot, msg, bot_deps.clone()).await?,
        Command::NewChat => handle_new_chat(bot, msg, bot_deps.clone()).await?,
        Command::C(prompt) => {
            let cmd_collector = bot_deps.cmd_collector.clone();

            if prompt.trim().is_empty() && msg.photo().is_some() {
                cmd_collector.add_command(msg, bot_deps.clone(), None).await;
            } else if prompt.trim().is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Please include a message after /c, e.g. /c What is the weather today?",
                )
                .await?;
            } else {
                handle_chat(bot, msg, prompt, None, bot_deps).await?;
            }
        }
        Command::G(prompt) => {
            let cmd_collector = bot_deps.cmd_collector.clone();

            let users_admin = bot.get_chat_administrators(msg.chat.id).await?;

            let group_id = msg.clone().chat.id.to_string();

            let multimedia_message = msg.clone();

            if !msg.chat.is_group() && !msg.chat.is_supergroup() {
                bot.send_message(msg.chat.id, "This command can only be used in a group.")
                    .await?;
                return Ok(());
            }

            let user = msg.from;

            if user.is_none() {
                bot.send_message(msg.chat.id, "Please login to use this command.")
                    .await?;
                return Ok(());
            }

            let user = user.unwrap();

            let is_admin = users_admin.iter().any(|member| member.user.id == user.id);

            if !is_admin {
                bot.send_message(msg.chat.id, "Only group admins can use this command.")
                    .await?;
                return Ok(());
            }

            if prompt.trim().is_empty() && multimedia_message.photo().is_some() {
                cmd_collector
                    .add_command(multimedia_message, bot_deps.clone(), Some(group_id))
                    .await;
            } else if prompt.trim().is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Please include a message after /g, e.g. /g What is the weather today?",
                )
                .await?;
            } else {
                handle_chat(
                    bot,
                    multimedia_message,
                    prompt,
                    Some(group_id),
                    bot_deps.clone(),
                )
                .await?;
            }
        }
        Command::R(prompt) => {
            let cmd_collector = bot_deps.cmd_collector.clone();

            if prompt.trim().is_empty() && msg.photo().is_some() {
                cmd_collector.add_command(msg, bot_deps.clone(), None).await;
            } else if prompt.trim().is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "Please include a message after /r, e.g. /r Explain quantum entanglement.",
                )
                .await?;
            } else {
                handle_reasoning_chat(bot, msg, prompt, bot_deps.clone()).await?;
            }
        }
        Command::Sentinel(param) => {
            handle_sentinel(bot, msg, param, bot_deps.clone()).await?;
        }
        Command::Mod => {
            handle_mod(bot, msg, bot_deps.clone()).await?;
        }
        Command::ModerationRules => {
            handle_moderation_rules(bot, msg).await?;
        }
        Command::PromptExamples => {
            bot.send_message(msg.chat.id, "Here are some example prompts you can use:\n\n💰 Wallet & Balance:\n- /prompt \"What's my wallet address?\" or /p \"What's my wallet address?\"\n- /prompt \"Show my balance\" or /p \"Show my balance\"\n- /prompt \"Check my SUI balance\" or /p \"Check my SUI balance\"\n- /prompt \"How much do I have?\" or /p \"How much do I have?\"\n\n💸 Transactions:\n- /prompt \"Send 10 SUI to @username\" or /p \"Send 10 SUI to @username\"\n- /prompt \"Withdraw 5 SUI\" or /p \"Withdraw 5 SUI\"\n- /prompt \"Send 100 SUI to everyone\" or /p \"Send 100 SUI to everyone\"\n\n❓ General:\n- /prompt \"What can you help me with?\" or /p \"What can you help me with?\"\n- /prompt \"Explain how this bot works\" or /p \"Explain how this bot works\"\n\n💡 Tip: Use /p as a shortcut for /prompt!").await?;
            ()
        }
        Command::SelectModel => handle_select_model(bot, msg).await?,
        Command::SelectReasoningModel => handle_select_reasoning_model(bot, msg).await?,
        Command::MySettings => handle_my_settings(bot, msg, bot_deps.clone()).await?,
        Command::GroupWalletAddress => {
            handle_group_wallet_address(bot, msg, bot_deps.clone()).await?;
        }
        Command::GroupBalance(symbol) => {
            handle_group_balance(bot, msg, bot_deps.clone(), &symbol).await?;
        }
        Command::DaoPreferences => {
            handle_dao_preferences(bot, msg, bot_deps.clone()).await?;
        }
        Command::Announcement(text) => {
            handle_announcement(bot, msg, text, bot_deps.clone()).await?;
        }
        Command::MigrateGroupId => {
            handle_migrate_group_id(bot, msg, bot_deps.clone()).await?;
        }
    };
    Ok(())
}
