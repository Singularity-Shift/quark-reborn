// Parse trigger input supporting comma-separated tokens and bracketed multi-word tokens.
// Examples:
//   "[the contract], ca, contract" -> ["the contract", "ca", "contract"]
//   "hello, world" -> ["hello", "world"]
//   "[multi word] , single" -> ["multi word", "single"]
pub fn parse_triggers(input: &str) -> Vec<String> {
    let mut triggers: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_brackets = false;

    for ch in input.chars() {
        match ch {
            '[' => {
                if in_brackets {
                    // Nested '[' treated as literal
                    buf.push(ch);
                } else {
                    in_brackets = true;
                }
            }
            ']' => {
                if in_brackets {
                    in_brackets = false;
                } else {
                    // Unmatched ']' treated as literal
                    buf.push(ch);
                }
            }
            ',' => {
                if in_brackets {
                    buf.push(ch);
                } else {
                    let token = buf.trim();
                    if !token.is_empty() {
                        triggers.push(strip_brackets(token).to_string());
                    }
                    buf.clear();
                }
            }
            _ => buf.push(ch),
        }
    }

    let token = buf.trim();
    if !token.is_empty() {
        triggers.push(strip_brackets(token).to_string());
    }

    // Normalize: trim and convert to lowercase for consistent storage and matching
    triggers
        .into_iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

fn strip_brackets(s: &str) -> &str {
    let s = s.trim();
    if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

use crate::filters::dto::{PendingFilterWizardState, MatchType};

pub fn summarize(state: &PendingFilterWizardState) -> String {
    let trigger_input = state
        .trigger
        .as_deref()
        .unwrap_or("(trigger not set)");
    let triggers_display = if trigger_input == "(trigger not set)" {
        trigger_input.to_string()
    } else {
        let parts = parse_triggers(trigger_input);
        if parts.is_empty() {
            "(no valid triggers)".to_string()
        } else {
            parts
                .into_iter()
                .map(|t| format!("<code>{}</code>", t))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let response = state
        .response
        .as_deref()
        .unwrap_or("(response not set)");
    let match_type = match state.match_type {
        MatchType::Exact => "Exact word match",
        MatchType::Contains => "Contains anywhere",
        MatchType::StartsWith => "Message starts with",
        MatchType::EndsWith => "Message ends with",
    };
    format!(
        "🔍 <b>Filter Summary</b>\n\n📝 Triggers: {}\n💬 Response: <code>{}</code>\n🎯 Match type: {}\n📄 Format: Markdown (supports both markdown and plain text)",
        triggers_display, response, match_type
    )
}

/// Replace filter response placeholders with actual values
/// 
/// Available placeholders:
/// - {username} -> @username (with @ prefix for Telegram mentions)
/// - {group_name} -> actual group name
/// - {trigger} -> actual trigger word/phrase
pub fn replace_filter_placeholders(
    response: &str, 
    username: Option<&str>, 
    group_name: &str, 
    trigger: &str
) -> String {
    let mut result = response.to_string();
    
    // First, unescape markdown characters that Telegram escaped
    result = unescape_markdown(&result);
    
    // Replace username with @ prefix for Telegram mentions
    let username_display = if let Some(username) = username {
        format!("@{}", username)
    } else {
        "User".to_string()
    };
    
    // Replace placeholders (unescaped versions only, since unescape_markdown handles the rest)
    result = result.replace("{username}", &username_display);
    result = result.replace("{group_name}", group_name);
    result = result.replace("{trigger}", trigger);
    
    result
}

/// Unescape essential markdown characters for filters
fn unescape_markdown(text: &str) -> String {
    let mut result = text.to_string();
    
    // Unescape essential markdown characters for filters
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

    #[test]
    fn test_replace_filter_placeholders() {
        // Test basic placeholder replacement
        let response = "Hello {username}! Welcome to {group_name}!";
        let result = replace_filter_placeholders(response, Some("john_doe"), "Test Group", "hello");
        
        assert_eq!(result, "Hello @john_doe! Welcome to Test Group!");
    }

    #[test]
    fn test_replace_filter_placeholders_with_trigger() {
        // Test trigger placeholder
        let response = "You said '{trigger}'!";
        let result = replace_filter_placeholders(response, Some("alice"), "My Group", "gm");
        
        assert_eq!(result, "You said 'gm'!");
    }

    #[test]
    fn test_replace_filter_placeholders_missing_username() {
        // Test fallback when username is missing
        let response = "Hello {username}!";
        let result = replace_filter_placeholders(response, None, "Group", "hi");
        
        assert_eq!(result, "Hello User!");
    }

    #[test]
    fn test_replace_filter_placeholders_all_placeholders() {
        // Test all placeholders together
        let response = "Hey {username}! You said '{trigger}' in {group_name}!";
        let result = replace_filter_placeholders(response, Some("bob"), "Awesome Group", "hello");
        
        assert_eq!(result, "Hey @bob! You said 'hello' in Awesome Group!");
    }

    #[test]
    fn test_replace_filter_placeholders_no_placeholders() {
        // Test response with no placeholders
        let response = "Hello world!";
        let result = replace_filter_placeholders(response, Some("user"), "Group", "hi");
        
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_replace_filter_placeholders_special_characters() {
        // Test with special characters in usernames and group names
        let response = "Hello {username} from {group_name}!";
        let result = replace_filter_placeholders(response, Some("user_123"), "Group & Co.", "test");
        
        assert_eq!(result, "Hello @user_123 from Group & Co.!");
    }

    #[test]
    fn test_replace_filter_placeholders_escaped() {
        // Test with escaped placeholders (as they appear when captured from Telegram)
        let response = "Hello \\{username\\}, you said '\\{trigger\\}' in \\{group_name\\}!";
        let result = replace_filter_placeholders(response, Some("bob"), "Test Group", "hello");
        
        assert_eq!(result, "Hello @bob, you said 'hello' in Test Group!");
    }

    #[test]
    fn test_replace_filter_placeholders_mixed() {
        // Test with mix of escaped and unescaped placeholders
        let response = "Hey \\{username\\}! Welcome to {group_name}!";
        let result = replace_filter_placeholders(response, Some("alice"), "Cool Group", "hi");
        
        assert_eq!(result, "Hey @alice! Welcome to Cool Group!");
    }

    #[test]
    fn test_replace_filter_placeholders_escaped_underscore() {
        // Test with escaped underscore in group name placeholder
        let response = "Hello {username}! Welcome to \\{group\\_name\\}!";
        let result = replace_filter_placeholders(response, Some("bob"), "Test Group", "hello");
        
        assert_eq!(result, "Hello @bob! Welcome to Test Group!");
    }

    #[test]
    fn test_replace_filter_placeholders_markdown_unescaping() {
        // Test with escaped markdown characters around placeholders
        let response = "You said \\*\\*{trigger}\\*\\*!";
        let result = replace_filter_placeholders(response, Some("alice"), "Test Group", "hello");
        
        assert_eq!(result, "You said **hello**!");
    }

    #[test]
    fn test_replace_filter_placeholders_mixed_markdown() {
        // Test with mixed escaped markdown and placeholders
        let response = "Hello \\*{username}\\*, you said \\`{trigger}\\` in \\*\\*{group_name}\\*\\*!";
        let result = replace_filter_placeholders(response, Some("bob"), "Test Group", "hi");
        
        assert_eq!(result, "Hello *@bob*, you said `hi` in **Test Group**!");
    }
}