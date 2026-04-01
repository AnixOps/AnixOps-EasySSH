//! A/B testing and feature flag support for gradual rollouts

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A/B test group assignment
#[derive(Debug, Clone)]
pub struct AbTestManager {
    /// Current user's test group
    group: Option<String>,
    /// Cached feature flags from server
    feature_flags: Arc<RwLock<HashMap<String, bool>>>,
    /// Rollout percentages
    rollouts: Arc<RwLock<HashMap<String, u8>>>,
    /// Install ID for consistent assignment
    install_id: String,
}

impl AbTestManager {
    pub async fn new(group: Option<String>) -> anyhow::Result<Self> {
        let install_id = super::generate_install_id();

        Ok(Self {
            group,
            feature_flags: Arc::new(RwLock::new(HashMap::new())),
            rollouts: Arc::new(RwLock::new(HashMap::new())),
            install_id,
        })
    }

    /// Check if user is in a specific test group
    pub fn is_in_group(&self, group: &str) -> bool {
        self.group.as_ref().map(|g| g == group).unwrap_or(false)
    }

    /// Check if feature is enabled via feature flag
    pub async fn is_feature_enabled(&self, feature: &str) -> bool {
        let flags = self.feature_flags.read().await;
        flags.get(feature).copied().unwrap_or(false)
    }

    /// Set feature flag (from server response)
    pub async fn set_feature_flag(&self, feature: &str, enabled: bool) {
        let mut flags = self.feature_flags.write().await;
        flags.insert(feature.to_string(), enabled);
    }

    /// Check if user is in rollout for a version
    pub fn is_in_rollout(&self, version: &str, percentage: u8) -> bool {
        // Deterministic assignment based on install_id + version
        let hash_input = format!("{}:{}", self.install_id, version);
        let hash = self.hash_string(&hash_input);

        // Convert hash to 0-99 range
        let bucket = (hash % 100) as u8;

        bucket < percentage
    }

    /// Set rollout percentage
    pub async fn set_rollout(&self, version: &str, percentage: u8) {
        let mut rollouts = self.rollouts.write().await;
        rollouts.insert(version.to_string(), percentage);
    }

    fn hash_string(&self, input: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        hasher.finish()
    }

    /// Get install ID
    pub fn get_install_id(&self) -> &str {
        &self.install_id
    }

    /// Get test group
    pub fn get_group(&self) -> Option<&String> {
        self.group.as_ref()
    }

    /// Update from server response
    pub async fn update_from_server(&self, features: Vec<String>, rollouts: HashMap<String, u8>) {
        // Update feature flags
        let mut flags = self.feature_flags.write().await;
        flags.clear();
        for feature in features {
            flags.insert(feature, true);
        }
        drop(flags);

        // Update rollouts
        let mut roll = self.rollouts.write().await;
        *roll = rollouts;
    }
}

/// Feature flag configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlags {
    pub version: String,
    pub features: Vec<FeatureConfig>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeatureConfig {
    pub name: String,
    pub enabled: bool,
    pub rollout_percentage: Option<u8>,
    pub target_groups: Vec<String>,
    pub requires_version: Option<String>,
}

/// Experiment configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Experiment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub control_group: String,
    pub test_groups: Vec<String>,
    pub metrics: Vec<String>,
}

/// Experiment tracking
pub struct ExperimentTracker {
    active_experiments: Arc<RwLock<Vec<Experiment>>>,
    assignments: Arc<RwLock<HashMap<String, String>>>, // experiment_id -> group
}

impl ExperimentTracker {
    pub fn new() -> Self {
        Self {
            active_experiments: Arc::new(RwLock::new(Vec::new())),
            assignments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register experiment participation
    pub async fn track_exposure(
        &self,
        experiment_id: &str,
        group: &str,
    ) -> anyhow::Result<()> {
        let mut assignments = self.assignments.write().await;
        assignments.insert(experiment_id.to_string(), group.to_string());
        Ok(())
    }

    /// Track conversion for experiment
    pub async fn track_conversion(
        &self,
        experiment_id: &str,
        metric: &str,
        value: f64,
    ) -> anyhow::Result<()> {
        // Send to analytics server
        // This would typically be async to an analytics backend
        log::info!(
            "Experiment conversion: {} - {} = {}",
            experiment_id,
            metric,
            value
        );
        Ok(())
    }

    /// Get assigned group for experiment
    pub async fn get_assignment(&self, experiment_id: &str) -> Option<String> {
        let assignments = self.assignments.read().await;
        assignments.get(experiment_id).cloned()
    }
}

/// Canary deployment support
pub struct CanaryDeployment {
    pub version: String,
    pub percentage: u8,
    pub regions: Vec<String>,
    pub user_groups: Vec<String>,
}

impl CanaryDeployment {
    /// Check if this deployment applies to current user
    pub fn applies_to(
        &self,
        region: &str,
        user_group: Option<&str>,
        install_id: &str,
    ) -> bool {
        // Check region
        if !self.regions.is_empty() && !self.regions.contains(&region.to_string()) {
            return false;
        }

        // Check user group
        if let Some(group) = user_group {
            if !self.user_groups.is_empty() && !self.user_groups.contains(&group.to_string()) {
                return false;
            }
        }

        // Percentage rollout
        let hash_input = format!("{}:{}", install_id, self.version);
        let hash = hash_string(&hash_input);
        let bucket = (hash % 100) as u8;

        bucket < self.percentage
    }
}

fn hash_string(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Gradual rollout strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloutStrategy {
    /// Percentage-based
    Percentage,
    /// Time-based (ramp up over time)
    TimeBased,
    /// User segment-based
    SegmentBased,
}

pub struct RolloutSchedule {
    pub strategy: RolloutStrategy,
    pub start_percentage: u8,
    pub target_percentage: u8,
    pub start_time: u64,
    pub duration_seconds: u64,
}

impl RolloutSchedule {
    /// Calculate current rollout percentage based on schedule
    pub fn current_percentage(&self) -> u8 {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now < self.start_time {
            return self.start_percentage;
        }

        let elapsed = now - self.start_time;
        if elapsed >= self.duration_seconds {
            return self.target_percentage;
        }

        let progress = elapsed as f64 / self.duration_seconds as f64;
        let percentage_range = self.target_percentage - self.start_percentage;

        self.start_percentage + (percentage_range as f64 * progress) as u8
    }
}

use std::time::SystemTime;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollout_assignment() {
        let manager = AbTestManager {
            group: None,
            feature_flags: Arc::new(RwLock::new(HashMap::new())),
            rollouts: Arc::new(RwLock::new(HashMap::new())),
            install_id: "test-install-123".to_string(),
        };

        // Same install_id + version should always get same result
        let result1 = manager.is_in_rollout("1.0.0", 50);
        let result2 = manager.is_in_rollout("1.0.0", 50);
        assert_eq!(result1, result2);

        // Different version should potentially differ
        let _result3 = manager.is_in_rollout("1.0.1", 50);
    }

    #[test]
    fn test_rollout_distribution() {
        let manager = AbTestManager {
            group: None,
            feature_flags: Arc::new(RwLock::new(HashMap::new())),
            rollouts: Arc::new(RwLock::new(HashMap::new())),
            install_id: "test".to_string(),
        };

        let mut in_rollout = 0;
        let total = 1000;

        for i in 0..total {
            let id = format!("install-{}", i);
            let hash = hash_string(&format!("{}:1.0.0", id));
            let bucket = (hash % 100) as u8;
            if bucket < 50 {
                in_rollout += 1;
            }
        }

        // Should be roughly 50% (with some variance due to hash distribution)
        let percentage = (in_rollout as f64 / total as f64) * 100.0;
        assert!(percentage > 45.0 && percentage < 55.0);
    }

    #[tokio::test]
    async fn test_feature_flags() {
        let manager = AbTestManager::new(None).await.unwrap();

        assert!(!manager.is_feature_enabled("test-feature").await);

        manager.set_feature_flag("test-feature", true).await;
        assert!(manager.is_feature_enabled("test-feature").await);
    }

    #[test]
    fn test_rollout_schedule() {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let schedule = RolloutSchedule {
            strategy: RolloutStrategy::TimeBased,
            start_percentage: 0,
            target_percentage: 100,
            start_time: now - 3600, // Started 1 hour ago
            duration_seconds: 7200, // 2 hour duration
        };

        // Should be at ~50% rollout
        let current = schedule.current_percentage();
        assert!(current >= 45 && current <= 55);
    }
}
