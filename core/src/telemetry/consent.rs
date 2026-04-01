//! Privacy-first consent management
//!
//! GDPR/CCPA compliant consent system with:
//! - Explicit opt-in required
//! - Granular consent categories
//! - Easy opt-out mechanism
//! - Data export and deletion
//! - Regional compliance detection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{TelemetryError, TelemetryEvent};

/// Consent categories for granular control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentCategory {
    /// Essential analytics (anonymized usage counts)
    Essential,
    /// Feature usage analytics
    Analytics,
    /// Performance metrics
    Performance,
    /// Crash reporting and error tracking
    CrashReporting,
    /// A/B testing and feature flags
    Experimentation,
    /// User feedback collection
    Feedback,
}

impl ConsentCategory {
    /// Get all categories
    pub fn all() -> Vec<ConsentCategory> {
        vec![
            ConsentCategory::Essential,
            ConsentCategory::Analytics,
            ConsentCategory::Performance,
            ConsentCategory::CrashReporting,
            ConsentCategory::Experimentation,
            ConsentCategory::Feedback,
        ]
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            ConsentCategory::Essential => "Essential Diagnostics",
            ConsentCategory::Analytics => "Usage Analytics",
            ConsentCategory::Performance => "Performance Metrics",
            ConsentCategory::CrashReporting => "Crash Reporting",
            ConsentCategory::Experimentation => "Feature Testing",
            ConsentCategory::Feedback => "User Feedback",
        }
    }

    /// Get description for UI
    pub fn description(&self) -> &'static str {
        match self {
            ConsentCategory::Essential => {
                "Anonymous app health metrics (startup time, session count). No personal data."
            }
            ConsentCategory::Analytics => {
                "Feature usage patterns to improve the app. No SSH credentials or server info."
            }
            ConsentCategory::Performance => {
                "Performance data to optimize speed and responsiveness."
            }
            ConsentCategory::CrashReporting => {
                "Error reports to fix bugs and improve stability. Stack traces without user data."
            }
            ConsentCategory::Experimentation => {
                "Participate in A/B tests to try new features and improvements."
            }
            ConsentCategory::Feedback => {
                "Collect your feedback to improve the product experience."
            }
        }
    }

    /// Check if this category is essential (always allowed, anonymized)
    pub fn is_essential(&self) -> bool {
        matches!(self, ConsentCategory::Essential)
    }
}

/// Regional privacy regulations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyRegion {
    /// European Union (GDPR)
    Eu,
    /// California (CCPA)
    California,
    /// Other regions with strict privacy laws
    Brazil,    // LGPD
    Canada,    // PIPEDA
    Australia, // Privacy Act
    /// Default/Other
    Other,
}

impl PrivacyRegion {
    /// Detect region from system locale
    pub fn detect() -> Self {
        let locale = Self::get_system_locale();
        let locale_lower = locale.to_lowercase();

        // EU countries
        let eu_countries = [
            "at", "be", "bg", "hr", "cy", "cz", "dk", "ee", "fi", "fr",
            "de", "gr", "hu", "ie", "it", "lv", "lt", "lu", "mt", "nl",
            "pl", "pt", "ro", "sk", "si", "es", "se", // EU-27
            "gb", "uk", // UK (still GDPR-covered)
            "no", "is", "li", "ch", // EEA
        ];

        // Check for EU
        for country in &eu_countries {
            if locale_lower.contains(country) {
                return PrivacyRegion::Eu;
            }
        }

        // Check for specific regions
        if locale_lower.contains("us-ca") || locale_lower.contains("california") {
            return PrivacyRegion::California;
        }

        if locale_lower.contains("br") || locale_lower.contains("brazil") {
            return PrivacyRegion::Brazil;
        }

        if locale_lower.contains("ca") || locale_lower.contains("canada") {
            return PrivacyRegion::Canada;
        }

        if locale_lower.contains("au") || locale_lower.contains("australia") {
            return PrivacyRegion::Australia;
        }

        PrivacyRegion::Other
    }

    fn get_system_locale() -> String {
        // Try to get system locale
        std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_CTYPE"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_else(|_| "en_US.UTF-8".to_string())
    }

    /// Get required consent mode for this region
    pub fn consent_mode(&self) -> ConsentMode {
        match self {
            PrivacyRegion::Eu | PrivacyRegion::California => ConsentMode::ExplicitOptIn,
            _ => ConsentMode::OptOut,
        }
    }

    /// Get specific requirements for this region
    pub fn requirements(&self) -> RegionRequirements {
        match self {
            PrivacyRegion::Eu => RegionRequirements {
                requires_explicit_consent: true,
                requires_data_processing_agreement: true,
                requires_data_protection_officer: false,
                requires_breach_notification: true,
                data_retention_limit_days: None, // As short as necessary
                requires_right_to_portability: true,
                requires_right_to_erasure: true,
            },
            PrivacyRegion::California => RegionRequirements {
                requires_explicit_consent: true,
                requires_data_processing_agreement: false,
                requires_data_protection_officer: false,
                requires_breach_notification: true,
                data_retention_limit_days: Some(365),
                requires_right_to_portability: true,
                requires_right_to_erasure: true,
            },
            _ => RegionRequirements {
                requires_explicit_consent: false,
                requires_data_processing_agreement: false,
                requires_data_protection_officer: false,
                requires_breach_notification: false,
                data_retention_limit_days: None,
                requires_right_to_portability: false,
                requires_right_to_erasure: false,
            },
        }
    }
}

/// Consent mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentMode {
    /// User must explicitly opt in (GDPR, CCPA)
    ExplicitOptIn,
    /// User can opt out (other regions)
    OptOut,
}

/// Regional compliance requirements
#[derive(Debug, Clone)]
pub struct RegionRequirements {
    pub requires_explicit_consent: bool,
    pub requires_data_processing_agreement: bool,
    pub requires_data_protection_officer: bool,
    pub requires_breach_notification: bool,
    pub data_retention_limit_days: Option<u32>,
    pub requires_right_to_portability: bool,
    pub requires_right_to_erasure: bool,
}

/// Consent status for each category
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsentStatus {
    /// User has made a choice (not pending)
    pub has_responded: bool,
    /// Timestamp of consent decision
    pub timestamp: Option<u64>,
    /// Consent per category
    pub categories: HashMap<ConsentCategory, bool>,
    /// Region detected
    pub region: Option<PrivacyRegion>,
    /// Consent version (for invalidation on updates)
    pub version: u32,
}

impl ConsentStatus {
    /// Create new consent status
    pub fn new(region: PrivacyRegion) -> Self {
        let mut categories = HashMap::new();

        // Essential is always true
        categories.insert(ConsentCategory::Essential, true);

        // Others default based on region
        let default_value = match region.consent_mode() {
            ConsentMode::ExplicitOptIn => false,
            ConsentMode::OptOut => true,
        };

        for cat in ConsentCategory::all() {
            if !cat.is_essential() {
                categories.insert(cat, default_value);
            }
        }

        Self {
            has_responded: false,
            timestamp: None,
            categories,
            region: Some(region),
            version: 1,
        }
    }

    /// Check if category is allowed
    pub fn is_allowed(&self, category: ConsentCategory) -> bool {
        if category.is_essential() {
            return true;
        }
        self.categories.get(&category).copied().unwrap_or(false)
    }

    /// Set consent for category
    pub fn set_consent(&mut self, category: ConsentCategory, allowed: bool) {
        if !category.is_essential() {
            self.categories.insert(category, allowed);
        }
    }

    /// Accept all categories
    pub fn accept_all(&mut self) {
        for cat in ConsentCategory::all() {
            if !cat.is_essential() {
                self.categories.insert(cat, true);
            }
        }
        self.mark_responded();
    }

    /// Reject all non-essential categories
    pub fn reject_all(&mut self) {
        for cat in ConsentCategory::all() {
            if !cat.is_essential() {
                self.categories.insert(cat, false);
            }
        }
        self.mark_responded();
    }

    /// Mark that user has responded
    fn mark_responded(&mut self) {
        self.has_responded = true;
        self.timestamp = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Check if consent is needed (not responded and requires explicit consent)
    pub fn needs_consent(&self) -> bool {
        if self.has_responded {
            return false;
        }

        match self.region {
            Some(region) => matches!(region.consent_mode(), ConsentMode::ExplicitOptIn),
            None => false,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, TelemetryError> {
        serde_json::to_string(self).map_err(|e| TelemetryError::Serialization(e))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, TelemetryError> {
        serde_json::from_str(json).map_err(|e| TelemetryError::Serialization(e))
    }
}

/// Manages user consent for telemetry
pub struct ConsentManager {
    status: Arc<Mutex<ConsentStatus>>,
    storage_path: PathBuf,
}

impl ConsentManager {
    /// Create new consent manager
    pub fn new() -> Result<Self, TelemetryError> {
        let region = PrivacyRegion::detect();
        let status = ConsentStatus::new(region);

        let storage_path = Self::get_storage_path()?;

        Ok(Self {
            status: Arc::new(Mutex::new(status)),
            storage_path,
        })
    }

    /// Get storage path for consent file
    fn get_storage_path() -> Result<PathBuf, TelemetryError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| TelemetryError::Config("Cannot find config directory".to_string()))?
            .join("easyssh");

        std::fs::create_dir_all(&config_dir)?;

        Ok(config_dir.join("telemetry_consent.json"))
    }

    /// Load persisted consent
    pub async fn load(&self) -> Result<(), TelemetryError> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let json = tokio::fs::read_to_string(&self.storage_path).await?;
        let loaded = ConsentStatus::from_json(&json)?;

        let mut status = self.status.lock().unwrap();
        *status = loaded;

        Ok(())
    }

    /// Save consent to disk
    async fn save(&self) -> Result<(), TelemetryError> {
        let status = self.status.lock().unwrap().clone();
        let json = status.to_json()?;

        tokio::fs::write(&self.storage_path, json).await?;

        Ok(())
    }

    /// Get current consent status
    pub fn get_status(&self) -> ConsentStatus {
        self.status.lock().unwrap().clone()
    }

    /// Check if a specific category is allowed
    pub fn is_allowed(&self, category: ConsentCategory) -> bool {
        self.status.lock().unwrap().is_allowed(category)
    }

    /// Check if analytics can be collected for this event
    pub fn can_collect(&self, event: &TelemetryEvent) -> bool {
        let status = self.status.lock().unwrap();

        let category = match event {
            TelemetryEvent::AppStarted { .. } | TelemetryEvent::AppClosed { .. } => {
                ConsentCategory::Essential
            }
            TelemetryEvent::FeatureUsed { .. } => ConsentCategory::Analytics,
            TelemetryEvent::ScreenViewed { .. } => ConsentCategory::Analytics,
            TelemetryEvent::PerformanceMetric { .. } => ConsentCategory::Performance,
            TelemetryEvent::SshConnected { .. } | TelemetryEvent::SshDisconnected { .. } => {
                ConsentCategory::Analytics
            }
            TelemetryEvent::ErrorOccurred { .. } => ConsentCategory::CrashReporting,
            TelemetryEvent::FeedbackSubmitted { .. } => ConsentCategory::Feedback,
            TelemetryEvent::ConsentChanged { .. } => ConsentCategory::Essential,
        };

        status.is_allowed(category)
    }

    /// Update consent for a category
    pub async fn set_consent(
        &self,
        category: ConsentCategory,
        allowed: bool,
    ) -> Result<(), TelemetryError> {
        let mut status = self.status.lock().unwrap();
        status.set_consent(category, allowed);
        status.mark_responded();
        drop(status);

        self.save().await?;

        // Track consent change
        if category == ConsentCategory::Analytics || category == ConsentCategory::CrashReporting {
            // This would need the collector, handled at higher level
        }

        Ok(())
    }

    /// Accept all telemetry
    pub async fn accept_all(&self) -> Result<(), TelemetryError> {
        let mut status = self.status.lock().unwrap();
        status.accept_all();
        drop(status);

        self.save().await
    }

    /// Reject all non-essential telemetry
    pub async fn reject_all(&self) -> Result<(), TelemetryError> {
        let mut status = self.status.lock().unwrap();
        status.reject_all();
        drop(status);

        self.save().await
    }

    /// Check if consent dialog should be shown
    pub fn needs_consent_dialog(&self) -> bool {
        self.status.lock().unwrap().needs_consent()
    }

    /// Get detected privacy region
    pub fn get_region(&self) -> Option<PrivacyRegion> {
        self.status.lock().unwrap().region
    }

    /// Reset consent (for testing)
    pub async fn reset(&self) -> Result<(), TelemetryError> {
        let region = PrivacyRegion::detect();
        let mut status = self.status.lock().unwrap();
        *status = ConsentStatus::new(region);
        drop(status);

        self.save().await
    }

    /// Get privacy policy URL based on region
    pub fn get_privacy_policy_url(&self) -> &'static str {
        "https://easyssh.io/privacy"
    }

    /// Get data controller info for GDPR
    pub fn get_data_controller_info(&self) -> Option<DataControllerInfo> {
        match self.get_region() {
            Some(PrivacyRegion::Eu) => Some(DataControllerInfo {
                name: "EasySSH Team",
                email: "privacy@easyssh.io",
                address: "Contact via email",
            }),
            _ => None,
        }
    }
}

/// Data controller information (GDPR)
pub struct DataControllerInfo {
    pub name: &'static str,
    pub email: &'static str,
    pub address: &'static str,
}

/// Consent dialog UI model
#[derive(Debug, Clone)]
pub struct ConsentDialogModel {
    pub region: PrivacyRegion,
    pub categories: Vec<ConsentCategoryModel>,
    pub privacy_url: String,
}

/// Consent category UI model
#[derive(Debug, Clone)]
pub struct ConsentCategoryModel {
    pub category: ConsentCategory,
    pub enabled: bool,
    pub title: String,
    pub description: String,
}

impl ConsentManager {
    /// Get UI model for consent dialog
    pub fn get_dialog_model(&self) -> ConsentDialogModel {
        let status = self.status.lock().unwrap();

        let categories = ConsentCategory::all()
            .into_iter()
            .filter(|c| !c.is_essential())
            .map(|c| ConsentCategoryModel {
                category: c,
                enabled: status.is_allowed(c),
                title: c.display_name().to_string(),
                description: c.description().to_string(),
            })
            .collect();

        ConsentDialogModel {
            region: status.region.unwrap_or(PrivacyRegion::Other),
            categories,
            privacy_url: self.get_privacy_policy_url().to_string(),
        }
    }
}
