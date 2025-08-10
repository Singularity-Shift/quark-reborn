use anyhow::Result;
use teloxide::{prelude::*, types::{InlineKeyboardMarkup, Message, ParseMode}};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    dependencies::BotDependencies,
    scheduled_prompts::dto::{PendingStep, PendingWizardState, RepeatPolicy, ScheduledPromptRecord},
    scheduled_prompts::runner::{register_schedule, register_all_schedules},
    scheduled_prompts::storage::ScheduledStorage,
    scheduled_prompts::wizard::summarize,
};

pub async fn bootstrap_scheduled_prompts(bot: Bot, bot_deps: BotDependencies) -> Result<()> {
    register_all_schedules(bot, bot_deps).await
}

pub async fn handle_scheduleprompt_command(bot: Bot, msg: Message, bot_deps: BotDependencies) -> Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(msg.chat.id, "❌ This command is only available in groups.").await?;
        return Ok(());
    }

    // Admin check
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let user = match msg.from { Some(u) => u, None => { return Ok(()); } };
    if !admins.iter().any(|m| m.user.id == user.id) {
        bot.send_message(msg.chat.id, "❌ Only administrators can use this command.").await?;
        return Ok(());
    }

    let username = match user.username.clone() { Some(u) => u, None => {
        bot.send_message(msg.chat.id, "❌ Username required to schedule prompts.").await?;
        return Ok(());
    }};

    let storage = ScheduledStorage::new(&bot_deps.db)?;
    let state = PendingWizardState {
        group_id: msg.chat.id.0 as i64,
        creator_user_id: user.id.0 as i64,
        creator_username: username,
        step: PendingStep::AwaitingPrompt,
        prompt: None,
        hour_utc: None,
        minute_utc: None,
        repeat: None,
    };
    storage.put_pending((&state.group_id, &state.creator_user_id), &state)?;

    bot.send_message(msg.chat.id, "📝 Please reply to this message with the prompt you want to schedule.")
        .await?;
    Ok(())
}

pub async fn handle_listscheduled_command(bot: Bot, msg: Message, bot_deps: BotDependencies) -> Result<()> {
    // Admin check
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let user = match msg.from { Some(u) => u, None => { return Ok(()); } };
    if !admins.iter().any(|m| m.user.id == user.id) {
        bot.send_message(msg.chat.id, "❌ Only administrators can use this command.").await?;
        return Ok(());
    }

    let storage = ScheduledStorage::new(&bot_deps.db)?;
    let list = storage.list_schedules_for_group(msg.chat.id.0 as i64);

    if list.is_empty() {
        bot.send_message(msg.chat.id, "📭 No active scheduled prompts in this group.").await?;
        return Ok(());
    }

    for rec in list {
        let repeat_label = match rec.repeat {
            RepeatPolicy::None => "No repeat".to_string(),
            RepeatPolicy::Every5m => "Every 5 min".to_string(),
            RepeatPolicy::Every15m => "Every 15 min".to_string(),
            RepeatPolicy::Every30m => "Every 30 min".to_string(),
            RepeatPolicy::Every45m => "Every 45 min".to_string(),
            RepeatPolicy::Every1h => "Every 1 hour".to_string(),
            RepeatPolicy::Every3h => "Every 3 hours".to_string(),
            RepeatPolicy::Every6h => "Every 6 hours".to_string(),
            RepeatPolicy::Every12h => "Every 12 hours".to_string(),
            RepeatPolicy::Daily => "Daily".to_string(),
            RepeatPolicy::Weekly => "Weekly".to_string(),
            RepeatPolicy::Monthly => "Monthly".to_string(),
        };
        let title = format!(
            "⏰ {:02}:{:02} UTC — {}\n\n{}",
            rec.start_hour_utc,
            rec.start_minute_utc,
            repeat_label,
            if rec.prompt.len() > 180 { format!("{}…", &rec.prompt[..180]) } else { rec.prompt.clone() }
        );
        let kb = InlineKeyboardMarkup::new(vec![vec![
            teloxide::types::InlineKeyboardButton::callback("❌ Cancel".to_string(), format!("sched_cancel:{}", rec.id)),
        ]]);
        bot.send_message(msg.chat.id, title)
            .reply_markup(kb)
            .await?;
    }

    Ok(())
}

pub async fn finalize_and_register(
    bot: Bot,
    bot_deps: BotDependencies,
    state: PendingWizardState,
) -> Result<()> {
    // Enforce per-group cap: max 10 active schedules
    let storage = ScheduledStorage::new(&bot_deps.db)?;
    let active_count = storage.list_schedules_for_group(state.group_id).len();
    if active_count >= 10 {
        bot.send_message(
            ChatId(state.group_id as i64),
            "❌ You already have 10 active scheduled prompts in this group.\n\nPlease cancel one with /listscheduled before adding a new schedule.",
        )
        .await?;
        return Ok(());
    }

    let id = Uuid::new_v4().to_string();
    let mut rec = ScheduledPromptRecord {
        id: id.clone(),
        group_id: state.group_id,
        creator_user_id: state.creator_user_id,
        creator_username: state.creator_username.clone(),
        prompt: state.prompt.clone().unwrap_or_default(),
        start_hour_utc: state.hour_utc.unwrap_or(0),
        start_minute_utc: state.minute_utc.unwrap_or(0),
        repeat: state.repeat.clone().unwrap_or(RepeatPolicy::None),
        active: true,
        created_at: Utc::now().timestamp(),
        last_run_at: None,
        next_run_at: None,
        run_count: 0,
        locked_until: None,
        scheduler_job_id: None,
        conversation_response_id: None,
    };

    storage.put_schedule(&rec)?;
    register_schedule(bot.clone(), bot_deps.clone(), &mut rec).await?;
    storage.put_schedule(&rec)?;

    bot.send_message(
        ChatId(rec.group_id as i64),
        format!(
            "✅ Scheduled created!\n\n{}",
            summarize(&PendingWizardState {
                group_id: rec.group_id,
                creator_user_id: rec.creator_user_id,
                creator_username: rec.creator_username,
                step: PendingStep::AwaitingConfirm,
                prompt: Some(rec.prompt),
                hour_utc: Some(rec.start_hour_utc),
                minute_utc: Some(rec.start_minute_utc),
                repeat: Some(rec.repeat),
            })
        ),
    )
    .parse_mode(ParseMode::Html)
    .await?;

    Ok(())
}


