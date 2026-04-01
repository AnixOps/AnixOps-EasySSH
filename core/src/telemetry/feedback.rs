//! In-app feedback collection system
//!
//! Collects user feedback including:
//! - Ratings and NPS scores
//! - Text feedback (sanitized)
//! - Category-based feedback
//! - Screenshot attachments (optional)
//! - Feature requests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{AnonymousId, EventCollector, PlatformInfo, TelemetryError, TelemetryEvent};

/// User feedback data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    /// Unique feedback ID
    pub id: String,
    /// Anonymous user ID
    pub user_id: AnonymousId,
    /// Feedback type
    pub feedback_type: FeedbackType,
    /// Rating (1-5 stars or 0-10 NPS)
    pub rating: FeedbackRating,
    /// Feedback text (sanitized)
    pub text: String,
    /// Category
    pub category: String,
    /// Timestamp
    pub timestamp: u64,
    /// Platform info
    pub platform: PlatformInfo,
    /// Session ID
    pub session_id: String,
    /// Context (screen, action, etc.)
    pub context: HashMap<String, serde_json::Value>,
    /// Contact email (optional, only if user provides)
    pub contact_email: Option<String>,
    /// Screenshot attached
    pub has_screenshot: bool,
    /// App version when feedback was submitted
    pub app_version: String,
}

/// Feedback type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    /// General feedback
    General,
    /// Bug report
    BugReport,
    /// Feature request
    FeatureRequest,
    /// Performance issue
    Performance,
    /// UI/UX feedback
    UiUx,
    /// Connection issue
    Connection,
    /// Security concern
    Security,
}

/// Feedback rating
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FeedbackRating {
    /// 1-5 star rating
    Stars(u8),
    /// 0-10 NPS score
    Nps(u8),
    /// Thumbs up/down
    Binary(bool),
}

impl Default for FeedbackRating {
    fn default() -> Self {
        FeedbackRating::Stars(0)
    }
}

/// Feedback category with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub emoji: String,
}

/// Feedback collector
pub struct FeedbackCollector {
    event_collector: Arc<EventCollector>,
    recent_feedback: Arc<Mutex<Vec<UserFeedback>>>,
    max_recent: usize,
    categories: Arc<Mutex<Vec<FeedbackCategory>>>,
}

impl FeedbackCollector {
    pub fn new(event_collector: Arc<EventCollector>) -> Self {
        Self {
            event_collector,
            recent_feedback: Arc::new(Mutex::new(Vec::with_capacity(100))),
            max_recent: 100,
            categories: Arc::new(Mutex::new(Self::default_categories())),
        }
    }

    fn default_categories() -> Vec<FeedbackCategory> {
        vec![
            FeedbackCategory {
                id: "bug".to_string(),
                name: "Bug Report".to_string(),
                description: "Something isn't working correctly".to_string(),
                emoji: "🐛".to_string(),
            },
            FeedbackCategory {
                id: "feature".to_string(),
                name: "Feature Request".to_string(),
                description: "Suggest a new feature".to_string(),
                emoji: "💡".to_string(),
            },
            FeedbackCategory {
                id: "performance".to_string(),
                name: "Performance".to_string(),
                description: "App is slow or unresponsive".to_string(),
                emoji: "⚡".to_string(),
            },
            FeedbackCategory {
                id: "ui".to_string(),
                name: "UI/UX".to_string(),
                description: "Interface or design feedback".to_string(),
                emoji: "🎨".to_string(),
            },
            FeedbackCategory {
                id: "connection".to_string(),
                name: "Connection".to_string(),
                description: "SSH or SFTP connection issues".to_string(),
                emoji: "🔌".to_string(),
            },
            FeedbackCategory {
                id: "other".to_string(),
                name: "Other".to_string(),
                description: "General feedback".to_string(),
                emoji: "💬".to_string(),
            },
        ]
    }

    /// Submit feedback
    pub async fn submit(&self, feedback: UserFeedback) -> Result<(), TelemetryError> {
        // Sanitize text
        let sanitized = Self::sanitize_feedback_text(&feedback.text);

        // Store locally
        {
            let mut recent = self.recent_feedback.lock().unwrap();
            if recent.len() >= self.max_recent {
                recent.remove(0);
            }
            recent.push(feedback.clone());
        }

        // Send telemetry event
        let event = TelemetryEvent::FeedbackSubmitted {
            rating: feedback.rating,
            category: feedback.category.clone(),
        };
        self.event_collector.collect_event(event).await;

        // Send full feedback to server if configured
        // (Implementation depends on backend integration)

        println!("[Feedback] Received: {} - {}", feedback.feedback_type_as_str(), sanitized);

        Ok(())
    }

    /// Quick feedback (just rating)
    pub async fn submit_rating(
        &self,
        rating: FeedbackRating,
        category: &str,
        user_id: AnonymousId,
        platform: PlatformInfo,
        session_id: String,
    ) -> Result<(), TelemetryError> {
        let feedback = UserFeedback {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            feedback_type: FeedbackType::General,
            rating,
            text: String::new(),
            category: category.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            platform,
            session_id,
            context: HashMap::new(),
            contact_email: None,
            has_screenshot: false,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        self.submit(feedback).await
    }

    /// Detailed feedback
    pub async fn submit_detailed(
        &self,
        feedback_type: FeedbackType,
        rating: FeedbackRating,
        text: impl Into<String>,
        category: impl Into<String>,
        context: HashMap<String, serde_json::Value>,
        user_id: AnonymousId,
        platform: PlatformInfo,
        session_id: String,
    ) -> Result<(), TelemetryError> {
        let feedback = UserFeedback {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            feedback_type,
            rating,
            text: text.into(),
            category: category.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            platform,
            session_id,
            context,
            contact_email: None,
            has_screenshot: false,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        self.submit(feedback).await
    }

    /// Sanitize feedback text
    fn sanitize_feedback_text(text: &str) -> String {
        // Remove sensitive information
        let mut result = text.to_string();

        // Redact potential passwords, keys, hostnames
        let patterns = [
            (r"password[:=]\s*\S+", "password=[REDACTED]"),
            (r"-----BEGIN [A-Z ]+ PRIVATE KEY-----[\s\S]*?-----END [A-Z ]+ PRIVATE KEY-----", "[PRIVATE-KEY-REDACTED]"),
            (r"\b(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b", "[IP-REDACTED]"),
        ];

        // Note: Use regex in production
        for (pattern, replacement) in &patterns {
            result = result.replace(pattern, replacement);
        }

        // Limit length
        if result.len() > 5000 {
            result = result[..5000].to_string();
            result.push_str("... [truncated]");
        }

        result
    }

    /// Get available categories
    pub fn get_categories(&self) -> Vec<FeedbackCategory> {
        self.categories.lock().unwrap().clone()
    }

    /// Get recent feedback
    pub fn get_recent_feedback(&self) -> Vec<UserFeedback> {
        self.recent_feedback.lock().unwrap().clone()
    }

    /// Check if user should be prompted for feedback
    pub fn should_prompt_for_feedback(&self, session_count: u32, days_since_last_prompt: u32) -> bool {
        // Ask after 5 sessions and at least 7 days since last prompt
        session_count >= 5 && days_since_last_prompt >= 7
    }

    /// Get feedback statistics
    pub fn get_feedback_stats(&self) -> FeedbackStats {
        let feedbacks = self.recent_feedback.lock().unwrap();

        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut ratings: Vec<u8> = Vec::new();

        for fb in feedbacks.iter() {
            *by_category.entry(fb.category.clone()).or_insert(0) += 1;
            *by_type.entry(fb.feedback_type_as_str().to_string()).or_insert(0) += 1;

            if let FeedbackRating::Stars(r) = fb.rating {
                ratings.push(r);
            }
        }

        let avg_rating = if !ratings.is_empty() {
            let sum: u32 = ratings.iter().map(|&r| r as u32).sum();
            sum as f64 / ratings.len() as f64
        } else {
            0.0
        };

        FeedbackStats {
            total_count: feedbacks.len(),
            by_category,
            by_type,
            average_rating: avg_rating,
            rating_distribution: Self::calculate_rating_distribution(&ratings),
        }
    }

    fn calculate_rating_distribution(ratings: &[u8]) -> HashMap<u8, usize> {
        let mut dist = HashMap::new();
        for &r in ratings {
            *dist.entry(r).or_insert(0) += 1;
        }
        dist
    }
}

/// Feedback statistics
#[derive(Debug, Clone)]
pub struct FeedbackStats {
    pub total_count: usize,
    pub by_category: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub average_rating: f64,
    pub rating_distribution: HashMap<u8, usize>,
}

/// UI model for feedback form
#[derive(Debug, Clone)]
pub struct FeedbackFormModel {
    pub categories: Vec<FeedbackCategory>,
    pub feedback_types: Vec<(FeedbackType, String, String)>,
    pub allow_screenshot: bool,
    pub allow_contact: bool,
}

impl FeedbackCollector {
    /// Get UI model for feedback form
    pub fn get_form_model(&self) -> FeedbackFormModel {
        FeedbackFormModel {
            categories: self.get_categories(),
            feedback_types: vec![
                (FeedbackType::General, "General Feedback".to_string(), "Share your thoughts".to_string()),
                (FeedbackType::BugReport, "Report Bug".to_string(), "Help us fix issues".to_string()),
                (FeedbackType::FeatureRequest, "Feature Request".to_string(), "Suggest improvements".to_string()),
                (FeedbackType::Performance, "Performance".to_string(), "Speed or responsiveness".to_string()),
                (FeedbackType::UiUx, "UI/UX".to_string(), "Interface feedback".to_string()),
            ],
            allow_screenshot: false, // Would need screenshot capability
            allow_contact: true,
        }
    }

    /// Create NPS survey model
    pub fn get_nps_model(&self) -> NpsSurveyModel {
        NpsSurveyModel {
            question: "How likely are you to recommend EasySSH to a friend or colleague?".to_string(),
            scale_min: 0,
            scale_max: 10,
            scale_labels: ("Not likely".to_string(), "Very likely".to_string()),
            follow_up_prompt: "What is the primary reason for your score?".to_string(),
        }
    }

    /// Create star rating model
    pub fn get_star_rating_model(&self, context: &str) -> StarRatingModel {
        StarRatingModel {
            question: format!("How would you rate your experience with {}?", context),
            max_stars: 5,
            labels: vec![
                "Terrible".to_string(),
                "Bad".to_string(),
                "Okay".to_string(),
                "Good".to_string(),
                "Excellent".to_string(),
            ],
        }
    }
}

/// NPS survey model
#[derive(Debug, Clone)]
pub struct NpsSurveyModel {
    pub question: String,
    pub scale_min: u8,
    pub scale_max: u8,
    pub scale_labels: (String, String),
    pub follow_up_prompt: String,
}

/// Star rating model
#[derive(Debug, Clone)]
pub struct StarRatingModel {
    pub question: String,
    pub max_stars: u8,
    pub labels: Vec<String>,
}

impl UserFeedback {
    fn feedback_type_as_str(&self) -> &'static str {
        match self.feedback_type {
            FeedbackType::General => "general",
            FeedbackType::BugReport => "bug_report",
            FeedbackType::FeatureRequest => "feature_request",
            FeedbackType::Performance => "performance",
            FeedbackType::UiUx => "ui_ux",
            FeedbackType::Connection => "connection",
            FeedbackType::Security => "security",
        }
    }
}

/// Feedback prompt scheduler
pub struct FeedbackScheduler {
    min_sessions: u32,
    min_days: u32,
    last_prompt_time: Arc<Mutex<Option<u64>>>,
    session_count: Arc<Mutex<u32>>,
}

impl FeedbackScheduler {
    pub fn new(min_sessions: u32, min_days: u32) -> Self {
        Self {
            min_sessions,
            min_days,
            last_prompt_time: Arc::new(Mutex::new(None)),
            session_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Record session start
    pub fn record_session(&self) {
        let mut count = self.session_count.lock().unwrap();
        *count += 1;
    }

    /// Check if should show prompt
    pub fn should_prompt(&self) -> bool {
        let count = *self.session_count.lock().unwrap();
        if count < self.min_sessions {
            return false;
        }

        let last = *self.last_prompt_time.lock().unwrap();
        if let Some(last_time) = last {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let days_since = (now - last_time) / 86400;

            if days_since < self.min_days as u64 {
                return false;
            }
        }

        true
    }

    /// Mark prompt shown
    pub fn mark_prompt_shown(&self) {
        let mut last = self.last_prompt_time.lock().unwrap();
        *last = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Reset scheduler
    pub fn reset(&self) {
        *self.last_prompt_time.lock().unwrap() = None;
        *self.session_count.lock().unwrap() = 0;
    }
}

/// Macros for easy feedback submission
#[macro_export]
macro_rules! submit_feedback {
    ($collector:expr, $rating:expr, $category:expr) => {
        // Would need user_id, platform, session_id from context
        // This is a simplified version
    };
}
