use super::dto::{EffectiveSummarizationPrefs, SummarizationPrefs};
use sled::{Db, Tree};
use std::env;

const TREE_NAME: &str = "summarization_prefs";

#[derive(Clone)]
pub struct SummarizationSettings {
    tree: Tree,
}

impl SummarizationSettings {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn get(&self, user_id: i64) -> SummarizationPrefs {
        let key = user_id.to_string();
        match self.tree.get(&key) {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
            _ => SummarizationPrefs::default(),
        }
    }

    pub fn set(&self, user_id: i64, prefs: &SummarizationPrefs) -> sled::Result<()> {
        let key = user_id.to_string();
        let bytes = serde_json::to_vec(prefs).unwrap();
        self.tree.insert(key, bytes)?;
        Ok(())
    }

    pub fn set_enabled(&self, user_id: i64, enabled: bool) -> sled::Result<()> {
        let mut prefs = self.get(user_id);
        prefs.summarizer_enabled = Some(enabled);
        self.set(user_id, &prefs)
    }

    pub fn set_token_limit(&self, user_id: i64, limit: u32) -> sled::Result<()> {
        let mut prefs = self.get(user_id);
        prefs.summarizer_token_limit = Some(limit);
        self.set(user_id, &prefs)
    }

    pub fn get_effective_prefs(&self, user_id: i64) -> EffectiveSummarizationPrefs {
        let prefs = self.get(user_id);
        
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

        // Resolve token limit: user pref -> env (both spellings) -> default 12000
        let token_limit = prefs.summarizer_token_limit.unwrap_or_else(|| {
            env::var("CONVERSATION_TOKEN_LIMIT")
                .or_else(|_| env::var("conversation_token_limit"))
                .unwrap_or_else(|_| "12000".to_string())
                .parse::<u32>()
                .unwrap_or(12000)
        });

        EffectiveSummarizationPrefs {
            enabled,
            token_limit,
        }
    }
}
