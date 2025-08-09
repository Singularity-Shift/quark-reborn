use anyhow::Result as AnyResult;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::dependencies::BotDependencies;
use crate::yield_ai::dto::TokenHolding;

const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Split a message into chunks that fit within Telegram's message limit
fn split_message(text: &str) -> Vec<String> {
    if text.len() <= TELEGRAM_MESSAGE_LIMIT {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for line in text.lines() {
        if current_chunk.len() + line.len() + 1 > TELEGRAM_MESSAGE_LIMIT {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                current_chunk.clear();
            }
            current_chunk = line.to_string();
        } else {
            if !current_chunk.is_empty() {
                current_chunk.push('\n');
            }
            current_chunk.push_str(line);
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

/// Send a potentially long message, splitting it into multiple messages if necessary
async fn send_long_message(bot: &Bot, chat_id: ChatId, text: &str) -> AnyResult<()> {
    // Text is already prepared as HTML with proper escaping; send directly
    let chunks = split_message(text);
    for chunk in chunks {
        bot.send_message(chat_id, chunk)
            .parse_mode(ParseMode::Html)
            .await?;
    }
    Ok(())
}

fn token_display_name(token: &TokenHolding) -> String {
    // Prefer symbol, then name, then short token address
    if let Some(symbol) = &token.symbol {
        if !symbol.trim().is_empty() {
            return symbol.clone();
        }
    }
    if let Some(name) = &token.name {
        if !name.trim().is_empty() {
            return name.clone();
        }
    }
    let addr = &token.token_address;
    if addr.len() > 16 {
        format!("{}…{}", &addr[..8], &addr[addr.len() - 6..])
    } else {
        addr.clone()
    }
}

fn format_usd(value: f64) -> String {
    format!("${:.2}", value)
}

/// Show a user's complete portfolio snapshot with per-token holdings and totals
pub async fn handle_balance(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
    is_group: bool,
) -> AnyResult<()> {
    // Validate user and credentials
    let user = match &msg.from {
        Some(u) => u,
        None => {
            bot.send_message(msg.chat.id, "❌ User not found").await?;
            return Ok(());
        }
    };

    let username = match &user.username {
        Some(u) => u,
        None => {
            bot.send_message(msg.chat.id, "❌ Username not found")
                .await?;
            return Ok(());
        }
    };

    let resource_account_address = if is_group {
        let group_credentials = bot_deps.group.get_credentials(msg.chat.id);

        if group_credentials.is_none() {
            bot.send_message(msg.chat.id, "❌ Group not found").await?;
            return Ok(());
        }

        group_credentials.unwrap().resource_account_address
    } else {
        match bot_deps.auth.get_credentials(username) {
            Some(c) => c.resource_account_address,
            None => {
                bot.send_message(
                    msg.chat.id,
                    "❌ Account not linked. Please use /loginuser to connect your wallet.",
                )
                .await?;
                return Ok(());
            }
        }
    };

    // Fetch portfolio snapshot
    let snapshot = bot_deps
        .yield_ai
        .get_portfolio_snapshot(resource_account_address.clone())
        .await;

    let snapshot = match snapshot {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "❌ <b>Failed to retrieve portfolio</b>\n\n<i>Please try again later.</i>\n\n<code>{}</code>",
                    teloxide::utils::html::escape(&e.to_string())
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;
            return Ok(());
        }
    };

    // Compute totals
    let total_tokens = snapshot.tokens.len();
    let total_value_usd: f64 = snapshot.tokens.iter().filter_map(|t| t.value_usd).sum();

    // Sort tokens by USD value (desc), None values go last
    let mut tokens = snapshot.tokens.clone();
    tokens.sort_by(|a, b| match (b.value_usd, a.value_usd) {
        (Some(bv), Some(av)) => bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    // Build header
    let mut message = String::new();
    message.push_str("📊 <b>Portfolio Snapshot</b>\n\n");
    message.push_str(&format!(
        "👤 <b>Address:</b> <code>{}</code>\n",
        teloxide::utils::html::escape(&resource_account_address)
    ));
    message.push_str(&format!(
        "🕒 <b>As of:</b> {}\n",
        teloxide::utils::html::escape(&snapshot.timestamp)
    ));
    message.push_str(&format!("💎 <b>Assets:</b> {} tokens\n", total_tokens));
    message.push_str(&format!(
        "💵 <b>Total Value:</b> {}\n\n",
        format_usd(total_value_usd)
    ));

    // Token list header
    message.push_str("<b>Holdings</b>\n");

    // Add each token line; break into multiple messages if needed
    let mut current_block = message;
    for token in tokens {
        let name = token_display_name(&token);
        let amount = &token.amount;
        let value_str = token
            .value_usd
            .map(format_usd)
            .unwrap_or_else(|| "N/A".to_string());

        // Escape dynamic text
        let name_esc = teloxide::utils::html::escape(&name);
        let amount_esc = teloxide::utils::html::escape(amount);
        let value_esc = teloxide::utils::html::escape(&value_str);

        let line = format!("• <b>{}</b>: {} — {}\n", name_esc, amount_esc, value_esc);

        if current_block.len() + line.len() > TELEGRAM_MESSAGE_LIMIT {
            // send current and start a new block with the header context
            send_long_message(&bot, msg.chat.id, &current_block).await?;
            current_block = String::from("<b>Holdings (cont.)</b>\n");
        }
        current_block.push_str(&line);
    }

    if !current_block.is_empty() {
        // Append footer attribution
        let mut with_footer = current_block;
        with_footer.push_str("\n<i>Powered by Yield AI</i>");
        send_long_message(&bot, msg.chat.id, &with_footer).await?;
    }

    Ok(())
}
