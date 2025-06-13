use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
}
