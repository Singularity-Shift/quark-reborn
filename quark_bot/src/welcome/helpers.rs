use crate::welcome::dto::WelcomeSettings;
use teloxide::utils::html;

pub fn get_default_welcome_message(username: &str, group_name: &str, timeout_minutes: u64) -> String {
    format!(
        "👋 Welcome to {}, @{}!\n\n🔒 Please verify you're human by clicking the button below within {} minutes.\n\n⚠️ You'll be automatically removed if you don't verify in time.",
        html::escape(group_name),
        html::escape(username),
        timeout_minutes
    )
}

pub fn get_custom_welcome_message(
    settings: &WelcomeSettings,
    username: &str,
    group_name: &str,
) -> String {
    if let Some(ref custom_msg) = settings.custom_message {
        let mut message = custom_msg.clone();
        
        // First, unescape markdown characters that Telegram escaped
        message = unescape_markdown(&message);
        
        // Replace placeholders (unescaped versions only, since unescape_markdown handles the rest)
        message = message.replace("{username}", &format!("@{}", username));
        message = message.replace("{group_name}", group_name);
        
        // Replace timeout
        let timeout_minutes = (settings.verification_timeout / 60).to_string();
        message = message.replace("{timeout}", &timeout_minutes);
        
        message
    } else {
        get_default_welcome_message(username, group_name, settings.verification_timeout / 60)
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

/// Unescape essential markdown characters for welcome messages
fn unescape_markdown(text: &str) -> String {
    let mut result = text.to_string();
    
    // Unescape essential markdown characters for welcome messages
    result = result.replace("\\*", "*");      // Bold/italic (very common)
    result = result.replace("\\_", "_");      // Underline (less common)
    result = result.replace("\\`", "`");      // Inline code (common for addresses, commands)
    result = result.replace("\\{", "{");      // Placeholders (essential)
    result = result.replace("\\}", "}");      // Placeholders (essential)
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::welcome::dto::WelcomeSettings;

    #[test]
    fn test_get_custom_welcome_message_unescaped() {
        let settings = WelcomeSettings {
            custom_message: Some("Hello {username}! Welcome to {group_name}! You have {timeout} minutes.".to_string()),
            verification_timeout: 300, // 5 minutes
            ..Default::default()
        };
        
        let result = get_custom_welcome_message(&settings, "john_doe", "Test Group");
        assert_eq!(result, "Hello @john_doe! Welcome to Test Group! You have 5 minutes.");
    }

    #[test]
    fn test_get_custom_welcome_message_escaped() {
        let settings = WelcomeSettings {
            custom_message: Some("Hello \\{username\\}! Welcome to \\{group_name\\}! You have \\{timeout\\} minutes.".to_string()),
            verification_timeout: 300, // 5 minutes
            ..Default::default()
        };
        
        let result = get_custom_welcome_message(&settings, "john_doe", "Test Group");
        assert_eq!(result, "Hello @john_doe! Welcome to Test Group! You have 5 minutes.");
    }

    #[test]
    fn test_get_custom_welcome_message_mixed() {
        let settings = WelcomeSettings {
            custom_message: Some("Hello \\{username\\}! Welcome to {group_name}! You have \\{timeout\\} minutes.".to_string()),
            verification_timeout: 300, // 5 minutes
            ..Default::default()
        };
        
        let result = get_custom_welcome_message(&settings, "john_doe", "Test Group");
        assert_eq!(result, "Hello @john_doe! Welcome to Test Group! You have 5 minutes.");
    }

    #[test]
    fn test_get_custom_welcome_message_fallback() {
        let settings = WelcomeSettings {
            custom_message: None,
            verification_timeout: 300, // 5 minutes
            ..Default::default()
        };
        
        let result = get_custom_welcome_message(&settings, "john_doe", "Test Group");
        // Should use default message
        assert!(result.contains("@john_doe"));
        assert!(result.contains("Test Group"));
        assert!(result.contains("5"));
    }

    #[test]
    fn test_get_custom_welcome_message_escaped_underscore() {
        let settings = WelcomeSettings {
            custom_message: Some("Hello {username}! Welcome to \\{group\\_name\\}! You have {timeout} minutes.".to_string()),
            verification_timeout: 300, // 5 minutes
            ..Default::default()
        };
        
        let result = get_custom_welcome_message(&settings, "john_doe", "Test Group");
        assert_eq!(result, "Hello @john_doe! Welcome to Test Group! You have 5 minutes.");
    }

    #[test]
    fn test_get_custom_welcome_message_markdown_unescaping() {
        // Test with escaped markdown characters around placeholders
        let settings = WelcomeSettings {
            custom_message: Some("Hello \\*{username}\\*! Welcome to \\*\\*{group_name}\\*\\*! You have {timeout} minutes.".to_string()),
            verification_timeout: 300, // 5 minutes
            ..Default::default()
        };
        
        let result = get_custom_welcome_message(&settings, "john_doe", "Test Group");
        assert_eq!(result, "Hello *@john_doe*! Welcome to **Test Group**! You have 5 minutes.");
    }
}
