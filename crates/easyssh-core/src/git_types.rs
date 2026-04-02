use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum GitError {
    #[error("Git repository not found: {0}")]
    RepositoryNotFound(String),

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Git operation failed: {0}")]
    OperationFailed(String),

    #[error("Invalid reference: {0}")]
    InvalidReference(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Remote error: {0}")]
    RemoteError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Submodule error: {0}")]
    SubmoduleError(String),

    #[error("Stash error: {0}")]
    StashError(String),

    #[error("Tag error: {0}")]
    TagError(String),

    #[error("Conflict resolution required")]
    ConflictResolutionRequired,

    #[error("Nothing to commit")]
    NothingToCommit,

    #[error("Uncommitted changes")]
    UncommittedChanges,
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        let msg = e.message().to_string();
        match e.class() {
            git2::ErrorClass::Repository => GitError::RepositoryNotFound(msg),
            git2::ErrorClass::Reference => GitError::InvalidReference(msg),
            git2::ErrorClass::Merge => GitError::MergeConflict(msg),
            git2::ErrorClass::Config => GitError::ConfigError(msg),
            git2::ErrorClass::Submodule => GitError::SubmoduleError(msg),
            git2::ErrorClass::Http | git2::ErrorClass::Net => GitError::NetworkError(msg),
            git2::ErrorClass::Ssh => GitError::AuthFailed,
            _ => GitError::OperationFailed(msg),
        }
    }
}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError::IoError(e.to_string())
    }
}

/// Git signature (author/committer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSignature {
    pub name: String,
    pub email: String,
    pub timestamp: DateTime<Utc>,
}

impl From<git2::Signature<'_>> for GitSignature {
    fn from(sig: git2::Signature<'_>) -> Self {
        let time = sig.when();
        let timestamp = DateTime::from_timestamp(time.seconds(), 0).unwrap_or_else(|| Utc::now());

        Self {
            name: sig.name().unwrap_or("Unknown").to_string(),
            email: sig.email().unwrap_or("unknown@example.com").to_string(),
            timestamp,
        }
    }
}

/// Repository status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStatus {
    pub is_bare: bool,
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub has_changes: bool,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub conflicted_count: usize,
    pub stashed_count: usize,
    pub last_commit: Option<String>,
    pub state: RepoState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepoState {
    Clean,
    Merge,
    Rebase,
    Revert,
    CherryPick,
    Bisect,
    ApplyMailbox,
}

impl From<git2::RepositoryState> for RepoState {
    fn from(state: git2::RepositoryState) -> Self {
        match state {
            git2::RepositoryState::Clean => RepoState::Clean,
            git2::RepositoryState::Merge => RepoState::Merge,
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge => RepoState::Rebase,
            git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => {
                RepoState::Revert
            }
            git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
                RepoState::CherryPick
            }
            git2::RepositoryState::Bisect => RepoState::Bisect,
            git2::RepositoryState::ApplyMailbox | git2::RepositoryState::ApplyMailboxOrRebase => {
                RepoState::ApplyMailbox
            }
        }
    }
}

/// File status in working directory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Unmodified,
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Ignored,
    Untracked,
    TypeChange,
    Conflict,
}

impl From<git2::Status> for FileStatus {
    fn from(status: git2::Status) -> Self {
        if status.is_conflicted() {
            FileStatus::Conflict
        } else if status.is_wt_new() || status.is_index_new() {
            FileStatus::Added
        } else if status.is_wt_deleted() || status.is_index_deleted() {
            FileStatus::Deleted
        } else if status.is_wt_renamed() || status.is_index_renamed() {
            FileStatus::Renamed
        } else if status.is_ignored() {
            FileStatus::Ignored
        } else if status.is_wt_new() {
            FileStatus::Untracked
        } else if status.is_wt_typechange() || status.is_index_typechange() {
            FileStatus::TypeChange
        } else {
            FileStatus::Modified
        }
    }
}

/// File entry with status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub status: FileStatus,
    pub staged: bool,
    pub old_path: Option<String>,
    pub similarity: u32,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub summary: String,
    pub author: GitSignature,
    pub committer: GitSignature,
    pub parents: Vec<String>,
    pub tree_id: String,
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub last_commit: Option<String>,
    pub is_remote: bool,
}

/// Remote information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

/// Tag information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub name: String,
    pub target: String,
    pub message: Option<String>,
    pub tagger: Option<GitSignature>,
    pub is_annotated: bool,
}

/// Stash entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub id: String,
}

/// Submodule information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: String,
    pub is_initialized: bool,
    pub head_id: Option<String>,
    pub index_id: Option<String>,
    pub workdir_id: Option<String>,
    pub ignore: SubmoduleIgnore,
    pub update: SubmoduleUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmoduleIgnore {
    None,
    Untracked,
    Dirty,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmoduleUpdate {
    Checkout,
    Rebase,
    Merge,
    None,
    Default,
}

/// Diff entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub old_file: Option<String>,
    pub new_file: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<DiffHunk>,
}

/// Diff hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Diff line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

/// Blame line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line_no: usize,
    pub commit_id: String,
    pub author: GitSignature,
    pub summary: String,
    pub content: String,
}

/// Conflict information for 3-way merge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub path: String,
    pub our_id: String,
    pub their_id: String,
    pub ancestor_id: Option<String>,
    pub our_content: Option<String>,
    pub their_content: Option<String>,
    pub ancestor_content: Option<String>,
}

/// Merge result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub success: bool,
    pub conflicts: Vec<ConflictInfo>,
    pub auto_committed: bool,
    pub message: String,
}

/// Clone options
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    pub branch: Option<String>,
    pub depth: Option<usize>,
    pub recursive: bool,
    pub bare: bool,
    pub single_branch: bool,
}

/// Push options
#[derive(Debug, Clone, Default)]
pub struct PushOptions {
    pub remote: String,
    pub force: bool,
    pub set_upstream: bool,
}

/// Fetch options
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    pub remote: String,
    pub prune: bool,
    pub tags: bool,
}

/// Credential type for authentication
#[derive(Debug, Clone)]
pub enum CredentialType {
    SshKey {
        username: String,
        private_key: PathBuf,
        passphrase: Option<String>,
    },
    SshAgent {
        username: String,
    },
    Https {
        username: String,
        password: String,
    },
    Token(String),
}

/// Repository statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStats {
    pub commit_count: usize,
    pub branch_count: usize,
    pub tag_count: usize,
    pub stash_count: usize,
    pub contributor_count: usize,
    pub size_bytes: u64,
}

/// Contributor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorStats {
    pub name: String,
    pub email: String,
    pub commit_count: usize,
    pub additions: usize,
    pub deletions: usize,
}
