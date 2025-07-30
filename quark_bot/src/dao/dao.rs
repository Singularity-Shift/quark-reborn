use anyhow::Result;
use chrono::Utc;
use sled::Tree;

use crate::dao::dto::{DaoAdminPreferences, DaoEntry, DaoStatus};

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

                    let admin_preference_index = admin_preferences
                        .iter()
                        .position(|preference| preference.group_id == group_id);

                    if let Some(index) = admin_preference_index {
                        // Update existing preference
                        admin_preferences[index].expiration_time = preferences.expiration_time;
                        admin_preferences[index].interval_active_dao_notifications =
                            preferences.interval_active_dao_notifications;
                    } else {
                        // Add new preference
                        admin_preferences.push(preferences.clone());
                    }

                    Some(serde_json::to_vec(&admin_preferences).unwrap())
                } else {
                    Some(serde_json::to_vec(&vec![preferences.clone()]).unwrap())
                }
            })?;

        Ok(())
    }

    pub fn get_dao_admin_preferences(&self, group_id: String) -> Result<DaoAdminPreferences> {
        let admin_preferences = self.db.get("dao_admin_preferences")?;

        if admin_preferences.is_none() {
            return Ok(DaoAdminPreferences {
                group_id,
                expiration_time: Utc::now().timestamp() as u64 + 7 * 24 * 60 * 60,
                interval_active_dao_notifications: 3600,
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
            Ok(admin_preference.clone())
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

    pub fn create_dao(&self, dao: DaoEntry) -> Result<()> {
        let group = self.db.fetch_and_update("daos", |entries| {
            if let Some(daos) = entries {
                let daos_result: Result<Vec<DaoEntry>, serde_json::Error> =
                    serde_json::from_slice(daos);

                if daos_result.is_err() {
                    return None;
                }

                let mut daos = daos_result.unwrap();

                daos.push(dao.clone());

                Some(serde_json::to_vec(&daos).unwrap())
            } else {
                Some(serde_json::to_vec(&vec![dao.clone()]).unwrap())
            }
        });

        if group.is_err() {
            return Err(anyhow::anyhow!("Failed to create dao"));
        }

        Ok(())
    }

    pub fn get_active_daos(&self) -> Result<Vec<DaoEntry>> {
        let now = Utc::now().timestamp() as u64;

        let daos = self.db.update_and_fetch("daos", |entries| {
            if let Some(daos) = entries {
                let daos_result: Result<Vec<DaoEntry>, serde_json::Error> =
                    serde_json::from_slice(daos);

                if daos_result.is_err() {
                    return None;
                }

                let daos = daos_result.unwrap();

                let daos = daos
                    .into_iter()
                    .map(|dao| {
                        if dao.start_date <= now
                            && dao.end_date >= now
                            && dao.status == DaoStatus::Pending
                        {
                            let mut dao = dao.clone();
                            dao.status = DaoStatus::Active;
                            dao
                        } else {
                            dao
                        }
                    })
                    .collect::<Vec<DaoEntry>>();

                Some(serde_json::to_vec(&daos).unwrap())
            } else {
                None
            }
        })?;

        if daos.is_none() {
            return Ok(vec![]);
        }

        let daos_result: Result<Vec<DaoEntry>, serde_json::Error> =
            serde_json::from_slice(daos.unwrap().as_ref());

        if daos_result.is_err() {
            return Err(anyhow::anyhow!("Failed to get active daos"));
        }

        let daos = daos_result.unwrap();

        Ok(daos
            .into_iter()
            .filter(|dao| dao.start_date <= now && dao.end_date >= now)
            .collect())
    }

    pub fn remove_expired_daos(&self) -> Result<()> {
        let now = Utc::now().timestamp() as u64;

        let admin_preferences = self.get_all_dao_admin_preferences()?;

        self.db.fetch_and_update("daos", |entries| {
            if let Some(daos) = entries {
                let daos_result: Result<Vec<DaoEntry>, serde_json::Error> =
                    serde_json::from_slice(daos);

                if daos_result.is_err() {
                    return None;
                }

                let mut daos = daos_result.unwrap();

                let admin_preference = admin_preferences
                    .iter()
                    .find(|preference| daos.iter().any(|dao| dao.group_id == preference.group_id));

                if let Some(admin_preference) = admin_preference {
                    daos.retain(|dao| dao.end_date + admin_preference.expiration_time > now);
                } else {
                    daos.retain(|dao| dao.end_date + 7 * 24 * 60 * 60 > now);
                }

                Some(serde_json::to_vec(&daos).unwrap())
            } else {
                None
            }
        })?;

        Ok(())
    }

    pub fn get_dao_results(&self) -> Result<Vec<DaoEntry>> {
        let now = Utc::now().timestamp() as u64;

        let dao_results = self.db.update_and_fetch("dao_results", |entries| {
            if let Some(dao_results) = entries {
                let dao_results_result: Result<Vec<DaoEntry>, serde_json::Error> =
                    serde_json::from_slice(dao_results);

                if dao_results_result.is_err() {
                    return None;
                }

                let dao_results = dao_results_result.unwrap();

                let dao_results = dao_results
                    .into_iter()
                    .map(|dao_result| {
                        if dao_result.end_date < now && dao_result.status == DaoStatus::Active {
                            let mut dao_result = dao_result.clone();
                            dao_result.status = DaoStatus::Completed;
                            dao_result
                        } else {
                            dao_result
                        }
                    })
                    .collect::<Vec<DaoEntry>>();

                Some(serde_json::to_vec(&dao_results).unwrap())
            } else {
                None
            }
        })?;

        if dao_results.is_none() {
            return Ok(vec![]);
        }

        let dao_results_result: Result<Vec<DaoEntry>, serde_json::Error> =
            serde_json::from_slice(dao_results.unwrap().as_ref());

        let dao_results: Vec<DaoEntry> = dao_results_result.unwrap();

        let dao_results = dao_results
            .into_iter()
            .filter(|dao_result| dao_result.end_date < now)
            .collect::<Vec<DaoEntry>>();

        Ok(dao_results)
    }

    pub fn update_last_active_notification(&self, dao_id: String) -> Result<()> {
        let now = Utc::now().timestamp() as u64;

        self.db.fetch_and_update("daos", |entries| {
            if let Some(daos) = entries {
                let daos_result: Result<Vec<DaoEntry>, serde_json::Error> =
                    serde_json::from_slice(daos);

                if daos_result.is_err() {
                    return None;
                }

                let mut daos = daos_result.unwrap();

                let dao = daos.iter_mut().find(|dao| dao.dao_id == dao_id);

                if let Some(dao) = dao {
                    dao.last_active_notification = now;
                }

                Some(serde_json::to_vec(&daos).unwrap())
            } else {
                None
            }
        })?;

        Ok(())
    }

    pub fn update_result_notified(&self, dao_id: String) -> Result<()> {
        self.db.fetch_and_update("daos", |entries| {
            if let Some(daos) = entries {
                let daos_result: Result<Vec<DaoEntry>, serde_json::Error> =
                    serde_json::from_slice(daos);

                if daos_result.is_err() {
                    return None;
                }

                let mut daos = daos_result.unwrap();

                let dao = daos.iter_mut().find(|dao| dao.dao_id == dao_id);

                if let Some(dao) = dao {
                    dao.result_notified = true;
                }

                Some(serde_json::to_vec(&daos).unwrap())
            } else {
                None
            }
        })?;

        Ok(())
    }
}
