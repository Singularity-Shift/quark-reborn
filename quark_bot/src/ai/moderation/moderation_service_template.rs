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
            r#"[INSERT YOUR MODERATION PROMPTING HERE]"#,
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
