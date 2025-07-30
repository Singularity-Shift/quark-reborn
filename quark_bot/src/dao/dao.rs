use anyhow::Result;
use chrono::Utc;
use sled::Tree;

use crate::dao::dto::{DaoAdminPreferences, ProposalEntry, ProposalStatus};

#[derive(Clone)]
pub struct Dao {
    db: Tree,
}

impl Dao {
    pub fn new(db: Tree) -> Self {
        Self { db }
    }

    pub fn set_dao_admin_preferences(
        &self,
        group_id: String,
        preferences: DaoAdminPreferences,
    ) -> Result<()> {
        self.db
            .fetch_and_update("dao_admin_preferences", |entries| {
                if let Some(admin_preferences) = entries {
                    let admin_preferences_result: Result<
                        Vec<DaoAdminPreferences>,
                        serde_json::Error,
                    > = serde_json::from_slice(admin_preferences);

                    if admin_preferences_result.is_err() {
                        return None;
                    }

                    let mut admin_preferences = admin_preferences_result.unwrap();

                    // Find and update existing preference or add new one
                    let existing_index = admin_preferences
                        .iter()
                        .position(|preference| preference.group_id == group_id);

                    if let Some(index) = existing_index {
                        // Update existing preference
                        admin_preferences[index].expiration_time = preferences.expiration_time;
                        admin_preferences[index].interval_active_proposal_notifications =
                            preferences.interval_active_proposal_notifications;
                        admin_preferences[index].default_dao_token = preferences.default_dao_token.to_uppercase();
                    } else {
                        // Add new preference with uppercase token
                        let mut new_prefs = preferences.clone();
                        new_prefs.default_dao_token = new_prefs.default_dao_token.to_uppercase();
                        admin_preferences.push(new_prefs);
                    }

                    Some(serde_json::to_vec(&admin_preferences).unwrap())
                } else {
                    let mut new_prefs = preferences.clone();
                    new_prefs.default_dao_token = new_prefs.default_dao_token.to_uppercase();
                    Some(serde_json::to_vec(&vec![new_prefs]).unwrap())
                }
            })?;

        Ok(())
    }

    pub fn get_dao_admin_preferences(&self, group_id: String) -> Result<DaoAdminPreferences> {
        let admin_preferences = self.db.get("dao_admin_preferences")?;

        if admin_preferences.is_none() {
            return Ok(DaoAdminPreferences {
                group_id,
                expiration_time: 7 * 24 * 60 * 60, // 7 days in seconds
                interval_active_proposal_notifications: 3600, // 1 hour in seconds
                default_dao_token: "📒".to_string(),
            });
        }

        let admin_preferences_result: Result<Vec<DaoAdminPreferences>, serde_json::Error> =
            serde_json::from_slice(admin_preferences.unwrap().as_ref());

        if admin_preferences_result.is_err() {
            return Err(anyhow::anyhow!("Failed to get admin preferences"));
        }

        let admin_preferences = admin_preferences_result.unwrap();

        let admin_preference = admin_preferences
            .iter()
            .find(|preference| preference.group_id == group_id);

        if let Some(admin_preference) = admin_preference {
            let mut corrected_preference = admin_preference.clone();
            
            // Fix corrupted expiration_time values (detect if they're timestamps instead of durations)
            // If expiration_time is greater than 100 years in seconds, it's likely a corrupted timestamp
            if corrected_preference.expiration_time > 100 * 365 * 24 * 60 * 60 {
                log::warn!("Detected corrupted expiration_time: {}, resetting to default 7 days", corrected_preference.expiration_time);
                corrected_preference.expiration_time = 7 * 24 * 60 * 60; // 7 days in seconds
            }
            
            // Ensure notification interval is reasonable (between 5 minutes and 7 days)
            if corrected_preference.interval_active_proposal_notifications < 5 * 60 || 
               corrected_preference.interval_active_proposal_notifications > 7 * 24 * 60 * 60 {
                log::warn!("Detected invalid notification interval: {}, resetting to default 1 hour", corrected_preference.interval_active_proposal_notifications);
                corrected_preference.interval_active_proposal_notifications = 3600; // 1 hour in seconds
            }
            
            // Ensure default_dao_token is set and uppercase
            if corrected_preference.default_dao_token.is_empty() {
                log::warn!("Detected empty default_dao_token, setting to default 📒");
                corrected_preference.default_dao_token = "📒".to_string();
            } else {
                corrected_preference.default_dao_token = corrected_preference.default_dao_token.to_uppercase();
            }
            
            // If we corrected any values, save them back to the database
            if corrected_preference.expiration_time != admin_preference.expiration_time ||
               corrected_preference.interval_active_proposal_notifications != admin_preference.interval_active_proposal_notifications ||
               corrected_preference.default_dao_token != admin_preference.default_dao_token {
                if let Err(e) = self.set_dao_admin_preferences(group_id.clone(), corrected_preference.clone()) {
                    log::error!("Failed to save corrected DAO preferences: {}", e);
                }
            }
            
            Ok(corrected_preference)
        } else {
            Err(anyhow::anyhow!("No admin preference found"))
        }
    }

    pub fn get_all_dao_admin_preferences(&self) -> Result<Vec<DaoAdminPreferences>> {
        let admin_preferences = self.db.get("dao_admin_preferences")?;

        if admin_preferences.is_none() {
            return Ok(vec![]);
        }

        let admin_preferences_result: Result<Vec<DaoAdminPreferences>, serde_json::Error> =
            serde_json::from_slice(admin_preferences.unwrap().as_ref());

        if admin_preferences_result.is_err() {
            return Err(anyhow::anyhow!("Failed to get all admin preferences"));
        }

        let admin_preferences = admin_preferences_result.unwrap();

        Ok(admin_preferences)
    }

    pub fn create_proposal(&self, proposal: ProposalEntry) -> Result<()> {
        let group = self.db.fetch_and_update("proposals", |entries| {
            if let Some(proposals) = entries {
                let proposals_result: Result<Vec<ProposalEntry>, serde_json::Error> =
                    serde_json::from_slice(proposals);

                if proposals_result.is_err() {
                    return None;
                }

                let mut proposals = proposals_result.unwrap();

                proposals.push(proposal.clone());

                Some(serde_json::to_vec(&proposals).unwrap())
            } else {
                Some(serde_json::to_vec(&vec![proposal.clone()]).unwrap())
            }
        });

        if group.is_err() {
            return Err(anyhow::anyhow!("Failed to create proposal"));
        }

        Ok(())
    }

    pub fn get_active_proposals(&self) -> Result<Vec<ProposalEntry>> {
        let now = Utc::now().timestamp() as u64;

        let proposals = self.db.update_and_fetch("proposals", |entries| {
            if let Some(proposals) = entries {
                let proposals_result: Result<Vec<ProposalEntry>, serde_json::Error> =
                    serde_json::from_slice(proposals);

                if proposals_result.is_err() {
                    return None;
                }

                let proposals = proposals_result.unwrap();

                let proposals = proposals
                    .into_iter()
                    .map(|dao| {
                        if dao.start_date <= now
                            && dao.end_date >= now
                            && dao.status == ProposalStatus::Pending
                        {
                            let mut dao = dao.clone();
                            dao.status = ProposalStatus::Active;
                            dao
                        } else {
                            dao
                        }
                    })
                    .collect::<Vec<ProposalEntry>>();

                Some(serde_json::to_vec(&proposals).unwrap())
            } else {
                None
            }
        })?;

        if proposals.is_none() {
            return Ok(vec![]);
        }

        let proposals_result: Result<Vec<ProposalEntry>, serde_json::Error> =
            serde_json::from_slice(proposals.unwrap().as_ref());

        if proposals_result.is_err() {
            return Err(anyhow::anyhow!("Failed to get active proposals"));
        }

        let proposals = proposals_result.unwrap();

        Ok(proposals
            .into_iter()
            .filter(|dao| dao.start_date <= now && dao.end_date >= now)
            .collect())
    }

    pub fn remove_expired_proposals(&self) -> Result<()> {
        let now = Utc::now().timestamp() as u64;

        let admin_preferences = self.get_all_dao_admin_preferences()?;

        self.db.fetch_and_update("proposals", |entries| {
            if let Some(proposals) = entries {
                let proposals_result: Result<Vec<ProposalEntry>, serde_json::Error> =
                    serde_json::from_slice(proposals);

                if proposals_result.is_err() {
                    return None;
                }

                let mut proposals = proposals_result.unwrap();

                let admin_preference = admin_preferences
                    .iter()
                    .find(|preference| proposals.iter().any(|dao| dao.group_id == preference.group_id));

                if let Some(admin_preference) = admin_preference {
                    proposals.retain(|dao| dao.end_date + admin_preference.expiration_time > now);
                } else {
                    proposals.retain(|dao| dao.end_date + 7 * 24 * 60 * 60 > now);
                }

                Some(serde_json::to_vec(&proposals).unwrap())
            } else {
                None
            }
        })?;

        Ok(())
    }

    pub fn get_proposal_results(&self) -> Result<Vec<ProposalEntry>> {
        let now = Utc::now().timestamp() as u64;

        let proposal_results = self.db.update_and_fetch("proposal_results", |entries| {
            if let Some(proposal_results) = entries {
                let proposal_results_result: Result<Vec<ProposalEntry>, serde_json::Error> =
                    serde_json::from_slice(proposal_results);

                if proposal_results_result.is_err() {
                    return None;
                }

                let proposal_results = proposal_results_result.unwrap();

                let proposal_results = proposal_results
                    .into_iter()
                    .map(|dao_result| {
                        if dao_result.end_date < now && dao_result.status == ProposalStatus::Active {
                            let mut dao_result = dao_result.clone();
                            dao_result.status = ProposalStatus::Completed;
                            dao_result
                        } else {
                            dao_result
                        }
                    })
                    .collect::<Vec<ProposalEntry>>();

                Some(serde_json::to_vec(&proposal_results).unwrap())
            } else {
                None
            }
        })?;

        if proposal_results.is_none() {
            return Ok(vec![]);
        }

        let proposal_results_result: Result<Vec<ProposalEntry>, serde_json::Error> =
            serde_json::from_slice(proposal_results.unwrap().as_ref());

        let proposal_results: Vec<ProposalEntry> = proposal_results_result.unwrap();

        let proposal_results = proposal_results
            .into_iter()
            .filter(|dao_result| dao_result.end_date < now)
            .collect::<Vec<ProposalEntry>>();

        Ok(proposal_results)
    }

    pub fn update_last_active_notification(&self, proposal_id: String) -> Result<()> {
        let now = Utc::now().timestamp() as u64;

        self.db.fetch_and_update("proposals", |entries| {
            if let Some(proposals) = entries {
                let proposals_result: Result<Vec<ProposalEntry>, serde_json::Error> =
                    serde_json::from_slice(proposals);

                if proposals_result.is_err() {
                    return None;
                }

                let mut proposals = proposals_result.unwrap();

                let dao = proposals.iter_mut().find(|dao| dao.proposal_id == proposal_id);

                if let Some(dao) = dao {
                    dao.last_active_notification = now;
                }

                Some(serde_json::to_vec(&proposals).unwrap())
            } else {
                None
            }
        })?;

        Ok(())
    }

    pub fn update_result_notified(&self, proposal_id: String) -> Result<()> {
        self.db.fetch_and_update("proposals", |entries| {
            if let Some(proposals) = entries {
                let proposals_result: Result<Vec<ProposalEntry>, serde_json::Error> =
                    serde_json::from_slice(proposals);

                if proposals_result.is_err() {
                    return None;
                }

                let mut proposals = proposals_result.unwrap();

                let dao = proposals.iter_mut().find(|dao| dao.proposal_id == proposal_id);

                if let Some(dao) = dao {
                    dao.result_notified = true;
                }

                Some(serde_json::to_vec(&proposals).unwrap())
            } else {
                None
            }
        })?;

        Ok(())
    }
}
