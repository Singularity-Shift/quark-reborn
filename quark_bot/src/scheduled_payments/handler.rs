use anyhow::Result;
use teloxide::{prelude::*, types::Message};

use crate::dependencies::BotDependencies;
use crate::scheduled_payments::dto::{PendingPaymentStep, PendingPaymentWizardState, ScheduledPaymentRecord};
use chrono::Utc;
use uuid::Uuid;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn handle_schedulepayment_command(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
)
-> Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(msg.chat.id, "❌ This command is only available in groups.")
            .await?;
        return Ok(());
    }

    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let user = match msg.from.clone() {
        Some(u) => u,
        None => return Ok(())
    };
    if !admins.iter().any(|m| m.user.id == user.id) {
        bot.send_message(msg.chat.id, "❌ Only administrators can use this command.")
            .await?;
        return Ok(());
    }

    let username = match user.username.clone() {
        Some(u) => u,
        None => {
            bot.send_message(msg.chat.id, "❌ Username required to schedule payments.").await?;
            return Ok(());
        }
    };

    let state = PendingPaymentWizardState {
        group_id: msg.chat.id.0 as i64,
        creator_user_id: user.id.0 as i64,
        creator_username: username,
        step: PendingPaymentStep::AwaitingRecipient,
        schedule_id: None,
        recipient_username: None,
        recipient_address: None,
        symbol: None,
        token_type: None,
        decimals: None,
        amount_display: None,
        date: None,
        hour_utc: None,
        minute_utc: None,
        repeat: None,
        weekly_weeks: None,
    };

    bot_deps
        .scheduled_payments
        .put_pending((&state.group_id, &state.creator_user_id), &state)?;

    bot.send_message(
        msg.chat.id,
        "👤 Send the recipient @username to receive payment (must have a linked wallet).",
    )
    .await?;

    Ok(())
}

pub async fn handle_listscheduledpayments_command(
    bot: Bot,
    msg: Message,
    bot_deps: BotDependencies,
) -> Result<()> {
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let user = match msg.from.clone() { Some(u) => u, None => return Ok(()) };
    if !admins.iter().any(|m| m.user.id == user.id) {
        bot.send_message(msg.chat.id, "❌ Only administrators can use this command.").await?;
        return Ok(());
    }

    let list = bot_deps
        .scheduled_payments
        .list_schedules_for_group(msg.chat.id.0 as i64);

    if list.is_empty() {
        bot.send_message(msg.chat.id, "📭 No active scheduled payments in this group.").await?;
        return Ok(());
    }

    for rec in list {
        let smallest = rec.amount_smallest_units.unwrap_or(0);
        let decimals = rec.decimals.unwrap_or(8);
        let human = (smallest as f64) / 10f64.powi(decimals as i32);
        let title = format!(
            "⏰ {:>11} — @{} — {:.4} {}",
            rec.next_run_at
                .map(|v| chrono::DateTime::<chrono::Utc>::from_timestamp(v, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| v.to_string()))
                .unwrap_or_else(|| "n/a".to_string()),
            rec.recipient_username.clone().unwrap_or_default(),
            human,
            rec.symbol.clone().unwrap_or_default(),
        );
        let toggle_label = if rec.active { "⏸ Pause" } else { "▶️ Resume" };
        let kb = InlineKeyboardMarkup::new(vec![
            vec![
                InlineKeyboardButton::callback("✏️ Edit", format!("schedpay_edit:{}", rec.id)),
                InlineKeyboardButton::callback(toggle_label, format!("schedpay_toggle:{}", rec.id)),
            ],
            vec![
                InlineKeyboardButton::callback("⚡ Run now", format!("schedpay_runnow:{}", rec.id)),
                InlineKeyboardButton::callback("🗑 Delete", format!("schedpay_delete:{}", rec.id)),
            ],
            vec![
                InlineKeyboardButton::callback("↩️ Close", format!("schedpay_close:{}", rec.id)),
            ],
        ]);
        bot.send_message(msg.chat.id, title).reply_markup(kb).await?;
    }

    Ok(())
}

pub async fn finalize_and_register_payment(
    bot: Bot,
    bot_deps: BotDependencies,
    state: PendingPaymentWizardState,
) -> Result<()> {
    // Enforce per-group cap 50
    let active_count = bot_deps
        .scheduled_payments
        .list_schedules_for_group(state.group_id)
        .len();
    if active_count >= 50 {
        bot.send_message(
            teloxide::types::ChatId(state.group_id),
            "❌ You already have 50 active scheduled payments in this group. Delete or pause some before adding new ones.",
        )
        .await?;
        return Ok(());
    }

    // Compute first run timestamp (UTC) from date + hour/minute
    let date = state.date.clone().unwrap_or_default();
    let hour = state.hour_utc.unwrap_or(0);
    let minute = state.minute_utc.unwrap_or(0);
    let datetime_str = format!("{} {:02}:{:02}", date, hour, minute);
    let first_run = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M")
        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc).timestamp())
        .unwrap_or(Utc::now().timestamp());

    // Convert display amount to smallest units using decimals
    let amount_smallest_units = state.amount_display.and_then(|amt|
        state.decimals.map(|d| (amt * 10f64.powi(d as i32)) as u64)
    );

    // Upsert: if editing an existing schedule, reuse its id and preserve job id if present
    let id = state.schedule_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let mut rec = ScheduledPaymentRecord {
        id: id.clone(),
        group_id: state.group_id,
        creator_user_id: state.creator_user_id,
        creator_username: state.creator_username.clone(),
        recipient_username: state.recipient_username.clone(),
        recipient_address: state.recipient_address.clone(),
        symbol: state.symbol.clone(),
        token_type: state.token_type.clone(),
        decimals: state.decimals,
        amount_smallest_units,
        start_timestamp_utc: Some(first_run),
        repeat: state.repeat.clone().unwrap_or(crate::scheduled_prompts::dto::RepeatPolicy::Weekly),
        weekly_weeks: state.weekly_weeks,
        active: true,
        created_at: Utc::now().timestamp(),
        last_run_at: None,
        next_run_at: Some(first_run),
        run_count: 0,
        locked_until: None,
        scheduler_job_id: None,
        last_error: None,
        last_attempt_status: None,
        notify_on_success: true,
        notify_on_failure: true,
    };

    bot_deps.scheduled_payments.put_schedule(&rec)?;
    // Register in scheduler (re-register if edit)
    crate::scheduled_payments::runner::register_schedule(bot.clone(), bot_deps.clone(), &mut rec).await?;
    bot_deps.scheduled_payments.put_schedule(&rec)?;
    bot.send_message(
        teloxide::types::ChatId(rec.group_id),
        "✅ Scheduled payment created!",
    )
    .await?;

    Ok(())
}


