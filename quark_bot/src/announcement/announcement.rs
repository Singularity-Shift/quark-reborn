use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::dto::AuthorizedAnnouncersConfig;

#[derive(Debug, Clone)]
pub struct AnnouncerAuth {
    authorized_usernames: HashSet<String>,
}

impl AnnouncerAuth {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config_content = fs::read_to_string(config_path.as_ref())
            .with_context(|| {
                format!(
                    "Failed to read authorized announcers config from {:?}",
                    config_path.as_ref()
                )
            })?;

        let config: AuthorizedAnnouncersConfig = ron::from_str(&config_content)
            .context("Failed to parse authorized announcers config RON")?;

        let authorized_usernames: HashSet<String> = config.usernames.into_iter().collect();

        log::info!("Loaded {} authorized announcers", authorized_usernames.len());

        Ok(Self { authorized_usernames })
    }

    pub fn is_authorized(&self, username: &str) -> bool {
        self.authorized_usernames.contains(username)
    }
}


