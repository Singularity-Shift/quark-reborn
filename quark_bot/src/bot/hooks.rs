use anyhow::Result;
use quark_core::helpers::utils::extract_url_from_markdown;
use reqwest::Url;
use teloxide::{
    Bot,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, WebAppInfo},
};

pub async fn withdraw_funds_hook(bot: Bot, msg: Message, text: String) -> Result<()> {
    let url = extract_url_from_markdown(&text);

    if url.is_none() {
        bot.send_message(msg.chat.id, "❌ Unable to extract URL from response.")
            .await?;
        return Ok(());
    }

    let url = url.unwrap();

    let url = Url::parse(&url).expect("Invalid URL");

    let web_app_info = WebAppInfo { url };

    let withdraw_funds_button = InlineKeyboardButton::web_app("Withdraw Funds", web_app_info);

    let withdraw_funds_markup = InlineKeyboardMarkup::new(vec![vec![withdraw_funds_button]]);

    bot.send_message(msg.chat.id, "Click the button below to withdraw funds")
        .reply_markup(withdraw_funds_markup)
        .await?;

    Ok(())
}

pub async fn fund_account_hook(bot: Bot, msg: Message, text: String) -> Result<()> {
    let url = extract_url_from_markdown(&text);

    if url.is_none() {
        bot.send_message(msg.chat.id, "❌ Unable to extract URL from response.")
            .await?;
        return Ok(());
    }

    let url = url.unwrap();

    let url = Url::parse(&url).expect("Invalid URL");

    let web_app_info = WebAppInfo { url };

    let fund_account_button = InlineKeyboardButton::web_app("Fund Account", web_app_info);

    let fund_account_markup = InlineKeyboardMarkup::new(vec![vec![fund_account_button]]);

    bot.send_message(msg.chat.id, "Click the button below to fund your account")
        .reply_markup(fund_account_markup)
        .await?;
    Ok(())
}
