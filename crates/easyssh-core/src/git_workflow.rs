use crate::git_types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Git Workflow Actions
// ============================================================================

/// Git workflow automation actions
#[derive(Debug, Clone)]
pub enum GitWorkflowAction {
    /// Clone a repository
    Clone {
        url: String,
        path: String,
        branch: Option<String>,
    },

    /// Stage files
    Stage { paths: Vec<String> },

    /// Unstage files
    Unstage { paths: Vec<String> },

    /// Commit changes
    Commit { message: String },

    /// Push to remote
    Push { remote: String, refspec: String },

    /// Pull from remote
    Pull { remote: String },

    /// Fetch from remote
    Fetch { remote: String },

    /// Checkout branch
    Checkout { branch: String, create: bool },

    /// Merge branch
    Merge { branch: String },

    /// Create tag
    CreateTag {
        name: String,
        message: Option<String>,
    },

    /// Push tag
    PushTag { name: String, remote: String },

    /// Stash save
    StashSave { message: Option<String> },

    /// Stash pop
    StashPop { index: usize },

    /// Submodule update
    SubmoduleUpdate,

    /// Discard changes
    Discard { paths: Vec<String> },

    /// Add remote
    AddRemote { name: String, url: String },

    /// Remove remote
    RemoveRemote { name: String },

    /// Set credentials
    SetCredentials { credentials: CredentialType },

    /// Get repository status
    GetStatus,

    /// Get commit log
    GetLog {
        branch: Option<String>,
        limit: usize,
    },

    // ============================================================================
    // Git Flow Actions
    // ============================================================================
    /// Initialize Git Flow
    GitFlowInit {
        /// Main production branch (default: main)
        main_branch: Option<String>,
        /// Development branch (default: develop)
        develop_branch: Option<String>,
        /// Feature branch prefix (default: feature/)
        feature_prefix: Option<String>,
        /// Release branch prefix (default: release/)
        release_prefix: Option<String>,
        /// Hotfix branch prefix (default: hotfix/)
        hotfix_prefix: Option<String>,
        /// Tag prefix (default: v)
        tag_prefix: Option<String>,
    },

    /// Start a feature branch
    GitFlowFeatureStart {
        name: String,
        base_branch: Option<String>,
    },

    /// Finish a feature branch
    GitFlowFeatureFinish { name: String, keep_branch: bool },

    /// Publish feature branch to remote
    GitFlowFeaturePublish { name: String },

    /// Start a release branch
    GitFlowReleaseStart {
        version: String,
        base_branch: Option<String>,
    },

    /// Finish a release branch
    GitFlowReleaseFinish {
        version: String,
        tag_message: Option<String>,
        push_to_remote: bool,
    },

    /// Start a hotfix branch
    GitFlowHotfixStart {
        version: String,
        base_branch: Option<String>,
    },

    /// Finish a hotfix branch
    GitFlowHotfixFinish {
        version: String,
        tag_message: Option<String>,
        push_to_remote: bool,
    },

    // ============================================================================
    // PR Management Actions
    // ============================================================================
    /// Create a pull request
    CreatePullRequest {
        title: String,
        description: String,
        source_branch: String,
        target_branch: String,
        draft: bool,
        reviewers: Vec<String>,
    },

    /// Update a pull request
    UpdatePullRequest {
        number: u64,
        title: Option<String>,
        description: Option<String>,
        state: Option<PullRequestState>,
    },

    /// List pull requests
    ListPullRequests {
        state: PullRequestState,
        limit: usize,
    },

    /// Get pull request details
    GetPullRequest { number: u64 },

    /// Review a pull request
    ReviewPullRequest {
        number: u64,
        action: PullRequestReviewAction,
        comment: Option<String>,
    },

    /// Merge a pull request
    MergePullRequest {
        number: u64,
        method: MergeMethod,
        commit_message: Option<String>,
        delete_source_branch: bool,
    },

    // ============================================================================
    // CI/CD Actions
    // ============================================================================
    /// Get CI/CD pipeline status
    GetCiStatus {
        branch: Option<String>,
        commit: Option<String>,
    },

    /// List CI/CD pipelines
    ListCiPipelines {
        branch: Option<String>,
        status: Option<CiPipelineStatus>,
        limit: usize,
    },

    /// Retry CI/CD pipeline
    RetryCiPipeline { pipeline_id: String },

    /// Cancel CI/CD pipeline
    CancelCiPipeline { pipeline_id: String },

    // ============================================================================
    // Code Review Actions
    // ============================================================================
    /// Run code review checklist
    RunCodeReviewChecklist {
        branch: String,
        base_branch: Option<String>,
        checklist_items: Vec<ChecklistItem>,
    },

    /// Get code review report
    GetCodeReviewReport { commit_id: String },

    /// Add PR comment
    AddPrComment {
        file_path: Option<String>,
        line_number: Option<u32>,
        comment: String,
    },
}

impl GitWorkflowAction {
    /// Get action name
    pub fn name(&self) -> &'static str {
        match self {
            GitWorkflowAction::Clone { .. } => "git_clone",
            GitWorkflowAction::Stage { .. } => "git_stage",
            GitWorkflowAction::Unstage { .. } => "git_unstage",
            GitWorkflowAction::Commit { .. } => "git_commit",
            GitWorkflowAction::Push { .. } => "git_push",
            GitWorkflowAction::Pull { .. } => "git_pull",
            GitWorkflowAction::Fetch { .. } => "git_fetch",
            GitWorkflowAction::Checkout { .. } => "git_checkout",
            GitWorkflowAction::Merge { .. } => "git_merge",
            GitWorkflowAction::CreateTag { .. } => "git_create_tag",
            GitWorkflowAction::PushTag { .. } => "git_push_tag",
            GitWorkflowAction::StashSave { .. } => "git_stash_save",
            GitWorkflowAction::StashPop { .. } => "git_stash_pop",
            GitWorkflowAction::SubmoduleUpdate => "git_submodule_update",
            GitWorkflowAction::Discard { .. } => "git_discard",
            GitWorkflowAction::AddRemote { .. } => "git_add_remote",
            GitWorkflowAction::RemoveRemote { .. } => "git_remove_remote",
            GitWorkflowAction::SetCredentials { .. } => "git_set_credentials",
            GitWorkflowAction::GetStatus => "git_get_status",
            GitWorkflowAction::GetLog { .. } => "git_get_log",

            // Git Flow
            GitWorkflowAction::GitFlowInit { .. } => "gitflow_init",
            GitWorkflowAction::GitFlowFeatureStart { .. } => "gitflow_feature_start",
            GitWorkflowAction::GitFlowFeatureFinish { .. } => "gitflow_feature_finish",
            GitWorkflowAction::GitFlowFeaturePublish { .. } => "gitflow_feature_publish",
            GitWorkflowAction::GitFlowReleaseStart { .. } => "gitflow_release_start",
            GitWorkflowAction::GitFlowReleaseFinish { .. } => "gitflow_release_finish",
            GitWorkflowAction::GitFlowHotfixStart { .. } => "gitflow_hotfix_start",
            GitWorkflowAction::GitFlowHotfixFinish { .. } => "gitflow_hotfix_finish",

            // PR Management
            GitWorkflowAction::CreatePullRequest { .. } => "pr_create",
            GitWorkflowAction::UpdatePullRequest { .. } => "pr_update",
            GitWorkflowAction::ListPullRequests { .. } => "pr_list",
            GitWorkflowAction::GetPullRequest { .. } => "pr_get",
            GitWorkflowAction::ReviewPullRequest { .. } => "pr_review",
            GitWorkflowAction::MergePullRequest { .. } => "pr_merge",

            // CI/CD
            GitWorkflowAction::GetCiStatus { .. } => "ci_status",
            GitWorkflowAction::ListCiPipelines { .. } => "ci_list",
            GitWorkflowAction::RetryCiPipeline { .. } => "ci_retry",
            GitWorkflowAction::CancelCiPipeline { .. } => "ci_cancel",

            // Code Review
            GitWorkflowAction::RunCodeReviewChecklist { .. } => "review_checklist",
            GitWorkflowAction::GetCodeReviewReport { .. } => "review_report",
            GitWorkflowAction::AddPrComment { .. } => "pr_comment",
        }
    }

    /// Get action description
    pub fn description(&self) -> String {
        match self {
            GitWorkflowAction::Clone { url, .. } => format!("Clone repository from {}", url),
            GitWorkflowAction::Stage { paths } => format!("Stage {} files", paths.len()),
            GitWorkflowAction::Unstage { paths } => format!("Unstage {} files", paths.len()),
            GitWorkflowAction::Commit { message } => format!("Commit: {}", message),
            GitWorkflowAction::Push { remote, refspec } => {
                format!("Push {} to {}", refspec, remote)
            }
            GitWorkflowAction::Pull { remote } => format!("Pull from {}", remote),
            GitWorkflowAction::Fetch { remote } => format!("Fetch from {}", remote),
            GitWorkflowAction::Checkout { branch, create } => {
                if *create {
                    format!("Create and checkout branch {}", branch)
                } else {
                    format!("Checkout branch {}", branch)
                }
            }
            GitWorkflowAction::Merge { branch } => format!("Merge branch {}", branch),
            GitWorkflowAction::CreateTag { name, .. } => format!("Create tag {}", name),
            GitWorkflowAction::PushTag { name, remote } => {
                format!("Push tag {} to {}", name, remote)
            }
            GitWorkflowAction::StashSave { .. } => "Save stash".to_string(),
            GitWorkflowAction::StashPop { index } => format!("Pop stash at index {}", index),
            GitWorkflowAction::SubmoduleUpdate => "Update submodules".to_string(),
            GitWorkflowAction::Discard { paths } => {
                format!("Discard changes in {} files", paths.len())
            }
            GitWorkflowAction::AddRemote { name, url } => format!("Add remote {} ({})", name, url),
            GitWorkflowAction::RemoveRemote { name } => format!("Remove remote {}", name),
            GitWorkflowAction::SetCredentials { .. } => "Set Git credentials".to_string(),
            GitWorkflowAction::GetStatus => "Get repository status".to_string(),
            GitWorkflowAction::GetLog { .. } => "Get commit log".to_string(),

            // Git Flow
            GitWorkflowAction::GitFlowInit { .. } => "Initialize Git Flow".to_string(),
            GitWorkflowAction::GitFlowFeatureStart { name, .. } => {
                format!("Start feature branch {}", name)
            }
            GitWorkflowAction::GitFlowFeatureFinish { name, .. } => {
                format!("Finish feature branch {}", name)
            }
            GitWorkflowAction::GitFlowFeaturePublish { name } => {
                format!("Publish feature branch {}", name)
            }
            GitWorkflowAction::GitFlowReleaseStart { version, .. } => {
                format!("Start release {}", version)
            }
            GitWorkflowAction::GitFlowReleaseFinish { version, .. } => {
                format!("Finish release {}", version)
            }
            GitWorkflowAction::GitFlowHotfixStart { version, .. } => {
                format!("Start hotfix {}", version)
            }
            GitWorkflowAction::GitFlowHotfixFinish { version, .. } => {
                format!("Finish hotfix {}", version)
            }

            // PR Management
            GitWorkflowAction::CreatePullRequest { title, .. } => format!("Create PR: {}", title),
            GitWorkflowAction::UpdatePullRequest { number, .. } => format!("Update PR #{}", number),
            GitWorkflowAction::ListPullRequests { .. } => "List pull requests".to_string(),
            GitWorkflowAction::GetPullRequest { number } => format!("Get PR #{}", number),
            GitWorkflowAction::ReviewPullRequest { number, .. } => format!("Review PR #{}", number),
            GitWorkflowAction::MergePullRequest { number, .. } => format!("Merge PR #{}", number),

            // CI/CD
            GitWorkflowAction::GetCiStatus { .. } => "Get CI status".to_string(),
            GitWorkflowAction::ListCiPipelines { .. } => "List CI pipelines".to_string(),
            GitWorkflowAction::RetryCiPipeline { pipeline_id } => {
                format!("Retry pipeline {}", pipeline_id)
            }
            GitWorkflowAction::CancelCiPipeline { pipeline_id } => {
                format!("Cancel pipeline {}", pipeline_id)
            }

            // Code Review
            GitWorkflowAction::RunCodeReviewChecklist { .. } => {
                "Run code review checklist".to_string()
            }
            GitWorkflowAction::GetCodeReviewReport { commit_id } => {
                format!("Get review report for {}", commit_id)
            }
            GitWorkflowAction::AddPrComment { file_path, .. } => match file_path {
                Some(path) => format!("Add comment on {}", path),
                None => "Add PR comment".to_string(),
            },
        }
    }

    /// Check if action is a Git Flow action
    pub fn is_gitflow(&self) -> bool {
        matches!(
            self,
            GitWorkflowAction::GitFlowInit { .. }
                | GitWorkflowAction::GitFlowFeatureStart { .. }
                | GitWorkflowAction::GitFlowFeatureFinish { .. }
                | GitWorkflowAction::GitFlowFeaturePublish { .. }
                | GitWorkflowAction::GitFlowReleaseStart { .. }
                | GitWorkflowAction::GitFlowReleaseFinish { .. }
                | GitWorkflowAction::GitFlowHotfixStart { .. }
                | GitWorkflowAction::GitFlowHotfixFinish { .. }
        )
    }

    /// Check if action is a PR management action
    pub fn is_pr_management(&self) -> bool {
        matches!(
            self,
            GitWorkflowAction::CreatePullRequest { .. }
                | GitWorkflowAction::UpdatePullRequest { .. }
                | GitWorkflowAction::ListPullRequests { .. }
                | GitWorkflowAction::GetPullRequest { .. }
                | GitWorkflowAction::ReviewPullRequest { .. }
                | GitWorkflowAction::MergePullRequest { .. }
        )
    }

    /// Check if action is a CI/CD action
    pub fn is_ci_cd(&self) -> bool {
        matches!(
            self,
            GitWorkflowAction::GetCiStatus { .. }
                | GitWorkflowAction::ListCiPipelines { .. }
                | GitWorkflowAction::RetryCiPipeline { .. }
                | GitWorkflowAction::CancelCiPipeline { .. }
        )
    }
}

// ============================================================================
// Git Flow Types
// ============================================================================

/// Git Flow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFlowConfig {
    pub main_branch: String,
    pub develop_branch: String,
    pub feature_prefix: String,
    pub release_prefix: String,
    pub hotfix_prefix: String,
    pub tag_prefix: String,
}

impl Default for GitFlowConfig {
    fn default() -> Self {
        Self {
            main_branch: "main".to_string(),
            develop_branch: "develop".to_string(),
            feature_prefix: "feature/".to_string(),
            release_prefix: "release/".to_string(),
            hotfix_prefix: "hotfix/".to_string(),
            tag_prefix: "v".to_string(),
        }
    }
}

/// Git Flow branch type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitFlowBranchType {
    Feature,
    Release,
    Hotfix,
    Develop,
    Main,
    Support,
}

impl GitFlowBranchType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitFlowBranchType::Feature => "feature",
            GitFlowBranchType::Release => "release",
            GitFlowBranchType::Hotfix => "hotfix",
            GitFlowBranchType::Develop => "develop",
            GitFlowBranchType::Main => "main",
            GitFlowBranchType::Support => "support",
        }
    }

    pub fn prefix(&self, config: &GitFlowConfig) -> String {
        match self {
            GitFlowBranchType::Feature => config.feature_prefix.clone(),
            GitFlowBranchType::Release => config.release_prefix.clone(),
            GitFlowBranchType::Hotfix => config.hotfix_prefix.clone(),
            GitFlowBranchType::Develop => String::new(),
            GitFlowBranchType::Main => String::new(),
            GitFlowBranchType::Support => "support/".to_string(),
        }
    }
}

/// Git Flow branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFlowBranchInfo {
    pub name: String,
    pub branch_type: GitFlowBranchType,
    pub base_commit: String,
    pub parent_branch: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_published: bool,
    pub associated_pr: Option<u64>,
}

/// Git Flow operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFlowResult {
    pub success: bool,
    pub branch_name: String,
    pub branch_type: GitFlowBranchType,
    pub message: String,
    pub commit_id: Option<String>,
}

// ============================================================================
// Pull Request Types
// ============================================================================

/// Pull Request state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
    Draft,
    All,
}

impl Default for PullRequestState {
    fn default() -> Self {
        PullRequestState::Open
    }
}

impl PullRequestState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PullRequestState::Open => "open",
            PullRequestState::Closed => "closed",
            PullRequestState::Merged => "merged",
            PullRequestState::Draft => "draft",
            PullRequestState::All => "all",
        }
    }
}

/// Merge method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }
}

/// Pull Request review action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PullRequestReviewAction {
    Approve,
    RequestChanges,
    Comment,
    Dismiss,
}

/// Pull Request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub description: String,
    pub state: PullRequestState,
    pub draft: bool,
    pub source_branch: String,
    pub target_branch: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub merge_commit_sha: Option<String>,
    pub head_sha: String,
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
    pub comments_count: usize,
    pub review_comments_count: usize,
    pub commits_count: usize,
    pub additions: usize,
    pub deletions: usize,
    pub changed_files: usize,
    pub is_mergeable: Option<bool>,
    pub mergeable_state: Option<String>,
    pub checks_status: Option<CiChecksSummary>,
}

/// Pull Request list filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PullRequestFilter {
    pub state: PullRequestState,
    pub head: Option<String>,
    pub base: Option<String>,
    pub author: Option<String>,
    pub limit: usize,
    pub sort_by: Option<String>,
    pub sort_direction: Option<String>,
}

/// Pull Request review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestReview {
    pub id: String,
    pub reviewer: String,
    pub state: PullRequestReviewState,
    pub comment: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

/// Pull Request review state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PullRequestReviewState {
    Approved,
    ChangesRequested,
    Commented,
    Pending,
}

/// Pull Request comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestComment {
    pub id: String,
    pub author: String,
    pub body: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create Pull Request input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePullRequestInput {
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
}

// ============================================================================
// CI/CD Types
// ============================================================================

/// CI/CD pipeline status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiPipelineStatus {
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
    Manual,
}

impl CiPipelineStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CiPipelineStatus::Pending => "pending",
            CiPipelineStatus::Running => "running",
            CiPipelineStatus::Success => "success",
            CiPipelineStatus::Failed => "failed",
            CiPipelineStatus::Canceled => "canceled",
            CiPipelineStatus::Skipped => "skipped",
            CiPipelineStatus::Manual => "manual",
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(
            self,
            CiPipelineStatus::Success
                | CiPipelineStatus::Failed
                | CiPipelineStatus::Canceled
                | CiPipelineStatus::Skipped
        )
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, CiPipelineStatus::Success)
    }
}

/// CI/CD pipeline information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiPipeline {
    pub id: String,
    pub sha: String,
    pub ref_name: String,
    pub status: CiPipelineStatus,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub web_url: String,
    pub triggered_by: String,
    pub stages: Vec<CiStage>,
    pub commit_message: String,
    pub branch: String,
}

/// CI/CD stage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStage {
    pub name: String,
    pub status: CiPipelineStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub jobs: Vec<CiJob>,
}

/// CI/CD job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJob {
    pub id: String,
    pub name: String,
    pub status: CiPipelineStatus,
    pub stage: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub web_url: String,
    pub failure_reason: Option<String>,
    pub allow_failure: bool,
}

/// CI checks summary for PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiChecksSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
    pub skipped: usize,
    pub conclusion: Option<CiPipelineStatus>,
    pub checks: Vec<CheckRun>,
}

/// Individual check run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRun {
    pub name: String,
    pub status: CiPipelineStatus,
    pub conclusion: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output_summary: Option<String>,
    pub details_url: String,
}

// ============================================================================
// Code Review Types
// ============================================================================

/// Code review checklist item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub id: String,
    pub category: ReviewCategory,
    pub title: String,
    pub description: String,
    pub is_checked: bool,
    pub is_required: bool,
    pub auto_checkable: bool,
}

/// Review category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewCategory {
    CodeStyle,
    Functionality,
    Security,
    Performance,
    Tests,
    Documentation,
    Architecture,
}

impl ReviewCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewCategory::CodeStyle => "code_style",
            ReviewCategory::Functionality => "functionality",
            ReviewCategory::Security => "security",
            ReviewCategory::Performance => "performance",
            ReviewCategory::Tests => "tests",
            ReviewCategory::Documentation => "documentation",
            ReviewCategory::Architecture => "architecture",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ReviewCategory::CodeStyle => "Code Style",
            ReviewCategory::Functionality => "Functionality",
            ReviewCategory::Security => "Security",
            ReviewCategory::Performance => "Performance",
            ReviewCategory::Tests => "Tests",
            ReviewCategory::Documentation => "Documentation",
            ReviewCategory::Architecture => "Architecture",
        }
    }
}

/// Code review checklist template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistTemplate {
    pub name: String,
    pub description: String,
    pub items: Vec<ChecklistItemTemplate>,
}

/// Checklist item template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItemTemplate {
    pub category: ReviewCategory,
    pub title: String,
    pub description: String,
    pub is_required: bool,
    pub auto_checkable: bool,
    pub check_command: Option<String>,
}

/// Code review report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReviewReport {
    pub commit_id: String,
    pub branch: String,
    pub base_branch: String,
    pub created_at: DateTime<Utc>,
    pub overall_status: ReviewStatus,
    pub categories: Vec<CategoryReviewResult>,
    pub issues: Vec<ReviewIssue>,
    pub statistics: ReviewStatistics,
}

/// Review status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewStatus {
    Pass,
    NeedsReview,
    HasIssues,
    Failed,
}

/// Category review result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryReviewResult {
    pub category: ReviewCategory,
    pub status: ReviewStatus,
    pub checked_items: usize,
    pub total_items: usize,
    pub issues_count: usize,
}

/// Review issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    pub id: String,
    pub category: ReviewCategory,
    pub severity: IssueSeverity,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub suggestion: Option<String>,
}

/// Issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl IssueSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueSeverity::Info => "info",
            IssueSeverity::Warning => "warning",
            IssueSeverity::Error => "error",
            IssueSeverity::Critical => "critical",
        }
    }
}

/// Review statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewStatistics {
    pub total_files_changed: usize,
    pub total_lines_added: usize,
    pub total_lines_deleted: usize,
    pub total_issues: usize,
    pub critical_issues: usize,
    pub error_issues: usize,
    pub warning_issues: usize,
    pub info_issues: usize,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create standard code review checklist
pub fn create_standard_checklist() -> Vec<ChecklistItem> {
    vec![
        // Code Style
        ChecklistItem {
            id: "cs-1".to_string(),
            category: ReviewCategory::CodeStyle,
            title: "Consistent formatting".to_string(),
            description: "Code follows project formatting standards".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        ChecklistItem {
            id: "cs-2".to_string(),
            category: ReviewCategory::CodeStyle,
            title: "Naming conventions".to_string(),
            description: "Variables, functions, and types follow naming conventions".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "cs-3".to_string(),
            category: ReviewCategory::CodeStyle,
            title: "No unused code".to_string(),
            description: "No unused imports, variables, or dead code".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        // Functionality
        ChecklistItem {
            id: "fn-1".to_string(),
            category: ReviewCategory::Functionality,
            title: "Logic correctness".to_string(),
            description: "Business logic is correct and handles edge cases".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "fn-2".to_string(),
            category: ReviewCategory::Functionality,
            title: "Error handling".to_string(),
            description: "Errors are properly handled and reported".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "fn-3".to_string(),
            category: ReviewCategory::Functionality,
            title: "Input validation".to_string(),
            description: "User inputs are validated".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        // Security
        ChecklistItem {
            id: "sc-1".to_string(),
            category: ReviewCategory::Security,
            title: "No hardcoded secrets".to_string(),
            description: "No passwords, tokens, or keys in code".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        ChecklistItem {
            id: "sc-2".to_string(),
            category: ReviewCategory::Security,
            title: "Injection prevention".to_string(),
            description: "SQL injection, XSS, and other injection attacks are prevented"
                .to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "sc-3".to_string(),
            category: ReviewCategory::Security,
            title: "Authorization checks".to_string(),
            description: "Proper authorization checks are in place".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        // Performance
        ChecklistItem {
            id: "pf-1".to_string(),
            category: ReviewCategory::Performance,
            title: "No N+1 queries".to_string(),
            description: "Database queries are optimized".to_string(),
            is_checked: false,
            is_required: false,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "pf-2".to_string(),
            category: ReviewCategory::Performance,
            title: "Resource cleanup".to_string(),
            description: "Resources are properly released".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        // Tests
        ChecklistItem {
            id: "ts-1".to_string(),
            category: ReviewCategory::Tests,
            title: "Unit tests".to_string(),
            description: "New code has appropriate unit tests".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        ChecklistItem {
            id: "ts-2".to_string(),
            category: ReviewCategory::Tests,
            title: "Test coverage".to_string(),
            description: "Code coverage is maintained or improved".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        ChecklistItem {
            id: "ts-3".to_string(),
            category: ReviewCategory::Tests,
            title: "Integration tests".to_string(),
            description: "Integration tests added where appropriate".to_string(),
            is_checked: false,
            is_required: false,
            auto_checkable: false,
        },
        // Documentation
        ChecklistItem {
            id: "dc-1".to_string(),
            category: ReviewCategory::Documentation,
            title: "API documentation".to_string(),
            description: "Public APIs are documented".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        ChecklistItem {
            id: "dc-2".to_string(),
            category: ReviewCategory::Documentation,
            title: "Code comments".to_string(),
            description: "Complex logic is explained with comments".to_string(),
            is_checked: false,
            is_required: false,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "dc-3".to_string(),
            category: ReviewCategory::Documentation,
            title: "Changelog updated".to_string(),
            description: "Changelog is updated with changes".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
        // Architecture
        ChecklistItem {
            id: "ar-1".to_string(),
            category: ReviewCategory::Architecture,
            title: "Single responsibility".to_string(),
            description: "Functions and classes have single responsibilities".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: false,
        },
        ChecklistItem {
            id: "ar-2".to_string(),
            category: ReviewCategory::Architecture,
            title: "No circular dependencies".to_string(),
            description: "No circular dependencies introduced".to_string(),
            is_checked: false,
            is_required: true,
            auto_checkable: true,
        },
    ]
}

/// Check if a branch name follows Git Flow naming convention
pub fn parse_gitflow_branch(
    name: &str,
    config: &GitFlowConfig,
) -> Option<(GitFlowBranchType, String)> {
    if name == config.main_branch {
        return Some((GitFlowBranchType::Main, name.to_string()));
    }
    if name == config.develop_branch {
        return Some((GitFlowBranchType::Develop, name.to_string()));
    }

    if name.starts_with(&config.feature_prefix) {
        let feature_name = name.strip_prefix(&config.feature_prefix).unwrap_or(name);
        return Some((GitFlowBranchType::Feature, feature_name.to_string()));
    }

    if name.starts_with(&config.release_prefix) {
        let version = name.strip_prefix(&config.release_prefix).unwrap_or(name);
        return Some((GitFlowBranchType::Release, version.to_string()));
    }

    if name.starts_with(&config.hotfix_prefix) {
        let version = name.strip_prefix(&config.hotfix_prefix).unwrap_or(name);
        return Some((GitFlowBranchType::Hotfix, version.to_string()));
    }

    None
}

/// Build full branch name from type and name
pub fn build_gitflow_branch_name(
    branch_type: GitFlowBranchType,
    name: &str,
    config: &GitFlowConfig,
) -> String {
    let prefix = branch_type.prefix(config);
    format!("{}{}", prefix, name)
}

// ============================================================================
// Workflow Builder
// ============================================================================

/// Workflow builder for chaining git operations
#[derive(Debug, Clone, Default)]
pub struct GitWorkflowBuilder {
    actions: Vec<GitWorkflowAction>,
    config: GitFlowConfig,
}

impl GitWorkflowBuilder {
    /// Create a new workflow builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set Git Flow configuration
    pub fn with_gitflow_config(mut self, config: GitFlowConfig) -> Self {
        self.config = config;
        self
    }

    /// Add clone action
    pub fn clone(mut self, url: impl Into<String>, path: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Clone {
            url: url.into(),
            path: path.into(),
            branch: None,
        });
        self
    }

    /// Add clone with branch action
    pub fn clone_branch(
        mut self,
        url: impl Into<String>,
        path: impl Into<String>,
        branch: impl Into<String>,
    ) -> Self {
        self.actions.push(GitWorkflowAction::Clone {
            url: url.into(),
            path: path.into(),
            branch: Some(branch.into()),
        });
        self
    }

    /// Add checkout action
    pub fn checkout(mut self, branch: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Checkout {
            branch: branch.into(),
            create: false,
        });
        self
    }

    /// Add checkout and create action
    pub fn checkout_create(mut self, branch: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Checkout {
            branch: branch.into(),
            create: true,
        });
        self
    }

    /// Add stage action
    pub fn stage(mut self, paths: Vec<String>) -> Self {
        self.actions.push(GitWorkflowAction::Stage { paths });
        self
    }

    /// Add stage all action
    pub fn stage_all(mut self) -> Self {
        self.actions.push(GitWorkflowAction::Stage {
            paths: vec![".".to_string()],
        });
        self
    }

    /// Add commit action
    pub fn commit(mut self, message: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Commit {
            message: message.into(),
        });
        self
    }

    /// Add push action
    pub fn push(mut self, remote: impl Into<String>, refspec: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Push {
            remote: remote.into(),
            refspec: refspec.into(),
        });
        self
    }

    /// Add pull action
    pub fn pull(mut self, remote: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Pull {
            remote: remote.into(),
        });
        self
    }

    /// Add fetch action
    pub fn fetch(mut self, remote: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Fetch {
            remote: remote.into(),
        });
        self
    }

    /// Add merge action
    pub fn merge(mut self, branch: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::Merge {
            branch: branch.into(),
        });
        self
    }

    /// Add Git Flow init action
    pub fn gitflow_init(mut self) -> Self {
        self.actions.push(GitWorkflowAction::GitFlowInit {
            main_branch: None,
            develop_branch: None,
            feature_prefix: None,
            release_prefix: None,
            hotfix_prefix: None,
            tag_prefix: None,
        });
        self
    }

    /// Add Git Flow feature start action
    pub fn gitflow_feature_start(mut self, name: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::GitFlowFeatureStart {
            name: name.into(),
            base_branch: None,
        });
        self
    }

    /// Add Git Flow feature finish action
    pub fn gitflow_feature_finish(mut self, name: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::GitFlowFeatureFinish {
            name: name.into(),
            keep_branch: false,
        });
        self
    }

    /// Add Git Flow release start action
    pub fn gitflow_release_start(mut self, version: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::GitFlowReleaseStart {
            version: version.into(),
            base_branch: None,
        });
        self
    }

    /// Add Git Flow release finish action
    pub fn gitflow_release_finish(mut self, version: impl Into<String>) -> Self {
        self.actions.push(GitWorkflowAction::GitFlowReleaseFinish {
            version: version.into(),
            tag_message: None,
            push_to_remote: true,
        });
        self
    }

    /// Add create PR action
    pub fn create_pr(
        mut self,
        title: impl Into<String>,
        description: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        self.actions.push(GitWorkflowAction::CreatePullRequest {
            title: title.into(),
            description: description.into(),
            source_branch: source.into(),
            target_branch: target.into(),
            draft: false,
            reviewers: vec![],
        });
        self
    }

    /// Add run code review checklist action
    pub fn run_checklist(mut self, branch: impl Into<String>) -> Self {
        self.actions
            .push(GitWorkflowAction::RunCodeReviewChecklist {
                branch: branch.into(),
                base_branch: None,
                checklist_items: create_standard_checklist(),
            });
        self
    }

    /// Get all actions in the workflow
    pub fn build(self) -> Vec<GitWorkflowAction> {
        self.actions
    }

    /// Get Git Flow configuration
    pub fn config(&self) -> &GitFlowConfig {
        &self.config
    }
}

// ============================================================================
// Helper Functions for Common Workflows
// ============================================================================

/// Helper function to create a clone action
pub fn git_clone(url: impl Into<String>, path: impl Into<String>) -> GitWorkflowAction {
    GitWorkflowAction::Clone {
        url: url.into(),
        path: path.into(),
        branch: None,
    }
}

/// Helper function to create a stage action
pub fn git_stage(paths: Vec<String>) -> GitWorkflowAction {
    GitWorkflowAction::Stage { paths }
}

/// Helper function to create a commit action
pub fn git_commit(message: impl Into<String>) -> GitWorkflowAction {
    GitWorkflowAction::Commit {
        message: message.into(),
    }
}

/// Helper function to create a push action
pub fn git_push(remote: impl Into<String>, refspec: impl Into<String>) -> GitWorkflowAction {
    GitWorkflowAction::Push {
        remote: remote.into(),
        refspec: refspec.into(),
    }
}

/// Helper function to create a pull action
pub fn git_pull(remote: impl Into<String>) -> GitWorkflowAction {
    GitWorkflowAction::Pull {
        remote: remote.into(),
    }
}

/// Helper function to create a checkout action
pub fn git_checkout(branch: impl Into<String>, create: bool) -> GitWorkflowAction {
    GitWorkflowAction::Checkout {
        branch: branch.into(),
        create,
    }
}

/// Helper function to create a fetch action
pub fn git_fetch(remote: impl Into<String>) -> GitWorkflowAction {
    GitWorkflowAction::Fetch {
        remote: remote.into(),
    }
}

/// Helper function to create a merge action
pub fn git_merge(branch: impl Into<String>) -> GitWorkflowAction {
    GitWorkflowAction::Merge {
        branch: branch.into(),
    }
}

/// Create a complete feature workflow
pub fn feature_workflow(config: &GitFlowConfig, feature_name: &str) -> Vec<GitWorkflowAction> {
    let feature_branch =
        build_gitflow_branch_name(GitFlowBranchType::Feature, feature_name, config);

    vec![
        GitWorkflowAction::GitFlowFeatureStart {
            name: feature_name.to_string(),
            base_branch: None,
        },
        GitWorkflowAction::Push {
            remote: "origin".to_string(),
            refspec: feature_branch.clone(),
        },
        GitWorkflowAction::CreatePullRequest {
            title: format!("Feature: {}", feature_name),
            description: format!("Implement feature: {}", feature_name),
            source_branch: feature_branch,
            target_branch: config.develop_branch.clone(),
            draft: false,
            reviewers: vec![],
        },
    ]
}

/// Create a complete release workflow
pub fn release_workflow(config: &GitFlowConfig, version: &str) -> Vec<GitWorkflowAction> {
    let release_branch = build_gitflow_branch_name(GitFlowBranchType::Release, version, config);
    let tag_name = format!("{}{}", config.tag_prefix, version);

    vec![
        GitWorkflowAction::GitFlowReleaseStart {
            version: version.to_string(),
            base_branch: Some(config.develop_branch.clone()),
        },
        GitWorkflowAction::Push {
            remote: "origin".to_string(),
            refspec: release_branch.clone(),
        },
        GitWorkflowAction::CreatePullRequest {
            title: format!("Release {}", version),
            description: format!("Prepare release {}", version),
            source_branch: release_branch,
            target_branch: config.main_branch.clone(),
            draft: false,
            reviewers: vec![],
        },
        GitWorkflowAction::GitFlowReleaseFinish {
            version: version.to_string(),
            tag_message: Some(format!("Release version {}", version)),
            push_to_remote: true,
        },
        GitWorkflowAction::PushTag {
            name: tag_name,
            remote: "origin".to_string(),
        },
    ]
}

/// Create a hotfix workflow
pub fn hotfix_workflow(config: &GitFlowConfig, version: &str) -> Vec<GitWorkflowAction> {
    let hotfix_branch = build_gitflow_branch_name(GitFlowBranchType::Hotfix, version, config);
    let tag_name = format!("{}{}", config.tag_prefix, version);

    vec![
        GitWorkflowAction::GitFlowHotfixStart {
            version: version.to_string(),
            base_branch: Some(config.main_branch.clone()),
        },
        GitWorkflowAction::Push {
            remote: "origin".to_string(),
            refspec: hotfix_branch.clone(),
        },
        GitWorkflowAction::GitFlowHotfixFinish {
            version: version.to_string(),
            tag_message: Some(format!("Hotfix version {}", version)),
            push_to_remote: true,
        },
        GitWorkflowAction::PushTag {
            name: tag_name,
            remote: "origin".to_string(),
        },
    ]
}

/// Create code review workflow for a PR
pub fn code_review_workflow(branch: &str, base_branch: Option<&str>) -> Vec<GitWorkflowAction> {
    vec![
        GitWorkflowAction::Fetch {
            remote: "origin".to_string(),
        },
        GitWorkflowAction::RunCodeReviewChecklist {
            branch: branch.to_string(),
            base_branch: base_branch.map(|s| s.to_string()),
            checklist_items: create_standard_checklist(),
        },
        GitWorkflowAction::GetCiStatus {
            branch: Some(branch.to_string()),
            commit: None,
        },
    ]
}

// ============================================================================
// PR Template
// ============================================================================

/// PR template for creating standardized pull requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestTemplate {
    pub name: String,
    pub title_template: String,
    pub description_template: String,
    pub default_reviewers: Vec<String>,
    pub default_labels: Vec<String>,
}

impl PullRequestTemplate {
    /// Create a feature PR template
    pub fn feature() -> Self {
        Self {
            name: "Feature".to_string(),
            title_template: "feat: {feature_name}".to_string(),
            description_template: r#"## Summary
<!-- Describe the feature -->
{description}

## Changes
- [ ] Implementation
- [ ] Tests
- [ ] Documentation

## Testing
<!-- How was this tested? -->

## Screenshots
<!-- If applicable -->

## Related Issues
<!-- Link to related issues -->
Closes #{issue_number}
"#
            .to_string(),
            default_reviewers: vec![],
            default_labels: vec!["feature".to_string()],
        }
    }

    /// Create a bugfix PR template
    pub fn bugfix() -> Self {
        Self {
            name: "Bugfix".to_string(),
            title_template: "fix: {bug_description}".to_string(),
            description_template: r#"## Bug Description
<!-- Describe the bug -->
{description}

## Root Cause
<!-- What caused the bug? -->

## Solution
<!-- How was it fixed? -->

## Testing
- [ ] Reproduced the bug
- [ ] Verified the fix
- [ ] Added regression test

## Related Issues
<!-- Link to related issues -->
Fixes #{issue_number}
"#
            .to_string(),
            default_reviewers: vec![],
            default_labels: vec!["bugfix".to_string()],
        }
    }

    /// Create a hotfix PR template
    pub fn hotfix() -> Self {
        Self {
            name: "Hotfix".to_string(),
            title_template: "hotfix: {hotfix_description}".to_string(),
            description_template: r#"## Critical Issue
<!-- Describe the critical issue -->
{description}

## Impact
<!-- Who is affected? -->

## Fix
<!-- What was changed? -->

## Verification
- [ ] Tested in staging
- [ ] Ready for immediate deploy

## Related Incidents
<!-- Link to incident reports -->
"#
            .to_string(),
            default_reviewers: vec![],
            default_labels: vec!["hotfix".to_string(), "priority-critical".to_string()],
        }
    }

    /// Create a release PR template
    pub fn release() -> Self {
        Self {
            name: "Release".to_string(),
            title_template: "release: version {version}".to_string(),
            description_template: r#"## Release {version}

## Included Changes
<!-- List major changes -->
{changelog}

## Pre-release Checklist
- [ ] All tests passing
- [ ] Version bumped
- [ ] Changelog updated
- [ ] Documentation updated

## Post-release
- [ ] Deploy to production
- [ ] Verify monitoring
- [ ] Notify stakeholders
"#
            .to_string(),
            default_reviewers: vec![],
            default_labels: vec!["release".to_string()],
        }
    }

    /// Render template with variables
    pub fn render(&self, variables: &HashMap<String, String>) -> (String, String) {
        let mut title = self.title_template.clone();
        let mut description = self.description_template.clone();

        for (key, value) in variables {
            title = title.replace(&format!("{{{}}}", key), value);
            description = description.replace(&format!("{{{}}}", key), value);
        }

        (title, description)
    }
}

// ============================================================================
// CI/CD Configuration
// ============================================================================

/// CI/CD provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiProviderConfig {
    pub provider: CiProvider,
    pub api_url: String,
    pub token: String,
    pub project_id: String,
    pub default_branch: String,
}

/// CI/CD provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CiProvider {
    GitHubActions,
    GitLabCI,
    Jenkins,
    CircleCI,
    TravisCI,
    AzureDevOps,
    BuildKite,
    DroneCI,
}

impl CiProvider {
    pub fn display_name(&self) -> &'static str {
        match self {
            CiProvider::GitHubActions => "GitHub Actions",
            CiProvider::GitLabCI => "GitLab CI",
            CiProvider::Jenkins => "Jenkins",
            CiProvider::CircleCI => "CircleCI",
            CiProvider::TravisCI => "Travis CI",
            CiProvider::AzureDevOps => "Azure DevOps",
            CiProvider::BuildKite => "BuildKite",
            CiProvider::DroneCI => "Drone CI",
        }
    }

    pub fn webhook_event(&self, event: &str) -> Option<CiWebhookEvent> {
        match event {
            "push" => Some(CiWebhookEvent::Push),
            "pull_request" | "merge_request" => Some(CiWebhookEvent::PullRequest),
            "tag" => Some(CiWebhookEvent::Tag),
            "release" => Some(CiWebhookEvent::Release),
            "schedule" => Some(CiWebhookEvent::Schedule),
            _ => None,
        }
    }
}

/// CI webhook event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiWebhookEvent {
    Push,
    PullRequest,
    Tag,
    Release,
    Schedule,
}

/// CI/CD integration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiIntegrationSettings {
    pub enabled: bool,
    pub provider: CiProvider,
    pub require_status_check: bool,
    pub required_statuses: Vec<String>,
    pub auto_merge_on_pass: bool,
    pub notify_on_failure: bool,
    pub notify_channels: Vec<String>,
}

impl Default for CiIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: CiProvider::GitHubActions,
            require_status_check: true,
            required_statuses: vec!["test".to_string(), "lint".to_string(), "build".to_string()],
            auto_merge_on_pass: false,
            notify_on_failure: true,
            notify_channels: vec!["email".to_string()],
        }
    }
}
