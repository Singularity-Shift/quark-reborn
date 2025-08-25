use crate::dependencies::BotDependencies;
use crate::sponsor::dto::{SponsorInterval, SponsorRequest, SponsorState, SponsorStep};
use crate::utils;
use anyhow::Result;
use teloxide::types::MessageId;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};

pub async fn handle_sponsor_settings_callback(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    let data = query.data.as_ref().unwrap();

    if data == "open_sponsor_settings" {
        // Open Sponsor Settings submenu inside Group Settings
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                // Admin check
                let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage sponsor settings")
                        .await?;
                    return Ok(());
                }

                show_sponsor_settings(&bot, m.chat.id, m.id, &bot_deps, &m.chat.id.to_string())
                    .await?;
            }
        }
    } else if data == "sponsor_set_requests" {
        // Enter request limit input mode
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage sponsor settings")
                        .await?;
                    return Ok(());
                }

                let group_id = m.chat.id.to_string();
                let admin_user_id = query.from.id.0;

                // Set sponsor state to await request limit input
                let sponsor_state = SponsorState {
                    group_id: group_id.clone(),
                    step: SponsorStep::AwaitingRequestLimit,
                    message_id: Some(m.id.0 as u32),
                    admin_user_id: Some(admin_user_id),
                };

                if let Err(e) = bot_deps
                    .sponsor
                    .set_sponsor_state(group_id.clone(), sponsor_state)
                {
                    bot.answer_callback_query(query.id)
                        .text(&format!("❌ Failed to start input mode: {}", e))
                        .await?;
                    return Ok(());
                }

                let kb = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
                    "❌ Cancel",
                    "sponsor_cancel_input",
                )]]);

                bot.edit_message_text(
                    m.chat.id,
                    m.id,
                    "📊 <b>Set Request Limit</b>\n\n💬 <b>Reply to this message with a number</b>\n\n• Enter the number of requests users can make per interval\n• Must be 1 or greater\n• Examples: 5, 10, 25, 100\n\n⚠️ <i>Only numeric input is accepted</i>",
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(kb)
                .await?;

                bot.answer_callback_query(query.id)
                    .text("✅ Enter the request limit number in your reply")
                    .await?;
            }
        }
    } else if data == "sponsor_cancel_input" {
        // Cancel the input mode and return to sponsor settings
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage sponsor settings")
                        .await?;
                    return Ok(());
                }

                let group_id = m.chat.id.to_string();

                // Remove sponsor state
                if let Err(e) = bot_deps.sponsor.remove_sponsor_state(group_id.clone()) {
                    log::warn!("Failed to remove sponsor state: {}", e);
                }

                // Return to sponsor settings
                show_sponsor_settings(&bot, m.chat.id, m.id, &bot_deps, &group_id).await?;

                bot.answer_callback_query(query.id)
                    .text("❌ Input mode cancelled")
                    .await?;
            }
        }
    } else if data == "sponsor_set_interval" {
        // Show interval options
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage sponsor settings")
                        .await?;
                    return Ok(());
                }

                let group_id = m.chat.id.to_string();
                let admin_user_id = query.from.id.0;

                // Set sponsor state to await interval selection
                let sponsor_state = SponsorState {
                    group_id: group_id.clone(),
                    step: SponsorStep::AwaitingInterval,
                    message_id: Some(m.id.0 as u32),
                    admin_user_id: Some(admin_user_id),
                };

                if let Err(e) = bot_deps
                    .sponsor
                    .set_sponsor_state(group_id.clone(), sponsor_state)
                {
                    bot.answer_callback_query(query.id)
                        .text(&format!("❌ Failed to start interval selection: {}", e))
                        .await?;
                    return Ok(());
                }

                let kb = InlineKeyboardMarkup::new(vec![
                    vec![InlineKeyboardButton::callback(
                        "⏰ Hourly",
                        "sponsor_interval_hourly",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "📅 Daily",
                        "sponsor_interval_daily",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "📆 Weekly",
                        "sponsor_interval_weekly",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "🗓️ Monthly",
                        "sponsor_interval_monthly",
                    )],
                    vec![InlineKeyboardButton::callback(
                        "↩️ Back",
                        "sponsor_cancel_input",
                    )],
                ]);

                bot.edit_message_text(
                    m.chat.id,
                    m.id,
                    "⏰ <b>Set Interval</b>\n\nChoose how often the request limit resets:",
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(kb)
                .await?;
            }
        }
    } else if data.starts_with("sponsor_interval_") {
        // Handle interval selection
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage sponsor settings")
                        .await?;
                    return Ok(());
                }

                // Check if this admin is the one who started the interval selection
                let group_id = m.chat.id.to_string();
                if let Some(sponsor_state) = bot_deps.sponsor.get_sponsor_state(group_id.clone()) {
                    if let Some(admin_user_id) = sponsor_state.admin_user_id {
                        if admin_user_id != query.from.id.0 {
                            bot.answer_callback_query(query.id)
                                .text("❌ Only the admin who started this action can complete it")
                                .await?;
                            return Ok(());
                        }
                    }
                }

                let interval = match data.strip_prefix("sponsor_interval_").unwrap() {
                    "hourly" => SponsorInterval::Hourly,
                    "daily" => SponsorInterval::Daily,
                    "weekly" => SponsorInterval::Weekly,
                    "monthly" => SponsorInterval::Monthly,
                    _ => SponsorInterval::Hourly,
                };

                let mut settings = bot_deps.sponsor.get_sponsor_settings(group_id.clone());
                settings.interval = interval.clone();

                if let Err(e) = bot_deps
                    .sponsor
                    .set_or_update_sponsor_settings(group_id.clone(), settings.clone())
                {
                    bot.answer_callback_query(query.id)
                        .text(&format!("❌ Failed to update settings: {}", e))
                        .await?;
                    return Ok(());
                }

                // Reset requests to full amount when interval changes since it's a new period
                let new_requests = SponsorRequest {
                    requests_left: settings.requests,
                    last_request: chrono::Utc::now().timestamp() as u64,
                };

                if let Err(e) = bot_deps
                    .sponsor
                    .set_or_update_sponsor_requests(group_id.clone(), new_requests)
                {
                    log::warn!("Failed to reset requests after interval change: {}", e);
                }

                // Clear the sponsor state
                if let Err(e) = bot_deps.sponsor.remove_sponsor_state(group_id.clone()) {
                    log::warn!("Failed to remove sponsor state: {}", e);
                }

                let interval_text = match interval {
                    SponsorInterval::Hourly => "hourly",
                    SponsorInterval::Daily => "daily",
                    SponsorInterval::Weekly => "weekly",
                    SponsorInterval::Monthly => "monthly",
                };

                bot.answer_callback_query(query.id)
                    .text(&format!("✅ Interval set to {}", interval_text))
                    .await?;

                // Return to sponsor settings
                show_sponsor_settings(&bot, m.chat.id, m.id, &bot_deps, &group_id).await?;
            }
        }
    } else if data == "sponsor_disable" {
        // Disable sponsor by setting requests to 0
        if let Some(message) = &query.message {
            if let teloxide::types::MaybeInaccessibleMessage::Regular(m) = message {
                let is_admin = utils::is_admin(&bot, m.chat.id, query.from.id).await;
                if !is_admin {
                    bot.answer_callback_query(query.id)
                        .text("❌ Only administrators can manage sponsor settings")
                        .await?;
                    return Ok(());
                }

                let group_id = m.chat.id.to_string();
                let mut settings = bot_deps.sponsor.get_sponsor_settings(group_id.clone());
                settings.requests = 0;

                if let Err(e) = bot_deps
                    .sponsor
                    .set_or_update_sponsor_settings(group_id.clone(), settings)
                {
                    bot.answer_callback_query(query.id)
                        .text(&format!("❌ Failed to disable sponsor: {}", e))
                        .await?;
                    return Ok(());
                }

                // Reset current requests to 0 as well
                let new_requests = crate::sponsor::dto::SponsorRequest {
                    requests_left: 0,
                    last_request: chrono::Utc::now().timestamp() as u64,
                };

                if let Err(e) = bot_deps
                    .sponsor
                    .set_or_update_sponsor_requests(group_id.clone(), new_requests)
                {
                    log::warn!("Failed to reset current requests: {}", e);
                }

                bot.answer_callback_query(query.id)
                    .text("🚫 Sponsor disabled successfully!")
                    .await?;

                // Return to sponsor settings
                show_sponsor_settings(&bot, m.chat.id, m.id, &bot_deps, &group_id).await?;
            }
        }
    }

    Ok(())
}

async fn show_sponsor_settings(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    bot_deps: &BotDependencies,
    group_id: &str,
) -> Result<()> {
    let settings = bot_deps.sponsor.get_sponsor_settings(group_id.to_string());
    let (requests_left, total_requests) = bot_deps
        .sponsor
        .get_request_status(group_id.to_string())
        .unwrap_or((0, 0));

    let interval_text = match settings.interval {
        SponsorInterval::Hourly => "Hourly",
        SponsorInterval::Daily => "Daily",
        SponsorInterval::Weekly => "Weekly",
        SponsorInterval::Monthly => "Monthly",
    };

    let text = format!(
        "🎯 <b>Sponsor Settings</b>\n\n\
        <b>Current Status:</b>\n\
        • Total Requests: <b>{}</b>\n\
        • Requests Left: <b>{}</b>\n\
        • Interval: <b>{}</b>\n\n\
        <b>How it works:</b>\n\
        • Users can use <code>/g</code> command\n\
        • No registration required\n\
        • Requests reset every interval\n\
        • Only admins can change settings\n\n\
        Choose an action below:",
        total_requests, requests_left, interval_text
    );

    let kb = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "📊 Set Request Limit",
            "sponsor_set_requests",
        )],
        vec![InlineKeyboardButton::callback(
            "⏰ Set Interval",
            "sponsor_set_interval",
        )],
        vec![InlineKeyboardButton::callback(
            "🚫 Disable Sponsor",
            "sponsor_disable",
        )],
        vec![InlineKeyboardButton::callback(
            "↩️ Back",
            "back_to_group_settings",
        )],
    ]);

    bot.edit_message_text(chat_id, message_id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(kb)
        .await?;

    Ok(())
}

pub async fn handle_sponsor_message(
    bot: &Bot,
    msg: &Message,
    bot_deps: &BotDependencies,
    current_group_id: String,
    user_id: UserId,
    group_id: ChatId,
) -> Result<bool> {
    // Check if there's an active sponsor input mode for this group
    if let Some(sponsor_state) = bot_deps.sponsor.get_sponsor_state(current_group_id.clone()) {
        // Only process if the user is an admin
        let is_admin = utils::is_admin(&bot, group_id, user_id).await;
        if !is_admin {
            // Non-admin users typing during sponsor setup - ignore silently
            return Ok(false);
        }

        // Check if this admin is the one who started the action
        if let Some(admin_user_id) = sponsor_state.admin_user_id {
            if admin_user_id != user_id.0 {
                // Other admin users typing during sponsor setup - ignore silently
                return Err(anyhow::anyhow!(
                    "User is not the admin who started the action"
                ));
            }
        }

        if let Some(text) = msg.text() {
            let text = text.trim();
            if !text.is_empty() {
                match sponsor_state.step {
                    crate::sponsor::dto::SponsorStep::AwaitingRequestLimit => {
                        // Parse the request limit number
                        match text.parse::<u64>() {
                            Ok(limit) => {
                                // Validate the limit
                                if limit == 0 {
                                    bot.send_message(
                                                msg.chat.id,
                                                "❌ Request limit cannot be 0. Please enter a number greater than 0."
                                            )
                                            .await?;
                                    return Ok(true);
                                }

                                // Update the sponsor settings
                                let mut settings = bot_deps
                                    .sponsor
                                    .get_sponsor_settings(current_group_id.clone());
                                settings.requests = limit;

                                if let Err(e) = bot_deps.sponsor.set_or_update_sponsor_settings(
                                    current_group_id.clone(),
                                    settings.clone(),
                                ) {
                                    bot.send_message(
                                        msg.chat.id,
                                        format!("❌ Failed to update request limit: {}", e),
                                    )
                                    .await?;
                                    return Ok(true);
                                }

                                // Reset requests to new limit when limit changes
                                let new_requests = crate::sponsor::dto::SponsorRequest {
                                    requests_left: limit,
                                    last_request: chrono::Utc::now().timestamp() as u64,
                                };

                                if let Err(e) = bot_deps.sponsor.set_or_update_sponsor_requests(
                                    current_group_id.clone(),
                                    new_requests,
                                ) {
                                    log::warn!(
                                        "Failed to reset requests after limit change: {}",
                                        e
                                    );
                                }

                                // Clear the sponsor state
                                if let Err(e) = bot_deps
                                    .sponsor
                                    .remove_sponsor_state(current_group_id.clone())
                                {
                                    log::warn!("Failed to remove sponsor state: {}", e);
                                }

                                // Send success message
                                bot.send_message(
                                    msg.chat.id,
                                    format!(
                                        "✅ <b>Request limit updated to {} per interval</b>",
                                        limit
                                    ),
                                )
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .await?;

                                let settings = bot_deps
                                    .sponsor
                                    .get_sponsor_settings(current_group_id.to_string());
                                let (requests_left, total_requests) = bot_deps
                                    .sponsor
                                    .get_request_status(current_group_id.to_string())
                                    .unwrap_or((0, 0));

                                let interval_text = match settings.interval {
                                    SponsorInterval::Hourly => "Hourly",
                                    SponsorInterval::Daily => "Daily",
                                    SponsorInterval::Weekly => "Weekly",
                                    SponsorInterval::Monthly => "Monthly",
                                };

                                let text = format!(
                                    "🎯 <b>Sponsor Settings</b>\n\n\
                                            <b>Current Status:</b>\n\
                                            • Total Requests: <b>{}</b>\n\
                                            • Requests Left: <b>{}</b>\n\
                                            • Interval: <b>{}</b>\n\n\
                                            <b>How it works:</b>\n\
                                            • Users can use <code>/g</code> command\n\
                                            • No registration required\n\
                                            • Requests reset every interval\n\
                                            • Only admins can change settings\n\n\
                                            Choose an action below:",
                                    total_requests, requests_left, interval_text
                                );

                                let kb = InlineKeyboardMarkup::new(vec![
                                    vec![InlineKeyboardButton::callback(
                                        "📊 Set Request Limit",
                                        "sponsor_set_requests",
                                    )],
                                    vec![InlineKeyboardButton::callback(
                                        "⏰ Set Interval",
                                        "sponsor_set_interval",
                                    )],
                                    vec![InlineKeyboardButton::callback(
                                        "🚫 Disable Sponsor",
                                        "sponsor_disable",
                                    )],
                                    vec![InlineKeyboardButton::callback(
                                        "↩️ Back",
                                        "back_to_group_settings",
                                    )],
                                ]);

                                bot.send_message(group_id, text)
                                    .parse_mode(teloxide::types::ParseMode::Html)
                                    .reply_markup(kb)
                                    .await?;

                                return Ok(true);
                            }
                            Err(_) => {
                                bot.send_message(
                                            msg.chat.id,
                                            "❌ Invalid input. Please enter a valid number (e.g., 5, 10, 25, 100)."
                                        )
                                        .await?;
                                return Ok(true);
                            }
                        }
                    }
                    _ => {
                        // Unknown step, clear sponsor state
                        if let Err(e) = bot_deps
                            .sponsor
                            .remove_sponsor_state(current_group_id.clone())
                        {
                            log::warn!("Failed to remove sponsor state: {}", e);
                        }
                        bot.send_message(msg.chat.id, "❌ Unknown input step. Please try again.")
                            .await?;
                        return Ok(true);
                    }
                }
            } else {
                // Empty text, ask for valid input
                bot.send_message(
                    msg.chat.id,
                    "❌ Please enter a valid number for the request limit.",
                )
                .await?;
                return Ok(true);
            }
        } else {
            // No text, ask for valid input
            bot.send_message(
                msg.chat.id,
                "❌ Please send a text message with the number for the request limit.",
            )
            .await?;
            return Ok(true);
        }
    } else {
        return Ok(false);
    }
}
