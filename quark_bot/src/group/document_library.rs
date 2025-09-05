use crate::user_conversation::dto::FileInfo;
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};

const TREE_NAME: &str = "group_documents";

#[derive(Serialize, Deserialize, Debug, Default, Clone, bincode::Encode, bincode::Decode)]
pub struct GroupDocumentData {
    pub vector_store_id: Option<String>,
    pub files: Vec<FileInfo>,
}

#[derive(Clone)]
pub struct GroupDocuments {
    tree: sled::Tree,
}

impl GroupDocuments {
    pub fn new(db: &Db) -> sled::Result<Self> {
        let tree = db.open_tree(TREE_NAME)?;
        Ok(Self { tree })
    }

    pub fn set_group_data(&self, group_id: String, data: &GroupDocumentData) -> sled::Result<()> {
        let key = group_id.as_bytes();
        let encoded = bincode::encode_to_vec(data, bincode::config::standard()).unwrap();
        self.tree.insert(key, encoded)?;
        Ok(())
    }

    pub fn get_group_data(&self, group_id: String) -> Option<GroupDocumentData> {
        let key = group_id.as_bytes();
        self.tree.get(key).ok().flatten().and_then(|ivec: IVec| {
            bincode::decode_from_slice(&ivec, bincode::config::standard())
                .ok()
                .map(|(data, _)| data)
        })
    }

    pub fn get_group_vector_store_id(&self, group_id: String) -> Option<String> {
        self.get_group_data(group_id)
            .and_then(|data| data.vector_store_id)
    }

    pub fn set_group_vector_store_id(&self, group_id: String, vector_store_id: &str) -> sled::Result<()> {
        let mut data = self.get_group_data(group_id.clone()).unwrap_or_default();
        data.vector_store_id = Some(vector_store_id.to_string());
        self.set_group_data(group_id, &data)
    }

    pub fn add_group_file(&self, group_id: String, file_id: &str, filename: &str) -> sled::Result<()> {
        let mut data = self.get_group_data(group_id.clone()).unwrap_or_default();
        if !data.files.iter().any(|f| f.id == file_id) {
            data.files.push(FileInfo {
                id: file_id.to_string(),
                name: filename.to_string(),
            });
        }
        self.set_group_data(group_id, &data)
    }

    pub fn get_group_files(&self, group_id: String) -> Vec<FileInfo> {
        self.get_group_data(group_id)
            .map(|data| data.files)
            .unwrap_or_else(Vec::new)
    }

    pub fn remove_group_file_id(&self, group_id: String, file_id: &str) -> sled::Result<()> {
        let mut data = self.get_group_data(group_id.clone()).unwrap_or_default();
        data.files.retain(|f| f.id != file_id);
        self.set_group_data(group_id, &data)
    }

    pub fn clear_group_files(&self, group_id: String) -> sled::Result<()> {
        let mut data = self.get_group_data(group_id.clone()).unwrap_or_default();
        data.files.clear();
        self.set_group_data(group_id, &data)
    }

    /// Clean up orphaned vector store references when vector store is not found in OpenAI
    pub fn cleanup_orphaned_group_vector_store(&self, group_id: String) -> sled::Result<()> {
        let mut data = self.get_group_data(group_id.clone()).unwrap_or_default();
        data.vector_store_id = None;
        data.files.clear();
        self.set_group_data(group_id, &data)
    }
}
