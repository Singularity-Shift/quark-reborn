use super::dto::FileInfo;
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};

const TREE_NAME: &str = "user_conversations";

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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
        let encoded = bincode::serialize(data).unwrap();
        self.tree.insert(key, encoded)?;
        Ok(())
    }

    pub fn get_user_data(&self, user_id: i64) -> Option<UserData> {
        let key = user_id.to_be_bytes();
        self.tree
            .get(key)
            .ok()
            .flatten()
            .and_then(|ivec: IVec| bincode::deserialize(&ivec).ok())
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

    pub fn set_wallet_address(&self, user_id: i64, wallet_address: &str) -> sled::Result<()> {
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.wallet_address = Some(wallet_address.to_string());
        self.set_user_data(user_id, &data)
    }

    pub fn get_wallet_address(&self, user_id: i64) -> Option<String> {
        self.get_user_data(user_id)
            .and_then(|data| data.wallet_address)
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

    pub fn get_file_ids(&self, user_id: i64) -> Vec<String> {
        self.get_files(user_id).into_iter().map(|f| f.id).collect()
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

    // --- New image URL helpers ---
    pub fn set_last_image_urls(&self, user_id: i64, urls: &[String]) -> sled::Result<()> {
        log::info!("CACHE SET: Storing {} image URLs for user {}: {:?}", urls.len(), user_id, urls);
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        data.last_image_urls = urls.to_vec();
        let result = self.set_user_data(user_id, &data);
        match &result {
            Ok(_) => log::info!("CACHE SET SUCCESS: URLs stored for user {}", user_id),
            Err(e) => log::error!("CACHE SET ERROR: Failed to store URLs for user {}: {}", user_id, e),
        }
        result
    }

    /// Retrieve and clear stored image URLs (so they are used only once)
    pub fn take_last_image_urls(&self, user_id: i64) -> Vec<String> {
        log::info!("CACHE TAKE: Retrieving and clearing image URLs for user {}", user_id);
        let mut data = self.get_user_data(user_id).unwrap_or_default();
        let urls = std::mem::take(&mut data.last_image_urls);
        log::info!("CACHE TAKE: Found {} URLs for user {}: {:?}", urls.len(), user_id, urls);
        let save_result = self.set_user_data(user_id, &data);
        match save_result {
            Ok(_) => log::info!("CACHE TAKE SUCCESS: URLs cleared for user {}", user_id),
            Err(e) => log::error!("CACHE TAKE ERROR: Failed to clear URLs for user {}: {}", user_id, e),
        }
        urls
    }
}
