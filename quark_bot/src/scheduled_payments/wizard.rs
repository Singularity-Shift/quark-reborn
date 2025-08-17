use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::scheduled_payments::dto::PendingPaymentWizardState;
use crate::scheduled_prompts::dto::RepeatPolicy;

pub fn build_repeat_keyboard_payments() -> InlineKeyboardMarkup {
    let rows = vec![
        vec![InlineKeyboardButton::callback(
            "Daily".to_string(),
            "schedpay_repeat:1d".to_string(),
        )],
        vec![InlineKeyboardButton::callback(
            "Weekly".to_string(),
            "schedpay_repeat:1w".to_string(),
        )],
        vec![
            InlineKeyboardButton::callback(
                "2-Weekly".to_string(),
                "schedpay_repeat:2w".to_string(),
            ),
            InlineKeyboardButton::callback(
                "4-Weekly".to_string(),
                "schedpay_repeat:4w".to_string(),
            ),
        ],
    ];
    InlineKeyboardMarkup::new(rows)
}

pub fn build_hours_keyboard_payments() -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    for h in 0..24u8 {
        row.push(InlineKeyboardButton::callback(
            format!("{:02}", h),
            format!("schedpay_hour:{}", h),
        ));
        if row.len() == 6 {
            rows.push(row);
            row = Vec::new();
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    InlineKeyboardMarkup::new(rows)
}

pub fn build_minutes_keyboard_payments() -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    let mut row: Vec<InlineKeyboardButton> = Vec::new();
    for m in (0..=55).step_by(5) {
        let mu = m as u8;
        row.push(InlineKeyboardButton::callback(
            format!("{:02}", mu),
            format!("schedpay_min:{}", mu),
        ));
        if row.len() == 6 {
            rows.push(row);
            row = Vec::new();
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    InlineKeyboardMarkup::new(rows)
}

pub fn summarize(state: &PendingPaymentWizardState) -> String {
    let recipient = state
        .recipient_username
        .as_deref()
        .map(|u| format!("@{}", u))
        .unwrap_or("(recipient not set)".to_string());
    let symbol = state
        .symbol
        .as_deref()
        .unwrap_or("(symbol not set)");
    let amount = state
        .amount_display
        .map(|v| format!("{:.4}", v))
        .unwrap_or("(amount not set)".to_string());
    let date = state.date.clone().unwrap_or("(date not set)".to_string());
    let hour = state
        .hour_utc
        .map(|h| format!("{:02}", h))
        .unwrap_or("--".into());
    let minute = state
        .minute_utc
        .map(|m| format!("{:02}", m))
        .unwrap_or("--".into());
    let repeat = match state.repeat {
        Some(RepeatPolicy::Daily) => "Daily".to_string(),
        Some(RepeatPolicy::Weekly) => "Weekly / 1w".to_string(),
        Some(_) => "(unsupported)".to_string(),
        None => "(not set)".to_string(),
    };
    format!(
        "💸 Payment schedule (UTC)\nRecipient: {}\nAmount: {} {}\nFirst run: {} {}:{}\nRepeat: {}",
        recipient, symbol, amount, date, hour, minute, repeat
    )
}


