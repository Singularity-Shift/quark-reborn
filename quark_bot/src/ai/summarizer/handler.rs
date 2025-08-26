use crate::ai::summarizer::dto::SummarizerState;
use crate::ai::summarizer::helpers::{
    build_summarization_prompt, generate_summary, get_conversation_summary_key, should_summarize,
};
use crate::dependencies::BotDependencies;
use crate::utils::create_purchase_request;
use open_ai_rust_responses_by_sshift::{Client as OAIClient, Model};
use sled::Db;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct SummarizerService {
    tree: sled::Tree,
    openai_client: OAIClient,
}

impl SummarizerService {
    pub fn new(db: Db, openai_client: OAIClient) -> Self {
        let tree = db
            .open_tree("conversation_summaries")
            .expect("Failed to open conversation_summaries tree");
        
        Self {
            tree,
            openai_client,
        }
    }

    pub fn get_state(&self, user_id: i64) -> Option<SummarizerState> {
        let key = get_conversation_summary_key(user_id);
        self.tree
            .get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|ivec| {
                match serde_json::from_slice::<SummarizerState>(&ivec) {
                    Ok(state) => Some(state),
                    Err(e) => {
                        log::error!("Failed to deserialize SummarizerState for user {}: {}", user_id, e);
                        None
                    }
                }
            })
    }

    pub fn save_state(&self, user_id: i64, state: &SummarizerState) -> sled::Result<()> {
        let key = get_conversation_summary_key(user_id);
        let json_data = match serde_json::to_vec(state) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to serialize SummarizerState for user {}: {}", user_id, e);
                return Err(sled::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("JSON serialization failed for user {}: {}", user_id, e)
                )));
            }
        };
        self.tree.insert(key.as_bytes(), json_data)?;
        Ok(())
    }

    pub async fn check_and_summarize(
        &self,
        user_id: i64,
        total_tokens: u32,
        token_limit: u32,
        latest_user_input: &str,
        latest_assistant_reply: &str,
        bot_deps: BotDependencies,
        group_id: Option<String>,
        jwt: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        if !should_summarize(total_tokens, token_limit) {
            return Ok(None);
        }

        log::info!(
            "Token limit exceeded for user {}: {} > {}, triggering summarization",
            user_id, total_tokens, token_limit
        );

        let prompt = build_summarization_prompt(
            latest_user_input,
            latest_assistant_reply,
        );

        let (new_summary, tokens_used) = match generate_summary(&self.openai_client, &prompt).await {
            Ok(result) => {
                log::info!(
                    "Successfully generated summary for user {}: {} characters",
                    user_id,
                    result.summary.len()
                );
                (result.summary, result.total_tokens)
            }
            Err(e) => {
                log::error!("Failed to generate summary for user {}: {}", user_id, e);
                return Err(e);
            }
        };

        // Charge for the summarization call
        let user_id_str = user_id.to_string();
        if let Err(e) = create_purchase_request(
            0, // file_search_calls
            0, // web_search_calls  
            0, // image_generation_calls
            tokens_used, // Use actual tokens from the summarization API call
            Model::GPT5Nano, // Summarization model
            jwt, // Use the actual JWT token
            group_id.clone(),
            Some(user_id_str),
            bot_deps.clone(),
        ).await {
            log::error!("Failed to charge for summarization for user {}: {}", user_id, e);
            // Don't fail the summarization, just log the payment error
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let new_state = SummarizerState {
            summary: Some(new_summary.clone()),
            last_rollover_unix: now,
            pending_thread_clear: true,
        };

        if let Err(e) = self.save_state(user_id, &new_state) {
            log::error!("Failed to save summarizer state for user {}: {}", user_id, e);
            return Err(anyhow::anyhow!("Failed to save summarizer state: {}", e));
        }

        log::info!(
            "Successfully saved summarizer state for user {} with {} character summary",
            user_id,
            new_summary.len()
        );

        Ok(Some(new_summary))
    }

    pub fn get_summary_for_instructions(&self, user_id: i64) -> Option<String> {
        self.get_state(user_id)
            .and_then(|state| state.summary)
    }

    pub fn clear_summary(&self, user_id: i64) -> sled::Result<()> {
        let key = get_conversation_summary_key(user_id);
        self.tree.remove(key.as_bytes())?;
        Ok(())
    }

    pub fn check_and_clear_pending_thread(&self, user_id: i64) -> Result<bool, anyhow::Error> {
        if let Some(mut state) = self.get_state(user_id) {
            if state.pending_thread_clear {
                // Reset the flag since we're handling it now
                state.pending_thread_clear = false;
                if let Err(e) = self.save_state(user_id, &state) {
                    log::error!("Failed to update summarizer state for user {}: {}", user_id, e);
                    return Err(anyhow::anyhow!("Failed to update summarizer state: {}", e));
                }
                return Ok(true);
            }
        }
        Ok(false)
    }
}
