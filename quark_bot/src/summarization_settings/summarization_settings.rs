use super::dto::{EffectiveSummarizationPrefs, SummarizationPrefs};
use sled::{Db, Tree};
use std::env;

const TREE_NAME: &str = "summarization_prefs";

fn get_summarization_prefs_key(user_id: &str, group_id: Option<String>) -> String {
    match group_id {
        Some(gid) => gid,
        None => user_id.to_string(),
    }
}

#[derive(Clone)]
pub struct SummarizationSettings {
    tree: Tree,
}

impl SummarizationSettings {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn get(&self, user_id: &str, group_id: Option<String>) -> SummarizationPrefs {
        let key = get_summarization_prefs_key(user_id, group_id);
        match self.tree.get(&key) {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
            _ => SummarizationPrefs::default(),
        }
    }

    pub fn set(
        &self,
        user_id: &str,
        group_id: Option<String>,
        prefs: &SummarizationPrefs,
    ) -> sled::Result<()> {
        let key = get_summarization_prefs_key(user_id, group_id);
        let bytes = match serde_json::to_vec(prefs) {
            Ok(data) => data,
            Err(e) => {
                log::error!(
                    "Failed to serialize SummarizationPrefs for user {}: {}",
                    user_id,
                    e
                );
                return Err(sled::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("JSON serialization failed for user {}: {}", user_id, e),
                )));
            }
        };
        self.tree.insert(key, bytes)?;
        Ok(())
    }

    pub fn set_enabled(
        &self,
        user_id: &str,
        group_id: Option<String>,
        enabled: bool,
    ) -> sled::Result<()> {
        let mut prefs = self.get(user_id, group_id.clone());
        prefs.summarizer_enabled = Some(enabled);
        self.set(user_id, group_id, &prefs)
    }

    pub fn set_token_limit(
        &self,
        user_id: &str,
        group_id: Option<String>,
        limit: u32,
    ) -> sled::Result<()> {
        let mut prefs = self.get(user_id, group_id.clone());
        prefs.summarizer_token_limit = Some(limit);
        self.set(user_id, group_id, &prefs)
    }

    pub fn get_effective_prefs(
        &self,
        user_id: &str,
        group_id: Option<String>,
    ) -> EffectiveSummarizationPrefs {
        let prefs = self.get(user_id, group_id);

        // Resolve enabled: user pref -> env (both spellings) -> default true
        let enabled = prefs.summarizer_enabled.unwrap_or_else(|| {
            env::var("SUMMARIZER_ENABLED")
                .or_else(|_| env::var("SUMMARIZER"))
                .or_else(|_| env::var("summarizer_enabled"))
                .or_else(|_| env::var("summarizer"))
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true)
        });

        // Resolve token limit: user pref -> env (both spellings) -> default 18000
        let token_limit = prefs.summarizer_token_limit.unwrap_or_else(|| {
            env::var("CONVERSATION_TOKEN_LIMIT")
                .or_else(|_| env::var("conversation_token_limit"))
                .unwrap_or_else(|_| "18000".to_string())
                .parse::<u32>()
                .unwrap_or(18000)
        });

        EffectiveSummarizationPrefs {
            enabled,
            token_limit,
        }
    }
}
