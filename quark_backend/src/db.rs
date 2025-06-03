use sled::{Db, IVec};

const TREE_NAME: &str = "user_conversations";

pub struct UserConversations {
    tree: sled::Tree,
}

impl UserConversations {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn set_response_id(&self, user_id: i64, response_id: &str) -> sled::Result<()> {
        let key = user_id.to_be_bytes();
        self.tree.insert(key, response_id.as_bytes())?;
        Ok(())
    }

    pub fn get_response_id(&self, user_id: i64) -> Option<String> {
        let key = user_id.to_be_bytes();
        self.tree.get(key).ok().flatten().and_then(|ivec: IVec| {
            String::from_utf8(ivec.to_vec()).ok()
        })
    }
} 