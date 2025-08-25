use anyhow::Result;
use sled::{Db, Tree};

use crate::filters::dto::{
    FilterDefinition, FilterError, FilterMatch, FilterMetadata, FilterStats, MatchType,
    PendingFilterWizardState, ValidationResult,
};

#[derive(Clone)]
pub struct Filters {
    pub filters_db: Tree,
    pub metadata_db: Tree,
    pub stats_db: Tree,
    pub settings_db: Tree,
    pub account_seed: String,
}

impl Filters {
    pub fn new(db: &Db) -> Self {
        let filters_db = db.open_tree("filters").expect("Failed to open filters tree");
        let metadata_db = db.open_tree("filter_metadata").expect("Failed to open filter metadata tree");
        let stats_db = db.open_tree("filter_stats").expect("Failed to open filter stats tree");
        let settings_db = db.open_tree("filter_settings").expect("Failed to open filter settings tree");
        
        let account_seed = std::env::var("ACCOUNT_SEED")
            .expect("ACCOUNT_SEED environment variable not found");
        
        Self {
            filters_db,
            metadata_db,
            stats_db,
            settings_db,
            account_seed,
        }
    }

    fn format_key(&self, group_id: &str, suffix: &str) -> String {
        format!("{}-{}:{}", group_id, self.account_seed, suffix)
    }

    fn format_prefix(&self, group_id: &str) -> String {
        format!("{}-{}:", group_id, self.account_seed)
    }

    pub fn create_filter(&self, filter: FilterDefinition) -> Result<(), FilterError> {
        let validation = self.validate_filter(&filter)?;
        if !validation.is_valid {
            return Err(FilterError::ValidationFailed(validation));
        }

        if self.filter_exists(&filter.group_id, &filter.trigger)? {
            return Err(FilterError::DuplicateFilter(format!(
                "Filter with trigger '{}' already exists",
                filter.trigger
            )));
        }

        let key = self.format_key(&filter.group_id, &filter.id);
        let filter_bytes = serde_json::to_vec(&filter)
            .map_err(|e| FilterError::InternalError(e.to_string()))?;

        self.filters_db
            .insert(&key, filter_bytes)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        let metadata = self.create_metadata(&filter);
        let metadata_key = self.format_key(&filter.group_id, &filter.id);
        let metadata_bytes = serde_json::to_vec(&metadata)
            .map_err(|e| FilterError::InternalError(e.to_string()))?;

        self.metadata_db
            .insert(&metadata_key, metadata_bytes)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        let stats = FilterStats {
            group_id: filter.group_id.clone(),
            filter_id: filter.id.clone(),
            usage_count: 0,
            last_triggered: None,
            last_triggered_by: None,
        };
        let stats_key = self.format_key(&filter.group_id, &filter.id);
        let stats_bytes = serde_json::to_vec(&stats)
            .map_err(|e| FilterError::InternalError(e.to_string()))?;

        self.stats_db
            .insert(&stats_key, stats_bytes)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn get_group_filters(&self, group_id: &str) -> Result<Vec<FilterDefinition>, FilterError> {
        let mut filters = Vec::new();
        let prefix = self.format_prefix(group_id);

        for result in self.filters_db.scan_prefix(&prefix) {
            let (_, value) = result.map_err(|e| FilterError::DatabaseError(e.to_string()))?;
            let filter: FilterDefinition = serde_json::from_slice(&value)
                .map_err(|e| FilterError::InternalError(e.to_string()))?;

            if filter.is_active {
                filters.push(filter);
            }
        }

        Ok(filters)
    }

    pub fn remove_filter(&self, group_id: &str, filter_id: &str) -> Result<(), FilterError> {
        let key = self.format_key(group_id, filter_id);

        if !self.filters_db.contains_key(&key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?
        {
            return Err(FilterError::NotFound(format!(
                "Filter with ID '{}' not found",
                filter_id
            )));
        }

        self.filters_db
            .remove(&key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        self.metadata_db
            .remove(&key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        self.stats_db
            .remove(&key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn reset_group_filters(&self, group_id: &str) -> Result<u32, FilterError> {
        let prefix = self.format_prefix(group_id);
        let mut removed_count = 0;

        let keys_to_remove: Vec<_> = self
            .filters_db
            .scan_prefix(&prefix)
            .map(|result| {
                result
                    .map(|(key, _)| key)
                    .map_err(|e| FilterError::DatabaseError(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for key in keys_to_remove {
            self.filters_db
                .remove(&key)
                .map_err(|e| FilterError::DatabaseError(e.to_string()))?;
            self.metadata_db
                .remove(&key)
                .map_err(|e| FilterError::DatabaseError(e.to_string()))?;
            self.stats_db
                .remove(&key)
                .map_err(|e| FilterError::DatabaseError(e.to_string()))?;
            removed_count += 1;
        }

        Ok(removed_count)
    }

    pub fn find_matching_filters(
        &self,
        group_id: &str,
        text: &str,
    ) -> Result<Vec<FilterMatch>, FilterError> {
        let filters = self.get_group_filters(group_id)?;
        let mut matches = Vec::new();
        let text_lower = text.to_lowercase();

        for filter in filters {
            if let Some(filter_match) = self.check_filter_match(&filter, &text_lower) {
                matches.push(filter_match);
            }
        }

        Ok(matches)
    }

    pub fn put_pending_settings(&self, key: String, state: &PendingFilterWizardState) -> Result<(), FilterError> {
        let state_bytes = serde_json::to_vec(state)
            .map_err(|e| FilterError::InternalError(e.to_string()))?;
        
        self.settings_db
            .insert(&key, state_bytes)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }

    pub fn get_pending_settings(&self, key: &str) -> Option<PendingFilterWizardState> {
        self.settings_db
            .get(key)
            .ok()
            .flatten()
            .and_then(|data| serde_json::from_slice(&data).ok())
    }

    pub fn remove_pending_settings(&self, key: &str) -> Result<(), FilterError> {
        self.settings_db
            .remove(key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn record_filter_usage(
        &self,
        group_id: &str,
        filter_id: &str,
        user_id: i64,
    ) -> Result<(), FilterError> {
        let key = self.format_key(group_id, filter_id);
        let stats_data = self
            .stats_db
            .get(&key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        let mut stats = if let Some(data) = stats_data {
            serde_json::from_slice::<FilterStats>(&data)
                .map_err(|e| FilterError::InternalError(e.to_string()))?
        } else {
            FilterStats {
                group_id: group_id.to_string(),
                filter_id: filter_id.to_string(),
                usage_count: 0,
                last_triggered: None,
                last_triggered_by: None,
            }
        };

        stats.usage_count += 1;
        stats.last_triggered = Some(chrono::Utc::now().timestamp());
        stats.last_triggered_by = Some(user_id);

        let stats_bytes = serde_json::to_vec(&stats)
            .map_err(|e| FilterError::InternalError(e.to_string()))?;

        self.stats_db
            .insert(&key, stats_bytes)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn get_filter_stats(&self, group_id: &str, filter_id: &str) -> Result<FilterStats, FilterError> {
        let key = self.format_key(group_id, filter_id);
        let stats_data = self
            .stats_db
            .get(&key)
            .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

        if let Some(data) = stats_data {
            let stats = serde_json::from_slice::<FilterStats>(&data)
                .map_err(|e| FilterError::InternalError(e.to_string()))?;
            Ok(stats)
        } else {
            Err(FilterError::NotFound(format!(
                "Stats for filter '{}' not found",
                filter_id
            )))
        }
    }

    fn validate_filter(&self, filter: &FilterDefinition) -> Result<ValidationResult, FilterError> {
        let mut result = ValidationResult::success();

        if filter.trigger.trim().is_empty() {
            result.errors.push("Trigger cannot be empty".to_string());
            result.is_valid = false;
        }

        if filter.trigger.len() > 100 {
            result.errors.push("Trigger too long (max 100 characters)".to_string());
            result.is_valid = false;
        }

        if filter.response.trim().is_empty() {
            result.errors.push("Response cannot be empty".to_string());
            result.is_valid = false;
        }

        if filter.response.len() > 2000 {
            result.errors.push("Response too long (max 2000 characters)".to_string());
            result.is_valid = false;
        }

        let forbidden_patterns = vec!["admin", "bot", "/"];
        for pattern in forbidden_patterns {
            if filter.trigger.to_lowercase().contains(pattern) {
                result.errors.push(format!(
                    "Trigger cannot contain forbidden pattern: {}",
                    pattern
                ));
                result.is_valid = false;
            }
        }

        Ok(result)
    }

    fn filter_exists(&self, group_id: &str, trigger: &str) -> Result<bool, FilterError> {
        let filters = self.get_group_filters(group_id)?;
        let trigger_lower = trigger.to_lowercase();

        for filter in filters {
            if filter.trigger.to_lowercase() == trigger_lower {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn create_metadata(&self, filter: &FilterDefinition) -> FilterMetadata {
        FilterMetadata {
            group_id: filter.group_id.clone(),
            trigger_hash: self.calculate_trigger_hash(&filter.trigger),
            display_name: self.truncate_string(&filter.trigger, 30),
            response_preview: self.truncate_string(&filter.response.replace('\n', " "), 50),
            last_modified: filter.created_at,
            modified_by: filter.created_by,
            filter_id: filter.id.clone(),
        }
    }

    fn calculate_trigger_hash(&self, trigger: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        trigger.to_lowercase().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn truncate_string(&self, s: &str, max_len: usize) -> String {
        if s.len() > max_len {
            format!("{}...", &s[..max_len - 3])
        } else {
            s.to_string()
        }
    }

    fn check_filter_match(&self, filter: &FilterDefinition, text: &str) -> Option<FilterMatch> {
        let trigger_lower = filter.trigger.to_lowercase();

        let (matched, position) = match filter.match_type {
            MatchType::Exact => {
                let words: Vec<&str> = text.split_whitespace().collect();
                for (i, word) in words.iter().enumerate() {
                    if word.trim_matches(|c: char| !c.is_alphanumeric()) == trigger_lower {
                        return Some(FilterMatch {
                            filter: filter.clone(),
                            matched_text: word.to_string(),
                            match_position: i,
                        });
                    }
                }
                (false, 0)
            }
            MatchType::Contains => {
                if let Some(pos) = text.find(&trigger_lower) {
                    (true, pos)
                } else {
                    (false, 0)
                }
            }
            MatchType::StartsWith => {
                if text.starts_with(&trigger_lower) {
                    (true, 0)
                } else {
                    (false, 0)
                }
            }
            MatchType::EndsWith => {
                if text.ends_with(&trigger_lower) {
                    (true, text.len() - trigger_lower.len())
                } else {
                    (false, 0)
                }
            }
        };

        if matched {
            Some(FilterMatch {
                filter: filter.clone(),
                matched_text: trigger_lower,
                match_position: position,
            })
        } else {
            None
        }
    }
}

