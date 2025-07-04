use super::dto::FileInfo;
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};

const TREE_NAME: &str = "user_conversations";

#[derive(Serialize, Deserialize, Debug, Default, Clone, bincode::Encode, bincode::Decode)]
pub struct UserData {
    pub response_id: Option<String>,
    pub vector_store_id: Option<String>,
    pub wallet_address: Option<String>,
    pub files: Vec<FileInfo>,
    pub last_image_urls: Vec<String>,
}

#[derive(Clone)]
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
        let encoded = bincode::encode_to_vec(data, bincode::config::standard()).unwrap();
        self.tree.insert(key, encoded)?;
        Ok(())
    }

    pub fn get_user_data(&self, user_id: i64) -> Option<UserData> {
        let key = user_id.to_be_bytes();
        self.tree.get(key).ok().flatten().and_then(|ivec: IVec| {
            bincode::decode_from_slice(&ivec, bincode::config::standard())
                .ok()
                .map(|(data, _)| data)
        })
    }

    pub fn set_response_id(&self, user_id: i64, response_id: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.response_id = Some(response_id.to_string());
        self.set_user_data(user_id, &data)
    }

    pub fn get_response_id(&self, user_id: i64) -> Option<String> {
        self.get_user_data(user_id)
            .and_then(|data| data.response_id)
    }

    pub fn set_vector_store_id(&self, user_id: i64, vector_store_id: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.vector_store_id = Some(vector_store_id.to_string());
        self.set_user_data(user_id, &data)
    }

    pub fn get_vector_store_id(&self, user_id: i64) -> Option<String> {
        self.get_user_data(user_id)
            .and_then(|data| data.vector_store_id)
    }

    pub fn add_file(&self, user_id: i64, file_id: &str, filename: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        if !data.files.iter().any(|f| f.id == file_id) {
            data.files.push(FileInfo {
                id: file_id.to_string(),
                name: filename.to_string(),
            });
        }
        self.set_user_data(user_id, &data)
    }

    pub fn get_files(&self, user_id: i64) -> Vec<FileInfo> {
        self.get_user_data(user_id)
            .map(|data| data.files)
            .unwrap_or_else(Vec::new)
    }

    pub fn remove_file_id(&self, user_id: i64, file_id: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.files.retain(|f| f.id != file_id);
        self.set_user_data(user_id, &data)
    }

    pub fn clear_files(&self, user_id: i64) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.files.clear();
        self.set_user_data(user_id, &data)
    }

    pub fn clear_response_id(&self, user_id: i64) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.response_id = None;
        self.set_user_data(user_id, &data)
    }

    /// Clean up orphaned vector store references when vector store is not found in OpenAI
    pub fn cleanup_orphaned_vector_store(&self, user_id: i64) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.vector_store_id = None;
        data.files.clear();
        self.set_user_data(user_id, &data)
    }
}
