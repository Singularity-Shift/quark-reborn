use std::env;

use sled::{Db, Tree};

#[derive(Clone)]
pub struct SentinelService {
    pub(crate) db: Tree,
    pub(crate) account_seed: String,
}

impl SentinelService {
    pub fn new(db: Db) -> Self {
        let account_seed: String =
            env::var("ACCOUNT_SEED").expect("ACCOUNT_SEED environment variable not found");

        let tree = db.open_tree("sentinel").unwrap();
        Self {
            db: tree,
            account_seed,
        }
    }

    pub fn get_sentinel(&self, chat_id: String) -> bool {
        let key = format!("{}_{}", chat_id, self.account_seed);
        let value = self.db.get(key.as_bytes());

        if value.is_err() {
            return false;
        }

        let value = value.unwrap();

        if value.is_none() {
            return false;
        }

        let value = value.unwrap();

        let value: bool = serde_json::from_slice(&value).unwrap_or(false);

        value
    }

    pub fn set_sentinel(&self, chat_id: String, value: bool) {
        let key = format!("{}_{}", chat_id, self.account_seed);
        self.db
            .insert(key.as_bytes(), value.to_string().as_bytes())
            .unwrap();
    }
}
