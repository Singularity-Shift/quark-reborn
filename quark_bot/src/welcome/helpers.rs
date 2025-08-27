use crate::welcome::dto::WelcomeSettings;
use teloxide::types::UserId;
use teloxide::utils::html;

pub fn get_default_welcome_message(username: &str, user_id: UserId, group_name: &str, timeout_minutes: u64) -> String {
    format!(
        "👋 Welcome to {}, <a href=\"tg://user?id={}\">@{}</a>!\n\n🔒 Please verify you're human by clicking the button below within {} minutes.\n\n⚠️ You'll be automatically removed if you don't verify in time.",
        html::escape(group_name),
        user_id.0,
        html::escape(username),
        timeout_minutes
    )
}

pub fn get_custom_welcome_message(
    settings: &WelcomeSettings,
    username: &str,
    user_id: UserId,
    group_name: &str,
) -> String {
    if let Some(ref custom_msg) = settings.custom_message {
        let mut message = custom_msg.clone();
        // Replace username with proper Telegram mention using HTML
        let mention_html = format!("<a href=\"tg://user?id={}\">@{}</a>", user_id.0, html::escape(username));
        message = message.replace("{username}", &mention_html);
        message = message.replace("{group_name}", group_name);
        message = message.replace("{timeout}", &(settings.verification_timeout / 60).to_string());
        message
    } else {
        get_default_welcome_message(username, user_id, group_name, settings.verification_timeout / 60)
    }
}

pub fn format_timeout_display(seconds: u64) -> String {
    if seconds < 60 {
        format!("{} seconds", seconds)
    } else if seconds < 3600 {
        format!("{} minutes", seconds / 60)
    } else {
        format!("{} hours", seconds / 3600)
    }
}

pub fn is_verification_expired(timestamp: i64) -> bool {
    chrono::Utc::now().timestamp() > timestamp
}

pub fn get_verification_expiry_time(timeout_seconds: u64) -> i64 {
    chrono::Utc::now().timestamp() + timeout_seconds as i64
}
