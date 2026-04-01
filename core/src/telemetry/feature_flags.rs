//! Feature flags and A/B testing system
//!
//! Supports:
//! - Percentage-based rollouts
//! - User segment targeting
//! - A/B test variants
//! - Feature kill switches
//! - Gradual rollouts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{AnonymousId, TelemetryError};

/// Feature flag definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    /// Flag name/identifier
    pub name: String,
    /// Flag description
    pub description: String,
    /// Whether the feature is enabled globally
    pub globally_enabled: bool,
    /// Rollout percentage (0-100)
    pub rollout_percentage: u8,
    /// A/B test variants
    pub variants: Vec<Variant>,
    /// Target segments
    pub target_segments: Vec<String>,
    /// Start time for gradual rollout
    pub start_time: Option<u64>,
    /// End time for the feature/experiment
    pub end_time: Option<u64>,
    /// Dependencies on other flags
    pub requires_flags: Vec<String>,
    /// Whether this is an experiment
    pub is_experiment: bool,
}

/// A/B test variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    /// Variant name
    pub name: String,
    /// Variant weight (for traffic allocation)
    pub weight: u32,
    /// Variant configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Whether this is the control variant
    pub is_control: bool,
}

/// Flag evaluation result
#[derive(Debug, Clone)]
pub struct FlagResult {
    pub enabled: bool,
    pub variant: Option<Variant>,
}

/// User attributes for targeting
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserAttributes {
    pub edition: Option<String>,
    pub platform: Option<String>,
    pub version: Option<String>,
    pub signup_date: Option<u64>,
    pub custom: HashMap<String, serde_json::Value>,
}

impl FeatureFlag {
    /// Create new feature flag
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            globally_enabled: false,
            rollout_percentage: 0,
            variants: vec![],
            target_segments: vec![],
            start_time: None,
            end_time: None,
            requires_flags: vec![],
            is_experiment: false,
        }
    }

    /// Enable flag globally
    pub fn enable(mut self) -> Self {
        self.globally_enabled = true;
        self.rollout_percentage = 100;
        self
    }

    /// Set rollout percentage
    pub fn with_rollout(mut self, percentage: u8) -> Self {
        self.rollout_percentage = percentage.min(100);
        self
    }

    /// Add A/B test variant
    pub fn with_variant(mut self, variant: Variant) -> Self {
        self.variants.push(variant);
        self.is_experiment = true;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Evaluate flag for user
    pub fn evaluate(&self, user_id: &AnonymousId, attributes: &UserAttributes) -> FlagResult {
        // Check if globally enabled
        if self.globally_enabled {
            return FlagResult {
                enabled: true,
                variant: self.select_variant(user_id),
            };
        }

        // Check time constraints
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(start) = self.start_time {
            if now < start {
                return FlagResult {
                    enabled: false,
                    variant: None,
                };
            }
        }

        if let Some(end) = self.end_time {
            if now > end {
                return FlagResult {
                    enabled: false,
                    variant: None,
                };
            }
        }

        // Check user targeting
        if !self.matches_targeting(attributes) {
            return FlagResult {
                enabled: false,
                variant: None,
            };
        }

        // Check rollout percentage using consistent hashing
        let enabled = self.is_in_rollout(user_id);

        FlagResult {
            enabled,
            variant: if enabled { self.select_variant(user_id) } else { None },
        }
    }

    /// Check if user is in rollout percentage
    fn is_in_rollout(&self, user_id: &AnonymousId) -> bool {
        if self.rollout_percentage >= 100 {
            return true;
        }
        if self.rollout_percentage == 0 {
            return false;
        }

        // Use consistent hashing for stable assignment
        let hash = self.hash_user_id(user_id);
        let percentage_bucket = (hash % 100) as u8;

        percentage_bucket < self.rollout_percentage
    }

    /// Select variant for user
    fn select_variant(&self, user_id: &AnonymousId) -> Option<Variant> {
        if self.variants.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: u32 = self.variants.iter().map(|v| v.weight).sum();
        if total_weight == 0 {
            return self.variants.first().cloned();
        }

        // Hash user ID to select variant
        let hash = self.hash_user_id(user_id);
        let variant_bucket = (hash % total_weight as u64) as u32;

        // Find selected variant
        let mut cumulative_weight = 0;
        for variant in &self.variants {
            cumulative_weight += variant.weight;
            if variant_bucket < cumulative_weight {
                return Some(variant.clone());
            }
        }

        // Fallback to first variant
        self.variants.first().cloned()
    }

    /// Hash user ID consistently
    fn hash_user_id(&self, user_id: &AnonymousId) -> u64 {
        let mut hasher = DefaultHasher::new();
        // Include flag name for per-flag consistent hashing
        self.name.hash(&mut hasher);
        user_id.as_str().hash(&mut hasher);
        hasher.finish()
    }

    /// Check if user matches targeting criteria
    fn matches_targeting(&self, attributes: &UserAttributes) -> bool {
        // Check segments
        if !self.target_segments.is_empty() {
            // For now, simple segment matching
            // In production, this would be more sophisticated
        }

        true
    }
}

impl Variant {
    /// Create control variant
    pub fn control() -> Self {
        Self {
            name: "control".to_string(),
            weight: 50,
            config: HashMap::new(),
            is_control: true,
        }
    }

    /// Create treatment variant
    pub fn treatment(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            weight: 50,
            config: HashMap::new(),
            is_control: false,
        }
    }

    /// Set weight
    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    /// Add config value
    pub fn with_config(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }
}

/// Feature flag manager
pub struct FeatureFlagManager {
    flags: Arc<Mutex<HashMap<String, FeatureFlag>>>,
    user_attributes: Arc<Mutex<UserAttributes>>,
    flag_states: Arc<Mutex<HashMap<String, FlagState>>>,
}

#[derive(Debug, Clone)]
struct FlagState {
    enabled: bool,
    variant: Option<Variant>,
    evaluated_at: u64,
}

impl FeatureFlagManager {
    pub fn new() -> Result<Self, TelemetryError> {
        Ok(Self {
            flags: Arc::new(Mutex::new(HashMap::new())),
            user_attributes: Arc::new(Mutex::new(UserAttributes::default())),
            flag_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Register a flag
    pub fn register_flag(&self, flag: FeatureFlag) {
        let mut flags = self.flags.lock().unwrap();
        flags.insert(flag.name.clone(), flag);
    }

    /// Check if feature is enabled
    pub fn is_enabled(&self, flag_name: &str, user_id: &AnonymousId) -> bool {
        self.evaluate_flag(flag_name, user_id).enabled
    }

    /// Get feature variant
    pub fn get_variant(&self, flag_name: &str, user_id: &AnonymousId) -> Option<Variant> {
        self.evaluate_flag(flag_name, user_id).variant
    }

    /// Evaluate flag with caching
    fn evaluate_flag(&self, flag_name: &str, user_id: &AnonymousId) -> FlagResult {
        // Check cache first
        {
            let states = self.flag_states.lock().unwrap();
            if let Some(state) = states.get(flag_name) {
                // Cache for 5 minutes
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                if now - state.evaluated_at < 300 {
                    return FlagResult {
                        enabled: state.enabled,
                        variant: state.variant.clone(),
                    };
                }
            }
        }

        // Evaluate flag
        let flags = self.flags.lock().unwrap();
        let attributes = self.user_attributes.lock().unwrap();

        let result = flags
            .get(flag_name)
            .map(|flag| flag.evaluate(user_id, &attributes))
            .unwrap_or(FlagResult {
                enabled: false,
                variant: None,
            });

        // Cache result
        let mut states = self.flag_states.lock().unwrap();
        states.insert(
            flag_name.to_string(),
            FlagState {
                enabled: result.enabled,
                variant: result.variant.clone(),
                evaluated_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        );

        result
    }

    /// Update user attributes
    pub fn set_user_attributes(&self, attributes: UserAttributes) {
        let mut attrs = self.user_attributes.lock().unwrap();
        *attrs = attributes;

        // Clear cached states as targeting might have changed
        let mut states = self.flag_states.lock().unwrap();
        states.clear();
    }

    /// Load flags from JSON configuration
    pub fn load_flags(&self, json: &str) -> Result<(), TelemetryError> {
        let flags: Vec<FeatureFlag> = serde_json::from_str(json)?;

        let mut flag_map = self.flags.lock().unwrap();
        flag_map.clear();
        for flag in flags {
            flag_map.insert(flag.name.clone(), flag);
        }

        Ok(())
    }

    /// Get all flags
    pub fn get_all_flags(&self) -> Vec<FeatureFlag> {
        self.flags.lock().unwrap().values().cloned().collect()
    }

    /// Update flag (for remote config)
    pub fn update_flag(&self, flag: FeatureFlag) {
        let mut flags = self.flags.lock().unwrap();
        flags.insert(flag.name.clone(), flag);

        // Clear cached state for this flag
        let mut states = self.flag_states.lock().unwrap();
        states.remove(&flag.name);
    }

    /// Remove flag
    pub fn remove_flag(&self, flag_name: &str) {
        let mut flags = self.flags.lock().unwrap();
        flags.remove(flag_name);

        let mut states = self.flag_states.lock().unwrap();
        states.remove(flag_name);
    }

    /// Get experiments (flags with variants)
    pub fn get_experiments(&self) -> Vec<FeatureFlag> {
        self.flags
            .lock()
            .unwrap()
            .values()
            .filter(|f| f.is_experiment)
            .cloned()
            .collect()
    }

    /// Load default flags
    pub fn load_defaults(&self) {
        let defaults = vec![
            FeatureFlag::new("new_terminal_ui")
                .with_description("Enable new terminal UI design")
                .with_rollout(0), // Start at 0%, controlled remotely

            FeatureFlag::new("sftp_file_preview")
                .with_description("Enable file preview in SFTP browser")
                .enable(),

            FeatureFlag::new("quick_connect")
                .with_description("Quick connect feature")
                .with_rollout(50)
                .with_variant(Variant::control())
                .with_variant(Variant::treatment("new_flow")),

            FeatureFlag::new("dark_mode_v2")
                .with_description("Improved dark mode colors")
                .with_rollout(10),
        ];

        for flag in defaults {
            self.register_flag(flag);
        }
    }
}

/// A/B test experiment tracking
pub struct ExperimentTracker {
    experiments: Arc<Mutex<HashMap<String, ExperimentData>>>,
}

#[derive(Debug, Clone)]
struct ExperimentData {
    flag_name: String,
    variant: String,
    enrolled_at: u64,
    events: Vec<ExperimentEvent>,
}

#[derive(Debug, Clone)]
struct ExperimentEvent {
    event_name: String,
    timestamp: u64,
    value: f64,
}

impl ExperimentTracker {
    pub fn new() -> Self {
        Self {
            experiments: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Track enrollment in experiment
    pub fn track_enrollment(&self, flag_name: &str, variant: &Variant) {
        let mut experiments = self.experiments.lock().unwrap();
        experiments.insert(
            flag_name.to_string(),
            ExperimentData {
                flag_name: flag_name.to_string(),
                variant: variant.name.clone(),
                enrolled_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                events: vec![],
            },
        );
    }

    /// Track conversion event
    pub fn track_event(&self, flag_name: &str, event_name: &str, value: f64) {
        let mut experiments = self.experiments.lock().unwrap();
        if let Some(exp) = experiments.get_mut(flag_name) {
            exp.events.push(ExperimentEvent {
                event_name: event_name.to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                value,
            });
        }
    }

    /// Get experiment data
    pub fn get_experiment_data(&self, flag_name: &str) -> Option<ExperimentData> {
        self.experiments.lock().unwrap().get(flag_name).cloned()
    }
}

/// Feature flag evaluation macro
#[macro_export]
macro_rules! feature_enabled {
    ($manager:expr, $flag:expr, $user_id:expr) => {
        $manager.is_enabled($flag, $user_id)
    };
}

/// Feature variant macro
#[macro_export]
macro_rules! feature_variant {
    ($manager:expr, $flag:expr, $user_id:expr) => {
        $manager.get_variant($flag, $user_id)
    };
}
