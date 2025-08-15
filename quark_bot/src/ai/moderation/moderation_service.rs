use anyhow::Result;
use open_ai_rust_responses_by_sshift::{Client, Model, Request, ReasoningEffort, Verbosity};
use teloxide::{Bot, prelude::*, types::Message};

use crate::ai::moderation::dto::{ModerationOverrides, ModerationResult};
use crate::ai::moderation::overrides::build_override_section;

#[derive(Clone)]
pub struct ModerationService {
    pub(crate) client: Client,
}

impl ModerationService {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::new(&api_key)?;
        Ok(Self { client })
    }

    pub async fn moderate_message(
        &self,
        message_text: &str,
        bot: &Bot,
        original_msg: &Message,
        replied_msg: &Message,
        overrides: Option<ModerationOverrides>,
    ) -> Result<ModerationResult> {
        // Check if the user who sent the replied message has admin role
        if let Some(user) = &replied_msg.from {
            let user_id = user.id;

            // Get chat administrators
            if let Ok(admins) = bot.get_chat_administrators(original_msg.chat.id).await {
                let is_admin = admins.iter().any(|member| member.user.id == user_id);

                if is_admin {
                    // Admin users automatically pass moderation (no API call, no tokens used)
                    return Ok(ModerationResult {
                        verdict: "P".to_string(),
                        total_tokens: 0,
                    });
                }
            }
        }

        // Build group override section if provided
        let override_section = build_override_section(overrides);

        // Proceed with AI moderation for non-admin users
        let prompt = format!(
            r#"Return EXACTLY one character:

'F' if ANY rule is violated

'P' if completely clean
(No explanations, no punctuation, no extra text.)

STRICT PRIORITY ORDER (apply in this order):
- Group Disallowed > Group Allowed > Default Rules

You are a high-precision content moderator for a crypto community.

{override_section}

Default Moderation Rules (flag 'F' on any match):

Promotion / Selling:
- Offering services, products, access, or benefits
- Positioning as authority/leader to gain trust
- Promising exclusive opportunities or deals
- Any commercial solicitation

Private Communication:
- Asking to move to DM/private, or sharing personal contact channels
- "DM me", "I'll message you", "Contact me privately", etc.

Unsolicited Links with Call-To-Action (phishing/scam risk):
- Any external link combined with CTAs like:
  vote, sign, proposal, claim, mint, verify, connect wallet, airdrop, whitelist or semantically similar words
- Urgency/pressure language (e.g., urgent, now, protect your holdings) with or without a link

Examples to flag (not exhaustive consider semantically similar words):
- "DM me for details"
- "I can offer whitelist access"
- "Vote here: http://…"
- "Sign the proposal here: https://…"
- "Connect your wallet to claim: https://…"

Notes:
- Do not flag neutral short expressions ("Let's go", "gm", etc.) unless they clearly violate rules.
- Use minimal but sufficient reasoning: if clear risk signals (link + CTA/urgency) are present, choose 'F'. If uncertain, rationalise based on the rules before deciding; otherwise choose 'P'.

  Message: "{}""#,
            message_text, override_section = override_section
        );

        let request = Request::builder()
            .model(Model::GPT5Nano)
            .input(prompt)
            .verbosity(Verbosity::Low)
            .reasoning_effort(ReasoningEffort::Minimal)
            .max_output_tokens(500)
            .build();

        let response = self.client.responses.create(request).await?;
        let result = response.output_text().trim().to_uppercase();

        // Extract token usage
        let total_tokens = if let Some(usage) = &response.usage {
            usage.total_tokens
        } else {
            0
        };

        // Ensure we only return P or F
        let verdict = if result.contains('F') {
            "F".to_string()
        } else {
            "P".to_string()
        };

        Ok(ModerationResult {
            verdict,
            total_tokens,
        })
    }
}


