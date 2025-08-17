use anyhow::Result;
use chrono::{Utc, Timelike};
use teloxide::{prelude::*, types::InlineKeyboardMarkup};

use crate::dependencies::BotDependencies;
use crate::scheduled_payments::dto::{PendingPaymentStep};
use crate::scheduled_payments::wizard::{build_repeat_keyboard_payments, summarize};
use crate::scheduled_prompts::dto::RepeatPolicy;

pub async fn handle_scheduled_payments_callback(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    let data = query.data.as_deref().unwrap_or("");
    let user = &query.from;
    let message = match &query.message {
        Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) => m,
        _ => {
            bot.answer_callback_query(query.id).text("❌ Invalid context").await?;
            return Ok(());
        }
    };

    // Admin-only actions
    let admins = bot.get_chat_administrators(message.chat.id).await?;
    if !admins.iter().any(|m| m.user.id == user.id) {
        bot.answer_callback_query(query.id).text("❌ Admins only").await?;
        return Ok(());
    }

    let key = (&message.chat.id.0, &(user.id.0 as i64));

    if data.starts_with("schedpay_hour:") {
        let hour: u8 = data.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
            st.step = PendingPaymentStep::AwaitingMinute;
            st.hour_utc = Some(hour);
            bot_deps.scheduled_payments.put_pending(key, &st)?;
            bot.answer_callback_query(query.id).await?;
            bot.edit_message_text(message.chat.id, message.id, "Select minute (UTC)")
                .reply_markup(crate::scheduled_payments::wizard::build_minutes_keyboard_payments())
                .await?;
        }
    } else if data.starts_with("schedpay_min:") || data.starts_with("sched_min:") {
        let minute: u8 = data.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
            st.step = PendingPaymentStep::AwaitingRepeat;
            st.minute_utc = Some(minute);
            bot_deps.scheduled_payments.put_pending(key, &st)?;
            bot.answer_callback_query(query.id).await?;
            bot.edit_message_text(message.chat.id, message.id, "Select repeat interval")
                .reply_markup(build_repeat_keyboard_payments())
                .await?;
        }
    } else if data.starts_with("schedpay_repeat:") {
        let (repeat, weeks) = match data.split(':').nth(1).unwrap_or("") {
            "1d" => (RepeatPolicy::Daily, None),
            "1w" => (RepeatPolicy::Weekly, Some(1)),
            "2w" => (RepeatPolicy::Weekly, Some(2)),
            "4w" => (RepeatPolicy::Weekly, Some(4)),
            _ => (RepeatPolicy::Weekly, Some(1)),
        };
        if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
            st.step = PendingPaymentStep::AwaitingConfirm;
            st.repeat = Some(repeat);
            st.weekly_weeks = weeks;
            bot_deps.scheduled_payments.put_pending(key, &st)?;
            let summary = summarize(&st);
            let kb = InlineKeyboardMarkup::new(vec![vec![
                teloxide::types::InlineKeyboardButton::callback(
                    "✔️ Create schedule".to_string(),
                    "schedpay_confirm".to_string(),
                ),
            ], vec![
                teloxide::types::InlineKeyboardButton::callback(
                    "↩️ Cancel", "schedpay_cancel".to_string()
                ),
            ]]);
            bot.answer_callback_query(query.id).await?;
            bot.edit_message_text(message.chat.id, message.id, summary)
                .reply_markup(kb)
                .await?;
        }
    } else if data == "schedpay_confirm" {
        if let Some(st) = bot_deps.scheduled_payments.get_pending(key) {
            bot_deps.scheduled_payments.delete_pending(key)?;
            super::handler::finalize_and_register_payment(bot.clone(), bot_deps.clone(), st).await?;
            bot.answer_callback_query(query.id).await?;
        }
    } else if data == "schedpay_cancel" {
        bot_deps.scheduled_payments.delete_pending(key)?;
        bot.answer_callback_query(query.id).text("✅ Cancelled").await?;
        if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
            let _ = bot.edit_message_reply_markup(m.chat.id, m.id).await;
        }
    } else if data.starts_with("schedpay_toggle:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(mut rec) = bot_deps.scheduled_payments.get_schedule(id) {
            rec.active = !rec.active;
            let _ = bot_deps.scheduled_payments.put_schedule(&rec);
            bot.answer_callback_query(query.id)
                .text(if rec.active { "▶️ Resumed" } else { "⏸ Paused" })
                .await?;
        }
    } else if data.starts_with("schedpay_delete:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(mut rec) = bot_deps.scheduled_payments.get_schedule(id) {
            rec.active = false;
            let _ = bot_deps.scheduled_payments.put_schedule(&rec);
            bot.answer_callback_query(query.id).text("🗑 Deleted").await?;
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                let _ = bot.delete_message(m.chat.id, m.id).await;
            }
        }
    } else if data.starts_with("schedpay_edit:") {
        // Creator-only edit: present submenu and open scoped wizard
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(rec) = bot_deps.scheduled_payments.get_schedule(id) {
            if rec.creator_user_id != query.from.id.0 as i64 {
                bot.answer_callback_query(query.id).text("❌ Only the creator can edit").await?;
                return Ok(());
            }
            let st = crate::scheduled_payments::dto::PendingPaymentWizardState {
                group_id: rec.group_id,
                creator_user_id: rec.creator_user_id,
                creator_username: rec.creator_username.clone(),
                step: crate::scheduled_payments::dto::PendingPaymentStep::AwaitingRecipient,
                schedule_id: Some(rec.id.clone()),
                recipient_username: rec.recipient_username.clone(),
                recipient_address: rec.recipient_address.clone(),
                symbol: rec.symbol.clone(),
                token_type: rec.token_type.clone(),
                decimals: rec.decimals,
                amount_display: rec
                    .amount_smallest_units
                    .and_then(|v| rec.decimals.map(|d| v as f64 / 10f64.powi(d as i32))),
                date: rec.start_timestamp_utc.map(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0).map(|dt| dt.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "".into())),
                hour_utc: rec
                    .start_timestamp_utc
                    .and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0).map(|dt| dt.hour() as u8)),
                minute_utc: rec
                    .start_timestamp_utc
                    .and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0).map(|dt| dt.minute() as u8)),
                repeat: Some(rec.repeat.clone()),
                weekly_weeks: rec.weekly_weeks,
            };
            bot_deps
                .scheduled_payments
                .put_pending((&st.group_id, &st.creator_user_id), &st)?;
            bot.answer_callback_query(query.id).await?;
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                use teloxide::types::{InlineKeyboardButton as Btn, InlineKeyboardMarkup as Kb};
                let kb = Kb::new(vec![
                    vec![
                        Btn::callback("👤 Recipient", format!("schedpay_editrecipient:{}", id)),
                        Btn::callback("💳 Token", format!("schedpay_edittoken:{}", id)),
                    ],
                    vec![
                        Btn::callback("💰 Amount", format!("schedpay_editamount:{}", id)),
                        Btn::callback("🗓 Date/Time", format!("schedpay_editdate:{}", id)),
                    ],
                    vec![Btn::callback("🔁 Repeat", format!("schedpay_editrepeat:{}", id))],
                ]);
                bot.edit_message_text(m.chat.id, m.id, "✏️ What would you like to edit?")
                    .reply_markup(kb)
                    .await?;
            }
        }
    } else if data.starts_with("schedpay_editrecipient:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(rec) = bot_deps.scheduled_payments.get_schedule(id) {
            let key = (&rec.group_id, &rec.creator_user_id);
            if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
                st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingRecipient;
                bot_deps.scheduled_payments.put_pending(key, &st)?;
            }
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                bot.edit_message_text(m.chat.id, m.id, "👤 Send the new @recipient username").await?;
            }
            bot.answer_callback_query(query.id).await?;
        }
    } else if data.starts_with("schedpay_edittoken:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(rec) = bot_deps.scheduled_payments.get_schedule(id) {
            let key = (&rec.group_id, &rec.creator_user_id);
            if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
                st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingToken;
                bot_deps.scheduled_payments.put_pending(key, &st)?;
            }
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                bot.edit_message_text(m.chat.id, m.id, "💳 Send token symbol (e.g., APT, USDC)").await?;
            }
            bot.answer_callback_query(query.id).await?;
        }
    } else if data.starts_with("schedpay_editamount:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(rec) = bot_deps.scheduled_payments.get_schedule(id) {
            let key = (&rec.group_id, &rec.creator_user_id);
            if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
                st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingAmount;
                bot_deps.scheduled_payments.put_pending(key, &st)?;
            }
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                bot.edit_message_text(m.chat.id, m.id, "💰 Send new amount").await?;
            }
            bot.answer_callback_query(query.id).await?;
        }
    } else if data.starts_with("schedpay_editdate:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(rec) = bot_deps.scheduled_payments.get_schedule(id) {
            let key = (&rec.group_id, &rec.creator_user_id);
            if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
                st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingDate;
                bot_deps.scheduled_payments.put_pending(key, &st)?;
            }
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                bot.edit_message_text(m.chat.id, m.id, "📅 Send new date YYYY-MM-DD (UTC)").await?;
            }
            bot.answer_callback_query(query.id).await?;
        }
    } else if data.starts_with("schedpay_editrepeat:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(rec) = bot_deps.scheduled_payments.get_schedule(id) {
            let key = (&rec.group_id, &rec.creator_user_id);
            if let Some(mut st) = bot_deps.scheduled_payments.get_pending(key) {
                st.step = crate::scheduled_payments::dto::PendingPaymentStep::AwaitingRepeat;
                bot_deps.scheduled_payments.put_pending(key, &st)?;
            }
            if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
                bot.edit_message_text(m.chat.id, m.id, "🔁 Select new repeat interval")
                    .reply_markup(build_repeat_keyboard_payments())
                    .await?;
            }
            bot.answer_callback_query(query.id).await?;
        }
    } else if data.starts_with("schedpay_runnow:") {
        let id = data.split(':').nth(1).unwrap_or("");
        if let Some(mut rec) = bot_deps.scheduled_payments.get_schedule(id) {
            // Set due now and let runner pick it up on next tick
            rec.next_run_at = Some(Utc::now().timestamp());
            let _ = bot_deps.scheduled_payments.put_schedule(&rec);
            bot.answer_callback_query(query.id).text("⚡ Queued to run").await?;
        }
    } else if data.starts_with("schedpay_close:") {
        bot.answer_callback_query(query.id).text("Closed").await?;
        if let Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) = &query.message {
            let _ = bot.edit_message_reply_markup(m.chat.id, m.id).await;
        }
    }

    Ok(())
}


