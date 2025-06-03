use sled::{Db, IVec};
use serde::{Serialize, Deserialize};

const TREE_NAME: &str = "user_conversations";

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct UserData {
    pub response_id: Option<String>,
    pub vector_store_id: Option<String>,
    pub wallet_address: Option<String>,
}

pub struct UserConversations {
    tree: sled::Tree,
}

impl UserConversations {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn set_user_data(&self, user_id: i64, data: &UserData) -> sled::Result<()> {
        let key = user_id.to_be_bytes();
        let encoded = bincode::serialize(data).unwrap();
        self.tree.insert(key, encoded)?;
        Ok(())
    }

    pub fn get_user_data(&self, user_id: i64) -> Option<UserData> {
        let key = user_id.to_be_bytes();
        self.tree.get(key).ok().flatten().and_then(|ivec: IVec| {
            bincode::deserialize(&ivec).ok()
        })
    }

    pub fn set_response_id(&self, user_id: i64, response_id: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.response_id = Some(response_id.to_string());
        self.set_user_data(user_id, &data)
    }

    pub fn get_response_id(&self, user_id: i64) -> Option<String> {
        self.get_user_data(user_id).and_then(|data| data.response_id)
    }

    pub fn set_vector_store_id(&self, user_id: i64, vector_store_id: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.vector_store_id = Some(vector_store_id.to_string());
        self.set_user_data(user_id, &data)
    }

    pub fn get_vector_store_id(&self, user_id: i64) -> Option<String> {
        self.get_user_data(user_id).and_then(|data| data.vector_store_id)
    }

    pub fn set_wallet_address(&self, user_id: i64, wallet_address: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.wallet_address = Some(wallet_address.to_string());
        self.set_user_data(user_id, &data)
    }

    pub fn get_wallet_address(&self, user_id: i64) -> Option<String> {
        self.get_user_data(user_id).and_then(|data| data.wallet_address)
    }
} 