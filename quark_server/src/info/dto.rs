use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct Info {
    name: String,
    version: String,
}

impl From<(String, String)> for Info {
    fn from(value: (String, String)) -> Self {
        let (name, version) = value;

        Self { name, version }
    }
}
