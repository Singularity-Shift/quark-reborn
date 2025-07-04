use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
}
