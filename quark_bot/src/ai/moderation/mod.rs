use anyhow::Result;
use open_ai_rust_responses_by_sshift::{Client, Request, Model};
use teloxide::{Bot, types::Message, prelude::*};

pub struct ModerationService {
    client: Client,
}

impl ModerationService {
    pub fn new(api_key: String) -> Result<Self> {
        let client = Client::new(&api_key)?;
        Ok(Self { client })
    }

    pub async fn moderate_message(&self, message_text: &str, bot: &Bot, original_msg: &Message, replied_msg: &Message) -> Result<String> {
        // Check if the user who sent the replied message has admin role
        if let Some(user) = &replied_msg.from {
            let user_id = user.id;
            
            // Get chat administrators
            if let Ok(admins) = bot.get_chat_administrators(original_msg.chat.id).await {
                let is_admin = admins.iter().any(|member| member.user.id == user_id);
                
                if is_admin {
                    // Admin users automatically pass moderation
                    return Ok("P".to_string());
                }
            }
        }

        // Proceed with AI moderation for non-admin users
        let prompt = format!(
            "Analyze the following message and determine if it violates any of these rules:

1. Is this message promoting something or trying to sell services? If yes then F
2. Is the message inviting either an individual or many to communicate in private? If yes then F

Return only either a P for pass or F for fail.

Message to analyze: \"{}\"",
            message_text
        );

        let request = Request::builder()
            .model(Model::GPT4oMini)
            .input(prompt)
            .max_output_tokens(20)
            .temperature(0.1)
            .build();

        let response = self.client.responses.create(request).await?;
        let result = response.output_text().trim().to_uppercase();
        
        // Ensure we only return P or F
        if result.contains('F') {
            Ok("F".to_string())
        } else {
            Ok("P".to_string())
        }
    }
} 