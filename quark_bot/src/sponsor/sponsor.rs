use std::env;

use anyhow::Result;
use sled::{Db, Tree};

use crate::sponsor::dto::{SponsorRequest, SponsorSettings, SponsorState};

#[derive(Clone)]
pub struct Sponsor {
    pub sponsor_settings_tree: Tree,
    pub sponsor_requests_tree: Tree,
    pub sponsor_state_tree: Tree,
    pub account_seed: String,
}

impl Sponsor {
    pub fn new(db: Db) -> Self {
        let account_seed: String =
            env::var("ACCOUNT_SEED").expect("ACCOUNT_SEED environment variable not found");

        let sponsor_settings_tree = db
            .open_tree("sponsor_settings")
            .expect("Failed to open sponsor settings tree");
        let sponsor_requests_tree = db
            .open_tree("sponsor_requests")
            .expect("Failed to open sponsor requests tree");
        let sponsor_state_tree = db
            .open_tree("sponsor_state")
            .expect("Failed to open sponsor state tree");

        Self {
            sponsor_settings_tree,
            sponsor_requests_tree,
            sponsor_state_tree,
            account_seed,
        }
    }

    pub fn get_sponsor_settings(&self, group_id: String) -> SponsorSettings {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        let settings = self.sponsor_settings_tree.get(group_id).unwrap();

        if let Some(settings) = settings {
            serde_json::from_slice(settings.as_ref()).unwrap_or_default()
        } else {
            SponsorSettings::default()
        }
    }

    pub fn set_or_update_sponsor_settings(
        &self,
        group_id: String,
        settings: SponsorSettings,
    ) -> Result<()> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        self.sponsor_settings_tree
            .fetch_and_update(group_id, |existing| {
                if let Some(existing) = existing {
                    let mut existing: SponsorSettings = serde_json::from_slice(existing).unwrap();
                    existing.requests = settings.requests;
                    existing.interval = settings.interval.clone();
                    Some(serde_json::to_vec(&existing).unwrap())
                } else {
                    Some(serde_json::to_vec(&settings).unwrap())
                }
            })
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub fn get_sponsor_requests(&self, group_id: String) -> Option<SponsorRequest> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        let requests = self.sponsor_requests_tree.get(group_id).unwrap();

        if let Some(requests) = requests {
            serde_json::from_slice(requests.as_ref()).unwrap_or_default()
        } else {
            None
        }
    }

    pub fn set_or_update_sponsor_requests(
        &self,
        group_id: String,
        requests: SponsorRequest,
    ) -> Result<()> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        self.sponsor_requests_tree
            .fetch_and_update(group_id, |existing| {
                if let Some(existing) = existing {
                    let mut existing: SponsorRequest = serde_json::from_slice(existing).unwrap();
                    existing.requests_left = requests.requests_left;
                    existing.last_request = requests.last_request;
                    Some(serde_json::to_vec(&existing).unwrap())
                } else {
                    Some(serde_json::to_vec(&requests).unwrap())
                }
            })?;

        Ok(())
    }

    /// Check if a request can be made and update the request count
    pub fn can_make_request(&self, group_id: String) -> Result<bool> {
        let settings = self.get_sponsor_settings(group_id.clone());
        let mut requests = self
            .get_sponsor_requests(group_id.clone())
            .unwrap_or(SponsorRequest {
                requests_left: settings.requests,
                last_request: 0,
            });

        let current_time = chrono::Utc::now().timestamp() as u64;

        // Check if we need to reset the interval
        if self.should_reset_interval(&settings, requests.last_request, current_time) {
            requests.requests_left = settings.requests;
            requests.last_request = current_time;
        }

        // Check if we have requests left
        if requests.requests_left > 0 {
            requests.requests_left -= 1;
            requests.last_request = current_time;
            self.set_or_update_sponsor_requests(group_id, requests)?;
            Ok(true)
        } else {
            // Update last request time even if we can't make the request
            requests.last_request = current_time;
            self.set_or_update_sponsor_requests(group_id, requests)?;
            Ok(false)
        }
    }

    /// Check if the interval should be reset based on the last request time
    fn should_reset_interval(
        &self,
        settings: &SponsorSettings,
        last_request: u64,
        current_time: u64,
    ) -> bool {
        let time_diff = current_time - last_request;

        match settings.interval {
            crate::sponsor::dto::SponsorInterval::Hourly => time_diff >= 3600, // 1 hour
            crate::sponsor::dto::SponsorInterval::Daily => time_diff >= 86400, // 24 hours
            crate::sponsor::dto::SponsorInterval::Weekly => time_diff >= 604800, // 7 days
            crate::sponsor::dto::SponsorInterval::Monthly => time_diff >= 2592000, // 30 days
        }
    }

    /// Get current request status and reset if interval has passed
    pub fn get_request_status_and_reset(&self, group_id: String) -> Result<(u64, u64)> {
        let settings = self.get_sponsor_settings(group_id.clone());
        let mut requests = self
            .get_sponsor_requests(group_id.clone())
            .unwrap_or(SponsorRequest {
                requests_left: settings.requests,
                last_request: 0,
            });

        let current_time = chrono::Utc::now().timestamp() as u64;

        // Check if we need to reset the interval
        if self.should_reset_interval(&settings, requests.last_request, current_time) {
            requests.requests_left = settings.requests;
            requests.last_request = current_time;
            // Update the database with the reset values
            self.set_or_update_sponsor_requests(group_id, requests)?;
            Ok((settings.requests, settings.requests))
        } else {
            Ok((requests.requests_left, settings.requests))
        }
    }

    /// Get sponsor state for a group
    pub fn get_sponsor_state(&self, group_id: String) -> Option<SponsorState> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        let state = self.sponsor_state_tree.get(group_id).unwrap();

        if let Some(state) = state {
            serde_json::from_slice(state.as_ref()).ok()
        } else {
            None
        }
    }

    /// Set or update sponsor state for a group
    pub fn set_sponsor_state(&self, group_id: String, state: SponsorState) -> Result<()> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        self.sponsor_state_tree
            .insert(group_id, serde_json::to_vec(&state)?)?;
        Ok(())
    }

    /// Remove sponsor state for a group
    pub fn remove_sponsor_state(&self, group_id: String) -> Result<()> {
        let group_id = format!("{}-{}", group_id, self.account_seed);
        self.sponsor_state_tree.remove(group_id)?;
        Ok(())
    }
}
