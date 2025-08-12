use anyhow::Result;
use teloxide::{prelude::*, types::{InlineKeyboardMarkup}};

use crate::{
    dependencies::BotDependencies,
    scheduled_prompts::dto::{PendingStep, RepeatPolicy},
    scheduled_prompts::handler::finalize_and_register,
    scheduled_prompts::storage::ScheduledStorage,
    scheduled_prompts::wizard::{build_minutes_keyboard, build_repeat_keyboard, summarize},
};

pub async fn handle_scheduled_prompts_callback(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    bot_deps: BotDependencies,
) -> Result<()> {
    let data = query.data.as_ref().unwrap();
    let user = &query.from;
    let message = match &query.message { Some(teloxide::types::MaybeInaccessibleMessage::Regular(m)) => m, _ => {
        bot.answer_callback_query(query.id).text("❌ Invalid context").await?; return Ok(());
    } };

    // Admin-only actions
    let admins = bot.get_chat_administrators(message.chat.id).await?;
    if !admins.iter().any(|m| m.user.id == user.id) {
        bot.answer_callback_query(query.id).text("❌ Admins only").await?;
        return Ok(());
    }

    let storage = ScheduledStorage::new(&bot_deps.db)?;
    let key = (&message.chat.id.0, &(user.id.0 as i64));

    if data.starts_with("sched_hour:") {
        let hour: u8 = data.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        if let Some(mut st) = storage.get_pending(key) {
            st.step = PendingStep::AwaitingMinute;
            st.hour_utc = Some(hour);
            storage.put_pending(key, &st)?;
            bot.answer_callback_query(query.id).await?;
            bot.edit_message_text(message.chat.id, message.id, "Select start minute (UTC)")
                .reply_markup(build_minutes_keyboard())
                .await?;
        }
    } else if data.starts_with("sched_min:") {
        let minute: u8 = data.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        if let Some(mut st) = storage.get_pending(key) {
            st.step = PendingStep::AwaitingRepeat;
            st.minute_utc = Some(minute);
            storage.put_pending(key, &st)?;
            bot.answer_callback_query(query.id).await?;
            bot.edit_message_text(message.chat.id, message.id, "Select repeat interval")
                .reply_markup(build_repeat_keyboard())
                .await?;
        }
    } else if data.starts_with("sched_repeat:") {
        let repeat = match data.split(':').nth(1).unwrap_or("") {
            "none" => RepeatPolicy::None,
            "5m" => RepeatPolicy::Every5m,
            "15m" => RepeatPolicy::Every15m,
            "30m" => RepeatPolicy::Every30m,
            "45m" => RepeatPolicy::Every45m,
            "1h" => RepeatPolicy::Every1h,
            "3h" => RepeatPolicy::Every3h,
            "6h" => RepeatPolicy::Every6h,
            "12h" => RepeatPolicy::Every12h,
            "1d" => RepeatPolicy::Daily,
            "1w" => RepeatPolicy::Weekly,
            "1mo" => RepeatPolicy::Monthly,
            _ => RepeatPolicy::Every1h,
        };
        if let Some(mut st) = storage.get_pending(key) {
            st.step = PendingStep::AwaitingConfirm;
            st.repeat = Some(repeat);
            storage.put_pending(key, &st)?;
            let summary = summarize(&st);
            let kb = InlineKeyboardMarkup::new(vec![vec![
                teloxide::types::InlineKeyboardButton::callback("✔️ Create schedule".to_string(), "sched_confirm".to_string()),
            ]]);
            bot.answer_callback_query(query.id).await?;
            bot.edit_message_text(message.chat.id, message.id, summary)
                .reply_markup(kb)
                .await?;
        }
    } else if data == "sched_confirm" {
        if let Some(st) = storage.get_pending(key) {
            storage.delete_pending(key)?;
            finalize_and_register(bot.clone(), bot_deps.clone(), st).await?;
            bot.answer_callback_query(query.id).await?;
        }
    } else if data.starts_with("sched_cancel:") {
        let id = data.split(':').nth(1).unwrap_or("");
        let rec = storage.get_schedule(id);
        if let Some(mut rec) = rec {
            if rec.group_id != message.chat.id.0 as i64 {
                bot.answer_callback_query(query.id).text("❌ Wrong group").await?;
                return Ok(());
            }
            rec.active = false;
            storage.put_schedule(&rec)?;
            bot.answer_callback_query(query.id).text("✅ Cancelled").await?;
            // Delete the message that contained the cancel button
            if let Err(e) = bot.delete_message(message.chat.id, message.id).await {
                log::warn!("Failed to delete schedule-cancel message {}: {}", message.id.0, e);
            }
        }
    } else {
        bot.answer_callback_query(query.id).text("❌ Unknown action").await?;
    }

    Ok(())
}


