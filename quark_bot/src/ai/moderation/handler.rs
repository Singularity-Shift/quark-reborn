use anyhow::Result;
use teloxide::{
    Bot,
    prelude::*,
    types::{Message, ParseMode},
};

use crate::{
    ai::moderation::dto::ModerationSettings,
    dependencies::BotDependencies,
    utils,
};

pub async fn handle_message_moderation(
    bot: &Bot,
    msg: &Message,
    bot_deps: &BotDependencies,
    chat_id: String,
) -> Result<bool> {
    if let Some(user) = &msg.from {
        // Only process moderation wizard if user is actually in wizard state
        if let Ok(mut moderation_state) = bot_deps.moderation.get_moderation_state(chat_id.clone()) {
            // Only the admin who started the wizard may respond
            let responder_id = user.id.0 as i64;
            match moderation_state.started_by_user_id {
                Some(owner) => {
                    if owner != responder_id {
                        return Ok(false); // not the wizard owner; ignore
                    }
                }
                None => {
                    // Backward-compat: if unspecified, allow only admins and claim the wizard to first admin responder
                    let is_admin = utils::is_admin(bot, msg.chat.id, user.id).await;
                    if !is_admin {
                        return Ok(false);
                    }
                    moderation_state.started_by_user_id = Some(responder_id);
                    bot_deps
                        .moderation
                        .set_moderation_state(chat_id.clone(), moderation_state.clone())
                        .unwrap();
                }
            }
            let text = msg
                .text()
                .or_else(|| msg.caption())
                .unwrap_or("")
                .trim()
                .to_string();
            if !text.is_empty() {
                let parse_items = |s: &str| -> Vec<String> {
                    s.split(';')
                        .map(|x| x.trim())
                        .filter(|x| !x.is_empty())
                        .take(50)
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                };
                if moderation_state.step == "AwaitingAllowed" {
                    let is_skip = text.eq_ignore_ascii_case("na");
                    let items = if is_skip {
                        Vec::new()
                    } else {
                        parse_items(&text)
                    };
                    moderation_state.allowed_items = Some(items);
                    moderation_state.step = "AwaitingDisallowed".to_string();
                    // Remove previous prompt (Step 1) if present
                    if let Some(mid) = moderation_state.message_id {
                        let _ = bot
                            .delete_message(msg.chat.id, teloxide::types::MessageId(mid as i32))
                            .await;
                    }
                    let sent = bot
                        .send_message(
                            msg.chat.id,
                            "🛡️ <b>Moderation Settings — Step 2/2</b>\n\n<b>Now send DISALLOWED items</b> for this group.\n\n<b>Be specific</b>: include concrete phrases, patterns, and examples you want flagged.\n\n<b>Cancel anytime</b>: Tap <b>Back</b> or <b>Close</b> in the Moderation menu — this prompt will be removed.\n\n<b>Format</b>:\n- Send them in a <b>single message</b>\n- Separate each item with <code>;</code>\n- To skip this section, send <code>na</code>\n\n<b>Examples (community standards)</b>:\n<code>harassment, insults, or personal attacks; hate speech or slurs (racism, homophobia, etc.); doxxing or sharing private information; NSFW/explicit content; graphic violence/gore; off-topic spam or mass mentions; repeated flooding/emoji spam; political or religious debates (off-topic); promotion of unrelated/non-affiliated projects; misinformation/FUD targeting members</code>\n\n<i>Notes:</i> \n- Avoid duplicating default scam rules (phishing links, wallet approvals, DM requests, giveaways) — those are already enforced by Default Rules.\n- <b>Group Disallowed</b> > <b>Group Allowed</b> > <b>Default Rules</b> (strict priority).\n- If any Group Disallowed item matches, the message will be flagged.",
                        )
                        .parse_mode(ParseMode::Html)
                        .await?;
                    // Track Step 2 prompt for cleanup
                    moderation_state.message_id = Some(sent.id.0 as i64);
                    bot_deps
                        .moderation
                        .set_moderation_state(chat_id.clone(), moderation_state)
                        .unwrap();
                    return Ok(true);
                } else if moderation_state.step == "AwaitingDisallowed" {
                    let is_skip = text.eq_ignore_ascii_case("na");
                    let disallowed = if is_skip {
                        Vec::new()
                    } else {
                        parse_items(&text)
                    };
                    let allowed = moderation_state.allowed_items.unwrap_or_default();
                    // Save to moderation_settings tree
                    let settings = ModerationSettings::from((
                        allowed.clone(),
                        disallowed.clone(),
                        user.id.0 as i64,
                        chrono::Utc::now().timestamp_millis(),
                    ));
                    bot_deps
                        .moderation
                        .set_or_update_moderation_settings(chat_id.clone(), settings)
                        .unwrap();
                    // Clear wizard and remove last prompt if present
                    if let Some(mid) = moderation_state.message_id {
                        let _ = bot
                            .delete_message(msg.chat.id, teloxide::types::MessageId(mid as i32))
                            .await;
                    }
                    bot_deps
                        .moderation
                        .remove_moderation_state(chat_id.clone())
                        .unwrap();
                    let allowed_list = if allowed.is_empty() {
                        "<i>(none)</i>".to_string()
                    } else {
                        allowed
                            .iter()
                            .map(|x| format!("• {}", teloxide::utils::html::escape(x)))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    let disallowed_list = if disallowed.is_empty() {
                        "<i>(none)</i>".to_string()
                    } else {
                        disallowed
                            .iter()
                            .map(|x| format!("• {}", teloxide::utils::html::escape(x)))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    let mut summary = format!(
                        "✅ <b>Custom moderation rules saved.</b>\n\n<b>Allowed ({})</b>:\n{}\n\n<b>Disallowed ({})</b>:\n{}",
                        allowed.len(),
                        allowed_list,
                        disallowed.len(),
                        disallowed_list,
                    );
                    if allowed.is_empty() && disallowed.is_empty() {
                        summary.push_str("\n\n<i>No custom rules recorded. Default moderation rules remain fully in effect.</i>");
                    }
                    bot.send_message(msg.chat.id, summary)
                        .parse_mode(ParseMode::Html)
                        .await?;
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}
