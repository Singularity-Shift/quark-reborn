use anyhow::Result;
use open_ai_rust_responses_by_sshift::{Client, Request, Model};
use teloxide::{Bot, types::Message, prelude::*};

pub struct ModerationService {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct ModerationResult {
    pub verdict: String,           // "P" or "F"
    pub prompt_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl ModerationService {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::new(&api_key)?;
        Ok(Self { client })
    }

    pub async fn moderate_message(&self, message_text: &str, bot: &Bot, original_msg: &Message, replied_msg: &Message) -> Result<ModerationResult> {
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
                        prompt_tokens: 0,
                        output_tokens: 0,
                        total_tokens: 0,
                    });
                }
            }
        }

        // Proceed with AI moderation for non-admin users
        let prompt = format!(
            "You are an expert content moderator. Analyze this message for violations:\n\nRULE 1 - PROMOTION/SELLING:\n- Offering services, products, access, or benefits\n- Positioning as authority/leader to gain trust\n- Promising exclusive opportunities or deals\n- Any form of commercial solicitation\n\nRULE 2 - PRIVATE COMMUNICATION:\n- Requesting to move conversation to DM/private\n- Offering to send details privately\n- Asking for personal contact information\n- Any attempt to bypass public group discussion\n\nEXAMPLES TO FLAG (NOT EXHAUSTIVE - look for similar patterns):\n- \"I can offer you whitelist access\"\n- \"DM me for details\"\n- \"React and I'll message you\"\n- \"I'm a [title] and can help you\"\n- \"Send me your wallet address\"\n- \"Contact me privately\"\n- \"I'll send you the link\"\n\nIMPORTANT: These examples are just patterns. Flag ANY message that violates the rules above, even if worded differently.\n\nReturn only:\n- 'F' if ANY rule is violated\n- 'P' if completely clean\n\nMessage: \"{}\"",
            message_text
        );

        let request = Request::builder()
            .model(Model::GPT41Nano)
            .input(prompt)
            .max_output_tokens(20)
            .temperature(0.1)
            .build();

        let response = self.client.responses.create(request).await?;
        let result = response.output_text().trim().to_uppercase();
        
        // Extract token usage
        let (prompt_tokens, output_tokens, total_tokens) = if let Some(usage) = &response.usage {
            (
                usage.input_tokens,
                usage.output_tokens,
                usage.total_tokens
            )
        } else {
            (0, 0, 0)
        };
        
        // Ensure we only return P or F
        let verdict = if result.contains('F') {
            "F".to_string()
        } else {
            "P".to_string()
        };

        Ok(ModerationResult {
            verdict,
            prompt_tokens,
            output_tokens,
            total_tokens,
        })
    }
} 