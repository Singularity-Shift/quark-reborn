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

use crate::filters::dto::{PendingFilterWizardState, MatchType, ResponseType};

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
    let response_type = match state.response_type {
        ResponseType::Text => "Plain text",
        ResponseType::Markdown => "Markdown",
    };
    
    format!(
        "🔍 <b>Filter Summary</b>\n\n📝 Triggers: {}\n💬 Response: <code>{}</code>\n🎯 Match type: {}\n📄 Format: {}",
        triggers_display, response, match_type, response_type
    )
}


