use crate::dependencies::BotDependencies;
use crate::payment::dto::PaymentPrefs;
use anyhow::Result;
use quark_core::helpers::dto::CoinVersion;
use serde_json;
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
            data if data.starts_with("pay_tokpage:") => {
                handle_payment_pagination(bot, query, bot_deps).await?
            }
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
            let mut default_currency = bot_deps.default_payment_prefs.label;

            let is_admin = crate::utils::is_admin(&bot, m.chat.id, query.from.id).await;

            if !is_admin {
                bot.answer_callback_query(query.id)
                    .text("❌ Only administrators can manage group payment settings")
                    .await?;
                return Ok(());
            }

            let prefs = bot_deps
                .payment
                .get_payment_token_session(m.chat.id.to_string());

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

            // Build entries (label, currency, version)
            let mut entries: Vec<PaymentPrefs> = Vec::new();
            for addr in allowed {
                let token = tokens.iter().find(|t| {
                    t.token_address.as_ref().is_some() && t.token_address.as_ref().unwrap() == &addr
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

            // Save session for pagination
            let sess_tree = bot_deps.db.open_tree("token_selector_sessions").unwrap();
            let sess_key = format!("u{}:c{}:m{}", query.from.id.0, m.chat.id.0, m.id.0);
            let _ = sess_tree.insert(sess_key.as_bytes(), serde_json::to_vec(&entries).unwrap());

            // Render page 0
            const PAGE_SIZE: usize = 5;
            let total = entries.len();
            let total_pages = if total == 0 {
                1
            } else {
                (total + PAGE_SIZE - 1) / PAGE_SIZE
            };
            let page: usize = 0;
            let start = 0;
            let end = std::cmp::min(PAGE_SIZE, total);

            let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
            let mut row: Vec<InlineKeyboardButton> = Vec::new();

            row.push(InlineKeyboardButton::callback(
                "📒 Default",
                format!(
                    "pay_selid-{}-{}-{}",
                    query.from.id.to_string(),
                    bot_deps.default_payment_prefs.currency,
                    bot_deps.default_payment_prefs.version.to_string()
                ),
            ));
            for entry in entries[start..end].iter() {
                row.push(InlineKeyboardButton::callback(
                    entry.label.clone(),
                    format!(
                        "pay_selid-{}-{}-{}",
                        query.from.id.0, entry.currency, entry.version
                    ),
                ));
            }
            rows.push(row);

            // Nav row
            if total_pages > 1 {
                let mut nav: Vec<InlineKeyboardButton> = Vec::new();
                // Next only for first page
                nav.push(InlineKeyboardButton::callback(
                    "Next ▶".to_string(),
                    format!("pay_tokpage:{}", page + 1),
                ));
                rows.push(nav);
            }
            bot.edit_message_text(m.chat.id, m.id, "Select a payment token:")
                .reply_markup(InlineKeyboardMarkup::new(rows))
                .await?;
        }
    }
    Ok(())
}

/// Handle payment token pagination
async fn handle_payment_pagination(
    bot: Bot,
    query: CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    if let Some(message) = &query.message {
        if let MaybeInaccessibleMessage::Regular(m) = message {
            let page: usize = query
                .data
                .as_ref()
                .unwrap()
                .strip_prefix("pay_tokpage:")
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            let sess_tree = bot_deps.db.open_tree("token_selector_sessions").unwrap();
            let sess_key = format!("u{}:c{}:m{}", query.from.id.0, m.chat.id.0, m.id.0);
            let Some(bytes) = sess_tree.get(sess_key.as_bytes()).unwrap_or(None) else {
                return Ok(());
            };
            let entries: Vec<PaymentPrefs> = serde_json::from_slice(&bytes).unwrap_or_default();
            const PAGE_SIZE: usize = 12;
            let total = entries.len();
            let total_pages = if total == 0 {
                1
            } else {
                (total + PAGE_SIZE - 1) / PAGE_SIZE
            };
            let cur_page = std::cmp::min(page, total_pages.saturating_sub(1));
            let start = cur_page * PAGE_SIZE;
            let end = std::cmp::min(start + PAGE_SIZE, total);
            let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
            let mut row: Vec<InlineKeyboardButton> = Vec::new();
            for entry in &entries[start..end] {
                row.push(InlineKeyboardButton::callback(
                    entry.label.clone(),
                    format!(
                        "pay_selid-{}-{}-{}",
                        query.from.id.to_string(),
                        entry.currency.clone(),
                        entry.version.to_string(),
                    ),
                ));
            }
            rows.push(row);
            // Nav row
            if total_pages > 1 {
                let mut nav: Vec<InlineKeyboardButton> = Vec::new();
                if cur_page > 0 {
                    nav.push(InlineKeyboardButton::callback(
                        "◀ Prev".to_string(),
                        format!("pay_tokpage:{}", cur_page - 1),
                    ));
                }
                if cur_page + 1 < total_pages {
                    nav.push(InlineKeyboardButton::callback(
                        "Next ▶".to_string(),
                        format!("pay_tokpage:{}", cur_page + 1),
                    ));
                }
                if !nav.is_empty() {
                    rows.push(nav);
                }
            }

            log::info!("rows: {:?}", rows);
            bot.edit_message_reply_markup(m.chat.id, m.id)
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
    let mut currency = parts[2].to_string();
    let version = parts[3].parse::<CoinVersion>().unwrap();
    let mut label = bot_deps.default_payment_prefs.label;

    if version == CoinVersion::V1 {
        let tokens = bot_deps.panora.get_panora_token_list().await;

        if tokens.is_err() {
            log::error!(
                "Error getting token list: {}",
                tokens.as_ref().err().unwrap()
            );
            return Err(anyhow::anyhow!("Error getting token list"));
        }

        let tokens = tokens.unwrap();

        let token = tokens.iter().find(|t| {
            t.token_address
                .as_ref()
                .unwrap_or(&"".to_string())
                .starts_with(&currency)
        });

        if token.is_none() {
            bot.answer_callback_query(query.id)
                .text("❌ Token not found")
                .await?;
            return Ok(());
        }

        let token = token.unwrap();

        let token_address = token.token_address.clone();

        if token_address.is_some() {
            currency = token_address.unwrap();
        }

        label = token.symbol.clone();
    }

    let prefs = PaymentPrefs::from((label, currency, version));
    if let Some(message) = &query.message {
        if let MaybeInaccessibleMessage::Regular(m) = message {
            if m.chat.is_private() {
                bot_deps.payment.set_payment_token_session(user_id, prefs);

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
                bot_deps.payment.set_payment_token_session(key, prefs);

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
                    "⚙️ <b>Group Settings</b>\n\n• Configure payment token, DAO preferences, and group migration.\n\n💡 Only group administrators can access these settings."
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(kb)
                .await?;
            }
        }
    }
    Ok(())
}
