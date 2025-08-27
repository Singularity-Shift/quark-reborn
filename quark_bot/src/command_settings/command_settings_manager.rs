use std::env;

use anyhow::Result;
use sled::{Db, Tree};

use crate::command_settings::dto::CommandSettings;

#[derive(Clone)]
pub struct CommandSettingsManager {
    pub command_settings_tree: Tree,
    pub account_seed: String,
}

impl CommandSettingsManager {
    pub fn new(db: Db) -> Self {
        let account_seed: String =
            env::var("ACCOUNT_SEED").expect("ACCOUNT_SEED environment variable not found");

        let command_settings_tree = db
            .open_tree("command_settings")
            .expect("Failed to open command settings tree");

        Self {
            command_settings_tree,
            account_seed,
        }
    }

    pub fn get_command_settings(&self, group_id: String) -> CommandSettings {
        let formatted_group_id = format!("{}-{}", group_id, self.account_seed);
        match self.command_settings_tree.get(formatted_group_id) {
            Ok(Some(bytes)) => match serde_json::from_slice(bytes.as_ref()) {
                Ok(settings) => settings,
                Err(e) => {
                    log::error!("Failed to deserialize CommandSettings for group {}: {}", group_id, e);
                    CommandSettings::default()
                }
            },
            Ok(None) => CommandSettings::default(),
            Err(e) => {
                log::error!("sled error reading command settings: {}", e);
                CommandSettings::default()
            }
        }
    }

    pub fn set_command_settings(&self, group_id: String, settings: CommandSettings) -> Result<()> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        let json_data = match serde_json::to_vec(&settings) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to serialize CommandSettings for group {}: {}", group_id, e);
                return Err(anyhow::anyhow!("JSON serialization failed: {}", e));
            }
        };
        self.command_settings_tree
            .fetch_and_update(group_id, |_| Some(json_data.clone()))
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub fn is_chat_commands_enabled(&self, group_id: String) -> bool {
        let settings = self.get_command_settings(group_id);
        settings.chat_commands_enabled
    }
}
