use crate::filters::dto::{PendingFilterWizardState, MatchType, ResponseType};

pub fn summarize(state: &PendingFilterWizardState) -> String {
    let trigger = state
        .trigger
        .as_deref()
        .unwrap_or("(trigger not set)");
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
        "🔍 <b>Filter Summary</b>\n\n📝 Trigger: <code>{}</code>\n💬 Response: <code>{}</code>\n🎯 Match type: {}\n📄 Format: {}",
        trigger, response, match_type, response_type
    )
}
