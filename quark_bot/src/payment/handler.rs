use crate::dependencies::BotDependencies;
use crate::payment::dto::PaymentPrefs;
use anyhow::Result;
use quark_core::helpers::dto::CoinVersion;
use teloxide::{
    Bot,
    prelude::*,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, MaybeInaccessibleMessage},
};

/// Main payment handler that routes all payment-related callbacks
pub async fn handle_payment(
    bot: Bot,
    query: CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(data) = &query.data {
        match data.as_str() {
            "open_payment_settings" => handle_open_payment_settings(bot, query, bot_deps).await?,
            "open_group_payment_settings" => {
                handle_open_group_payment_settings(bot, query, bot_deps).await?
            }
            "payment_selected" => handle_payment_selected(bot, query, bot_deps).await?,
            data if data.starts_with("pay_selid-") => {
                handle_payment_selection(bot, query, bot_deps).await?
            }
            _ => {
                bot.answer_callback_query(query.id)
                    .text("❌ Unknown payment action")
                    .await?;
            }
        }
    }
    Ok(())
}

/// Handle opening user payment settings
async fn handle_open_payment_settings(
    bot: Bot,
    query: CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(message) = &query.message {
        if let MaybeInaccessibleMessage::Regular(m) = message {
            let default_currency = match bot_deps.panora.aptos.get_token_address().await {
                Ok(addr) => addr,
                Err(_) => "0x1::aptos_coin::AptosCoin".to_string(),
            };
            let kb = InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::callback(
                    "💳 Choose Payment Token",
                    "payment_selected",
                )],
                vec![InlineKeyboardButton::callback(
                    "↩️ Back",
                    "back_to_user_settings",
                )],
            ]);
            bot.edit_message_text(
                m.chat.id,
                m.id,
                format!(
                    "💳 <b>Payment Settings</b>\n\nDefault currency: <code>{}</code>",
                    default_currency
                ),
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .reply_markup(kb)
            .await?;
        }
    }
    Ok(())
}

/// Handle opening group payment settings
async fn handle_open_group_payment_settings(
    bot: Bot,
    query: CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(message) = &query.message {
        if let MaybeInaccessibleMessage::Regular(m) = message {
            let mut default_currency = bot_deps.default_payment_prefs.label.clone();

            let is_admin = crate::utils::is_admin(&bot, m.chat.id, query.from.id).await;

            if !is_admin {
                bot.answer_callback_query(query.id)
                    .text("❌ Only administrators can manage group payment settings")
                    .await?;
                return Ok(());
            }

            let prefs = bot_deps
                .payment
                .get_payment_token(m.chat.id.to_string(), &bot_deps)
                .await;

            log::info!("prefs: {:?}", prefs);

            if prefs.is_some() {
                let prefs = prefs.unwrap();
                default_currency = prefs.label;
            }

            let kb = InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::callback(
                    "💳 Choose Group Payment Token",
                    "payment_selected",
                )],
                vec![InlineKeyboardButton::callback(
                    "↩️ Back",
                    "back_to_group_settings",
                )],
            ]);

            log::info!("default_currency: {:?}", default_currency);
            bot.edit_message_text(
                m.chat.id,
                m.id,
                format!(
                    "💳 <b>Group Payment Settings</b>\n\nDefault currency: <code>{}</code>",
                    default_currency
                ),
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .reply_markup(kb)
            .await?;
        }
    }
    Ok(())
}

/// Handle payment token selection (shows the token list)
async fn handle_payment_selected(
    bot: Bot,
    query: CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(message) = &query.message {
        if let MaybeInaccessibleMessage::Regular(m) = message {
            if m.chat.is_group() || m.chat.is_supergroup() {
                let is_admin = crate::utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage group payment settings")
                        .await?;
                    return Ok(());
                }
            }

            let allowed = match bot_deps.panora.aptos.get_fees_currency_payment_list().await {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to load allowed tokens: {}", e);
                    bot.answer_callback_query(query.id)
                        .text("❌ Failed to load allowed tokens")
                        .await?;
                    return Ok(());
                }
            };
            let tokens = match bot_deps.panora.get_panora_token_list().await {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to load token list: {}", e);
                    bot.answer_callback_query(query.id)
                        .text("❌ Failed to load token list")
                        .await?;
                    return Ok(());
                }
            };

            log::info!("allowed: {:?}", allowed);

            // Build entries (label, currency, version)
            let mut entries: Vec<PaymentPrefs> = Vec::new();
            for mut addr in allowed {
                if addr == "0x1" {
                    addr = "0x1::aptos_coin::AptosCoin".to_string();
                }

                let token = tokens.iter().find(|t| {
                    t.token_address.as_ref().is_some()
                        && t.token_address.as_ref().unwrap().starts_with(&addr)
                        || t.fa_address == addr
                });

                if let Some(t) = token {
                    let label = if t.symbol.trim().is_empty() {
                        t.panora_symbol.clone()
                    } else {
                        t.symbol.clone()
                    };

                    let version = if t.token_address.is_some() {
                        CoinVersion::V1
                    } else {
                        CoinVersion::V2
                    };

                    entries.push(PaymentPrefs {
                        label,
                        currency: addr.clone(),
                        version,
                    });
                }
            }

            let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
            let mut row: Vec<InlineKeyboardButton> = Vec::new();

            row.push(InlineKeyboardButton::callback(
                format!("{} (Default)", bot_deps.default_payment_prefs.label),
                format!(
                    "pay_selid-{}-{}-{}",
                    query.from.id.to_string(),
                    bot_deps.default_payment_prefs.label,
                    bot_deps.default_payment_prefs.version.to_string()
                ),
            ));
            for entry in entries.iter() {
                row.push(InlineKeyboardButton::callback(
                    entry.label.clone(),
                    format!(
                        "pay_selid-{}-{}-{}",
                        query.from.id.to_string(),
                        entry.label,
                        entry.version.to_string()
                    ),
                ));
            }
            rows.push(row);

            bot.edit_message_text(m.chat.id, m.id, "Select a payment token:")
                .reply_markup(InlineKeyboardMarkup::new(rows))
                .await?;
        }
    }
    Ok(())
}

/// Handle payment token selection from the list
async fn handle_payment_selection(
    bot: Bot,
    query: CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    let parts: Vec<&str> = query.data.as_ref().unwrap().split('-').collect();
    if parts.len() != 4 {
        bot.answer_callback_query(query.id)
            .text("❌ Invalid callback data")
            .await?;
        return Ok(());
    }
    let user_id = parts[1].to_string();
    let mut label = parts[2].to_string();
    let version = parts[3].parse::<CoinVersion>().unwrap();

    let token = bot_deps.panora.get_token_by_symbol(&label).await;

    if token.is_err() {
        log::error!("Error getting token: {}", token.as_ref().err().unwrap());
        return Err(anyhow::anyhow!("Error getting token"));
    }

    let token = token.unwrap();

    let currency = if token.token_address.is_some() {
        token.token_address.as_ref().unwrap().to_string()
    } else {
        token.fa_address
    };

    label = token.symbol.clone();

    let prefs = PaymentPrefs::from((label, currency, version));
    if let Some(message) = &query.message {
        if let MaybeInaccessibleMessage::Regular(m) = message {
            if m.chat.is_private() {
                bot_deps.payment.set_payment_token(user_id, prefs);

                // Show popup notification instead of closing
                bot.answer_callback_query(query.id)
                    .text("✅ Payment token saved for your account")
                    .await?;

                // Return to user settings menu with close option
                let kb = InlineKeyboardMarkup::new(vec![
                    vec![InlineKeyboardButton::callback(
                        "🧠 Select Model",
                        "open_select_model",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "💳 Payment Settings",
                        "open_payment_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "📋 View My Settings",
                        "open_my_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "↩️ Close",
                        "user_settings_close",
                    )],
                ]);

                bot.edit_message_text(
                    m.chat.id,
                    m.id,
                    "⚙️ <b>User Settings</b>\n\n• Manage your model, view current settings, and configure payment.\n\n💡 If no payment token is selected, the on-chain default will be used."
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(kb)
                .await?;
            } else if m.chat.is_group() || m.chat.is_supergroup() {
                let is_admin = crate::utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage group payment settings")
                        .await?;
                    return Ok(());
                }

                let key = m.chat.id.to_string();
                log::info!("prefs: {:?}", prefs);
                bot_deps.payment.set_payment_token(key, prefs);

                // Show popup notification instead of closing
                bot.answer_callback_query(query.id)
                    .text("✅ Group payment token saved")
                    .await?;

                // Return to group settings menu with close option
                let kb = InlineKeyboardMarkup::new(vec![
                    vec![InlineKeyboardButton::callback(
                        "💳 Payment Settings",
                        "open_group_payment_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "🏛️ DAO Preferences",
                        "open_dao_preferences",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "🛡️ Moderation",
                        "open_moderation_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "🎯 Sponsor Settings",
                        "open_sponsor_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "👋 Welcome Settings",
                        "welcome_settings",
                    )],
                    vec![InlineKeyboardButton::callback("🔍 Filters", "filters_main")],
                    vec![InlineKeyboardButton::callback(
                        "⚙️ Command Settings",
                        "open_command_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "📋 Summarization Settings",
                        "open_group_summarization_settings",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "🔄 Migrate Group ID",
                        "open_migrate_group_id",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "↩️ Close",
                        "group_settings_close",
                    )],
                ]);

                bot.edit_message_text(
                    m.chat.id,
                    m.id,
                    "⚙️ <b>Group Settings</b>\n\n• Configure payment token, DAO preferences, moderation, sponsor settings, command settings, filters, and group migration.\n\n💡 Only group administrators can access these settings."
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(kb)
                .await?;
            }
        }
    }
    Ok(())
}
