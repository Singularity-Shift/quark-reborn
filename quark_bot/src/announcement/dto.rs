use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizedAnnouncersConfig {
    pub usernames: Vec<String>,
}


