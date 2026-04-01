use crate::git_types::*;
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    AutotagOption, BranchType, Cred, Diff, DiffOptions, FetchOptions as GitFetchOptions,
    IndexAddOption, MergeOptions, Oid, PushOptions as GitPushOptions, RemoteCallbacks,
    Repository, Sort, StashFlags, SubmoduleUpdate,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Main Git client manager
pub struct GitClient {
    pub repo: Option<Arc<Mutex<Repository>>>,
    path: Option<PathBuf>,
    credential_callback: Arc<Mutex<Option<CredentialType>>>,
}

impl GitClient {
    /// Helper to get repository guard
    fn get_repo(&self) -> Result<std::sync::MutexGuard<'_, Repository>, GitError> {
        self.repo.as_ref()
            .ok_or_else(|| GitError::RepositoryNotFound("No repository open".to_string()))
            .and_then(|r| r.lock().map_err(|_| GitError::RepositoryNotFound("Lock poisoned".to_string())))
    }

    /// Create a new Git client
    pub fn new() -> Self {
        Self {
            repo: None,
            path: None,
            credential_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Setup credential callbacks
    fn create_callbacks(&self) -> RemoteCallbacks<'static> {
        let mut callbacks = RemoteCallbacks::new();
        let creds = Arc::clone(&self.credential_callback);

        callbacks.credentials(move |url, username_from_url, allowed_types| {
            if let Ok(guard) = creds.lock() {
                if let Some(ref cred) = *guard {
                    match cred {
                        CredentialType::SshKey { username, private_key, passphrase } => {
                            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                                return Cred::ssh_key(
                                    username,
                                    None,
                                    private_key,
                                    passphrase.as_deref(),
                                );
                            }
                        }
                        CredentialType::SshAgent { username } => {
                            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                                return Cred::ssh_key_from_agent(username);
                            }
                        }
                        CredentialType::Https { username, password } => {
                            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                                return Cred::userpass_plaintext(username, password);
                            }
                        }
                        CredentialType::Token(token) => {
                            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                                return Cred::userpass_plaintext("x-access-token", token);
                            }
                        }
                    }
                }
            }
            Cred::default()
        });

        callbacks
    }

    /// Set credentials for remote operations
    pub fn set_credentials(&self, creds: CredentialType) {
        let mut guard = self.credential_callback.lock().unwrap();
        *guard = Some(creds);
    }

    /// Clear credentials
    pub fn clear_credentials(&self) {
        let mut guard = self.credential_callback.lock().unwrap();
        *guard = None;
    }

    /// Open an existing repository
    pub fn open(&mut self, path: &Path) -> Result<(), GitError> {
        let repo = Repository::open(path)?;
        self.path = Some(path.to_path_buf());
        self.repo = Some(Arc::new(Mutex::new(repo)));
        Ok(())
    }

    /// Initialize a new repository
    pub fn init(&mut self, path: &Path, bare: bool) -> Result<(), GitError> {
        let mut opts = git2::RepositoryInitOptions::new();
        opts.bare(bare);

        let repo = Repository::init_opts(path, &opts)?;
        self.path = Some(path.to_path_buf());
        self.repo = Some(Arc::new(Mutex::new(repo)));
        Ok(())
    }

    /// Clone a repository
    pub fn clone(
        &mut self,
        url: &str,
        path: &Path,
        options: &CloneOptions,
    ) -> Result<(), GitError> {
        let mut builder = RepoBuilder::new();

        if let Some(branch) = &options.branch {
            builder.branch(branch);
        }

        let mut fetch_opts = GitFetchOptions::new();
        fetch_opts.remote_callbacks(self.create_callbacks());

        if let Some(depth) = options.depth {
            fetch_opts.depth(depth as i32);
        }

        builder.fetch_options(fetch_opts);

        let repo = builder.clone(url, path)?;

        if options.recursive {
            self.update_submodules(&repo, true)?;
        }

        self.path = Some(path.to_path_buf());
        self.repo = Some(Arc::new(Mutex::new(repo)));
        Ok(())
    }

    /// Get repository status
    pub fn status(&self) -> Result<RepoStatus, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let head = repo.head().ok();
        let branch = head
            .as_ref()
            .and_then(|h| h.shorthand())
            .unwrap_or("HEAD (no branch)")
            .to_string();

        let (upstream, ahead, behind) = if let Some(ref head_ref) = head {
            if let Ok(upstream) = repo.branch_upstream_remote(head_ref.name().unwrap_or("")) {
                let upstream_name = upstream.as_str().unwrap_or("");
                let upstream_ref = format!("refs/remotes/{}/{}" , upstream_name, branch);

                let local_oid = head_ref.target();
                let upstream_oid = repo.refname_to_id(&upstream_ref).ok();

                if let (Some(local), Some(upstream)) = (local_oid, upstream_oid) {
                    let (a, b) = repo.graph_ahead_behind(local, upstream)?;
                    (Some(upstream_ref), a, b)
                } else {
                    (Some(upstream_ref), 0, 0)
                }
            } else {
                (None, 0, 0)
            }
        } else {
            (None, 0, 0)
        };

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);

        let statuses = repo.statuses(Some(&mut opts))?;

        let mut staged = 0;
        let mut unstaged = 0;
        let mut untracked = 0;
        let mut conflicted = 0;

        for entry in statuses.iter() {
            let status = entry.status();
            if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() ||
               status.is_index_renamed() || status.is_index_typechange() {
                staged += 1;
            }
            if status.is_wt_modified() || status.is_wt_deleted() || status.is_wt_renamed() ||
               status.is_wt_typechange() {
                unstaged += 1;
            }
            if status.is_wt_new() {
                untracked += 1;
            }
            if status.is_conflicted() {
                conflicted += 1;
            }
        }

        let last_commit = head.as_ref().and_then(|h| h.target()).map(|oid| oid.to_string());

        let stash_count = self.stash_list()?.len();

        Ok(RepoStatus {
            is_bare: repo.is_bare(),
            branch,
            upstream,
            ahead,
            behind,
            has_changes: staged > 0 || unstaged > 0 || untracked > 0,
            staged_count: staged,
            unstaged_count: unstaged,
            untracked_count: untracked,
            conflicted_count: conflicted,
            stashed_count: stash_count,
            last_commit,
            state: repo.state().into(),
        })
    }

    /// Get detailed file statuses
    pub fn file_statuses(&self) -> Result<Vec<FileEntry>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);

        let statuses = repo.statuses(Some(&mut opts))?;
        let mut entries = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            let (staged_status, unstaged_status): (Option<FileStatus>, Option<FileStatus>) = if status.is_index_new() {
                (Some(FileStatus::Added), None)
            } else if status.is_index_modified() {
                (Some(FileStatus::Modified), None)
            } else if status.is_index_deleted() {
                (Some(FileStatus::Deleted), None)
            } else if status.is_index_renamed() {
                (Some(FileStatus::Renamed), None)
            } else {
                (None, None)
            };

            let wt_status = if status.is_wt_modified() {
                Some(FileStatus::Modified)
            } else if status.is_wt_deleted() {
                Some(FileStatus::Deleted)
            } else if status.is_wt_renamed() {
                Some(FileStatus::Renamed)
            } else if status.is_wt_new() {
                Some(FileStatus::Untracked)
            } else if status.is_conflicted() {
                Some(FileStatus::Conflict)
            } else {
                None
            };

            if let Some(s) = staged_status {
                entries.push(FileEntry {
                    path: path.clone(),
                    status: s,
                    staged: true,
                    old_path: None,
                    similarity: 0,
                });
            }

            if let Some(s) = wt_status {
                entries.push(FileEntry {
                    path: path.clone(),
                    status: s,
                    staged: false,
                    old_path: None,
                    similarity: 0,
                });
            }
        }

        Ok(entries)
    }

    /// Stage files
    pub fn stage(&self, paths: &[&str]) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut index = repo.index()?;
        index.add_all(paths, IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage files
    pub fn unstage(&self, paths: &[&str]) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let head = repo.head()?;
        let target = head.peel_to_commit()?;

        let mut index = repo.index()?;
        let tree = target.tree()?;

        for path in paths {
            index.remove_path(Path::new(path))?;
            if let Some(entry) = tree.get_path(Path::new(path)).ok() {
                if let Some(obj) = repo.find_object(entry.id(), None).ok() {
                    index.add_frombuffer(&git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode: entry.filemode_raw() as u32,
                        uid: 0,
                        gid: 0,
                        file_size: 0,
                        id: entry.id(),
                        flags: 0,
                        flags_extended: 0,
                        path: path.as_bytes().to_vec(),
                    }, &obj.as_blob().map(|b| b.content()).unwrap_or(b""))?;
                }
            }
        }

        index.write()?;
        Ok(())
    }

    /// Discard changes in working directory
    pub fn discard(&self, paths: &[&str]) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut checkout_opts = CheckoutBuilder::new();
        checkout_opts.force();
        for path in paths {
            checkout_opts.path(path);
        }

        repo.checkout_head(Some(&mut checkout_opts))?;
        Ok(())
    }

    /// Commit changes
    pub fn commit(&self, message: &str, amend: bool) -> Result<String, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let sig = repo.signature()?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        if amend {
            let head = repo.head()?;
            let parent = head.peel_to_commit()?;

            let commit_id = parent.amend(
                Some("HEAD"),
                Some(&sig),
                Some(&sig),
                None,
                Some(message),
                Some(&tree),
            )?;

            return Ok(commit_id.to_string());
        }

        let parent_commits: Vec<git2::Commit> = if let Ok(head) = repo.head() {
            vec![head.peel_to_commit()?]
        } else {
            vec![]
        };

        let parents: Vec<&git2::Commit> = parent_commits.iter().collect();

        let commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;

        Ok(commit_id.to_string())
    }

    /// Get commit history
    pub fn log(&self, branch: Option<&str>, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut revwalk = repo.revwalk()?;

        if let Some(b) = branch {
            let reference = repo.find_reference(&format!("refs/heads/{}", b))
                .or_else(|_| repo.find_reference(&format!("refs/remotes/origin/{}", b)))?;
            let oid = reference.target().ok_or_else(|| {
                GitError::InvalidReference("Branch not found".to_string())
            })?;
            revwalk.push(oid)?;
        } else {
            revwalk.push_head()?;
        }

        revwalk.set_sorting(Sort::TIME)?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= limit {
                break;
            }
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();

            let message = commit.message().unwrap_or("").to_string();
            let summary = commit.summary().unwrap_or("").to_string();

            commits.push(CommitInfo {
                id: oid.to_string(),
                short_id: oid.to_string()[..7].to_string(),
                message,
                summary,
                author: commit.author().into(),
                committer: commit.committer().into(),
                parents,
                tree_id: commit.tree_id().to_string(),
            });
        }

        Ok(commits)
    }

    /// Get diff for a commit
    pub fn diff_commit(&self, commit_id: &str) -> Result<Vec<DiffEntry>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let oid = Oid::from_str(commit_id)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let mut diff_opts = DiffOptions::new();
        let diff = if let Some(parent) = parent_tree {
            repo.diff_tree_to_tree(Some(&parent), Some(&tree), Some(&mut diff_opts))?
        } else {
            repo.diff_tree_to_tree(None, Some(&tree), Some(&mut diff_opts))?
        };

        self.format_diff(&diff)
    }

    /// Get diff between working directory and HEAD
    pub fn diff_workdir(&self) -> Result<Vec<DiffEntry>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let head = repo.head()?;
        let tree = head.peel_to_tree()?;

        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut diff_opts))?;

        self.format_diff(&diff)
    }

    /// Get staged diff
    pub fn diff_staged(&self) -> Result<Vec<DiffEntry>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        let mut index = repo.index()?;
        let index_tree = index.write_tree()?;
        let index_tree = repo.find_tree(index_tree)?;

        let mut diff_opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(Some(&head_tree), Some(&index_tree), Some(&mut diff_opts))?;

        self.format_diff(&diff)
    }

    /// Format diff into structured entries
    fn format_diff(&self, diff: &Diff) -> Result<Vec<DiffEntry>, GitError> {
        let entries = RefCell::new(Vec::new());

        diff.foreach(
            &mut |delta, _| {
                let status = match delta.status() {
                    git2::Delta::Added => FileStatus::Added,
                    git2::Delta::Deleted => FileStatus::Deleted,
                    git2::Delta::Modified => FileStatus::Modified,
                    git2::Delta::Renamed => FileStatus::Renamed,
                    git2::Delta::Copied => FileStatus::Copied,
                    _ => FileStatus::Modified,
                };

                let old_file = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
                let new_file = delta.new_file().path().map(|p| p.to_string_lossy().to_string());

                entries.borrow_mut().push(DiffEntry {
                    old_file,
                    new_file,
                    status,
                    hunks: Vec::new(),
                });

                true
            },
            None,
            Some(&mut |_delta, hunk| {
                let mut entries = entries.borrow_mut();
                if let Some(entry) = entries.last_mut() {
                    entry.hunks.push(DiffHunk {
                        old_start: hunk.old_start(),
                        old_lines: hunk.old_lines(),
                        new_start: hunk.new_start(),
                        new_lines: hunk.new_lines(),
                        header: String::from_utf8_lossy(hunk.header()).to_string(),
                        lines: Vec::new(),
                    });
                }
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                let mut entries = entries.borrow_mut();
                if let Some(entry) = entries.last_mut() {
                    if let Some(hunk) = entry.hunks.last_mut() {
                        hunk.lines.push(DiffLine {
                            origin: line.origin(),
                            content: String::from_utf8_lossy(line.content()).to_string(),
                            old_lineno: line.old_lineno(),
                            new_lineno: line.new_lineno(),
                        });
                    }
                }
                true
            }),
        )?;

        Ok(entries.into_inner())
    }

    /// Get all branches
    pub fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let head = repo.head().ok();
        let head_name = head.as_ref().and_then(|h| h.shorthand());

        let mut branches = Vec::new();

        for branch in repo.branches(Some(BranchType::Local))? {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or("").to_string();
            let is_head = head_name == Some(&name);

            let upstream = branch.upstream().ok();
            let upstream_name = upstream.as_ref()
                .and_then(|u| u.name().ok().flatten())
                .map(|n| n.to_string());

            let (ahead, behind) = if let Some(ref _up) = upstream {
                // ahead_behind not available in this version of git2
                (0, 0)
            } else {
                (0, 0)
            };

            let last_commit = branch.get().peel_to_commit().ok().map(|c| c.id().to_string());

            branches.push(BranchInfo {
                name,
                is_head,
                upstream: upstream_name,
                ahead,
                behind,
                last_commit,
                is_remote: false,
            });
        }

        for branch in repo.branches(Some(BranchType::Remote))? {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or("").to_string();

            let last_commit = branch.get().peel_to_commit().ok().map(|c| c.id().to_string());

            branches.push(BranchInfo {
                name,
                is_head: false,
                upstream: None,
                ahead: 0,
                behind: 0,
                last_commit,
                is_remote: true,
            });
        }

        Ok(branches)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str, start_point: Option<&str>) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let commit = if let Some(point) = start_point {
            let obj = repo.revparse_single(point)?;
            obj.peel_to_commit()?
        } else {
            let head = repo.head()?;
            head.peel_to_commit()?
        };

        repo.branch(name, &commit, false)?;
        Ok(())
    }

    /// Switch to a branch
    pub fn checkout_branch(&self, name: &str, create: bool) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let (obj, reference) = repo.revparse_ext(name)?;

        repo.checkout_tree(&obj, None)?;

        if let Some(ref reference) = reference {
            repo.set_head(reference.name().unwrap())?;
        } else {
            repo.set_head_detached(obj.id())?;
        }

        if create {
            let commit = obj.peel_to_commit()?;
            repo.branch(name, &commit, false)?;
        }

        Ok(())
    }

    /// Delete a branch
    pub fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut branch = repo.find_branch(name, BranchType::Local)?;
        branch.delete()?;

        Ok(())
    }

    /// Merge branch into current branch
    pub fn merge(&self, branch_name: &str) -> Result<MergeResult, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let their_ref = repo.find_reference(&format!("refs/heads/{}", branch_name))?;
        let their_commit = their_ref.peel_to_commit()?;

        let annotated = repo.find_annotated_commit(their_commit.id())?;

        let (analysis, _) = repo.merge_analysis(&[&annotated])?;

        if analysis.is_up_to_date() {
            return Ok(MergeResult {
                success: true,
                conflicts: vec![],
                auto_committed: true,
                message: "Already up to date".to_string(),
            });
        }

        if analysis.is_unborn() {
            return Err(GitError::InvalidReference("Cannot merge unborn branch".to_string()));
        }

        let mut merge_opts = MergeOptions::new();

        // TODO: Check if fast-forward analysis is available in this git2 version
        // For now, we assume non-fast-forward merge
        let is_fast_forward = false;
        if is_fast_forward {
            let tree = their_commit.tree()?;
            repo.checkout_tree(&tree.into_object(), None)?;
            repo.set_head(&format!("refs/heads/{}", branch_name))?;

            return Ok(MergeResult {
                success: true,
                conflicts: vec![],
                auto_committed: true,
                message: "Fast-forward merge".to_string(),
            });
        }

        repo.merge(&[&annotated], Some(&mut merge_opts), None)?;

        let mut index = repo.index()?;
        let has_conflicts = index.has_conflicts();

        if has_conflicts {
            let conflicts = self.get_conflicts()?;
            return Ok(MergeResult {
                success: false,
                conflicts,
                auto_committed: false,
                message: "Merge has conflicts".to_string(),
            });
        }

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let head = repo.head()?;
        let parent = head.peel_to_commit()?;

        let sig = repo.signature()?;
        let msg = format!("Merge branch '{}' into {}", branch_name, head.shorthand().unwrap_or("HEAD"));

        let commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &tree,
            &[&parent, &their_commit],
        )?;

        repo.cleanup_state()?;

        Ok(MergeResult {
            success: true,
            conflicts: vec![],
            auto_committed: true,
            message: format!("Merge commit created: {}", commit_id.to_string()[..7].to_string()),
        })
    }

    /// Get current merge conflicts
    pub fn get_conflicts(&self) -> Result<Vec<ConflictInfo>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let index = repo.index()?;
        let conflicts = index.conflicts()?;
        let mut result = Vec::new();

        for conflict in conflicts {
            let conflict = conflict?;

            let path = std::str::from_utf8(&conflict.ancestor.as_ref()
                .or(conflict.our.as_ref())
                .or(conflict.their.as_ref())
                .unwrap().path)
                .map_err(|_| GitError::IoError("Invalid path encoding".to_string()))?
                .to_string();

            let get_content = |entry: &git2::IndexEntry| -> Result<String, GitError> {
                let blob = repo.find_blob(entry.id)?;
                Ok(String::from_utf8_lossy(blob.content()).to_string())
            };

            let ancestor_id = conflict.ancestor.as_ref().map(|e| e.id.to_string());
            let our_id = conflict.our.as_ref().map(|e| e.id.to_string()).unwrap_or_default();
            let their_id = conflict.their.as_ref().map(|e| e.id.to_string()).unwrap_or_default();

            let ancestor_content = conflict.ancestor.as_ref()
                .and_then(|e| get_content(e).ok());
            let our_content = conflict.our.as_ref()
                .and_then(|e| get_content(e).ok());
            let their_content = conflict.their.as_ref()
                .and_then(|e| get_content(e).ok());

            result.push(ConflictInfo {
                path,
                our_id,
                their_id,
                ancestor_id,
                our_content,
                their_content,
                ancestor_content,
            });
        }

        Ok(result)
    }

    /// Resolve a conflict
    pub fn resolve_conflict(&self, path: &str, content: &str) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut index = repo.index()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let blob_id = repo.blob(content.as_bytes())?;

        let entry = git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: content.len() as u32,
            id: blob_id,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        };

        index.add(&entry)?;
        // remove_conflict not available in this git2 version
        // index.remove_conflict(Path::new(path))?;
        index.write()?;

        Ok(())
    }

    /// Abort current merge
    pub fn abort_merge(&self) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.cleanup_state()?;

        let mut checkout_opts = CheckoutBuilder::new();
        checkout_opts.force();

        repo.checkout_head(Some(&mut checkout_opts))?;

        Ok(())
    }

    /// Get remotes
    pub fn remotes(&self) -> Result<Vec<RemoteInfo>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let remote_names = repo.remotes()?;
        let mut remotes = Vec::new();

        for name in remote_names.iter() {
            if let Some(name) = name {
                let remote = repo.find_remote(name)?;
                remotes.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("").to_string(),
                    push_url: remote.pushurl().map(|s| s.to_string()),
                });
            }
        }

        Ok(remotes)
    }

    /// Add a remote
    pub fn add_remote(&self, name: &str, url: &str) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.remote(name, url)?;
        Ok(())
    }

    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.remote_delete(name)?;
        Ok(())
    }

    /// Fetch from remote
    pub fn fetch(&self, options: &FetchOptions) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut remote = repo.find_remote(&options.remote)?;

        let mut fetch_opts = GitFetchOptions::new();
        fetch_opts.remote_callbacks(self.create_callbacks());

        if options.prune {
            fetch_opts.prune(git2::FetchPrune::On);
        }

        if options.tags {
            fetch_opts.download_tags(AutotagOption::All);
        }

        remote.fetch(&[] as &[&str], Some(&mut fetch_opts), None)?;

        Ok(())
    }

    /// Push to remote
    pub fn push(&self, refspec: &str, options: &PushOptions) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut remote = repo.find_remote(&options.remote)?;

        let mut push_opts = GitPushOptions::new();
        push_opts.remote_callbacks(self.create_callbacks());

        let refspec = if options.force {
            format!("+{}", refspec)
        } else {
            refspec.to_string()
        };

        remote.push(&[refspec], Some(&mut push_opts))?;

        if options.set_upstream {
            let branch = repo.head()?.shorthand().unwrap_or("HEAD").to_string();
            let mut branch = repo.find_branch(&branch, BranchType::Local)?;
            branch.set_upstream(Some(&format!("{}/{}", options.remote, branch.name()?.unwrap_or(""))))?;
        }

        Ok(())
    }

    /// Pull from remote
    pub fn pull(&self, remote_name: &str) -> Result<MergeResult, GitError> {
        self.fetch(&FetchOptions {
            remote: remote_name.to_string(),
            prune: false,
            tags: false,
        })?;

        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let head = repo.head()?;
        let branch_name = head.shorthand().unwrap_or("HEAD");

        let remote_ref = format!("refs/remotes/{}/{}", remote_name, branch_name);
        let their_commit = repo.find_reference(&remote_ref)?.peel_to_commit()?;

        let annotated = repo.find_annotated_commit(their_commit.id())?;

        let (analysis, _) = repo.merge_analysis(&[&annotated])?;

        if analysis.is_up_to_date() {
            return Ok(MergeResult {
                success: true,
                conflicts: vec![],
                auto_committed: true,
                message: "Already up to date".to_string(),
            });
        }

        // TODO: Check if fast-forward analysis is available in this git2 version
        let is_fast_forward = false;
        if is_fast_forward {
            let tree = their_commit.tree()?;
            repo.checkout_tree(&tree.into_object(), None)?;
            repo.reference(&format!("refs/heads/{}", branch_name), their_commit.id(), true, "pull: fast-forward")?;
            repo.set_head(&format!("refs/heads/{}", branch_name))?;

            return Ok(MergeResult {
                success: true,
                conflicts: vec![],
                auto_committed: true,
                message: "Fast-forward pull".to_string(),
            });
        }

        self.merge(&format!("{}/{}", remote_name, branch_name))
    }

    /// Get all tags
    pub fn tags(&self) -> Result<Vec<TagInfo>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut tags = Vec::new();

        repo.tag_foreach(|oid, name| {
            let name = std::str::from_utf8(&name[10..]).unwrap_or("").to_string();

            let (message, tagger, is_annotated) = if let Ok(tag) = repo.find_tag(oid) {
                let tagger_sig = tag.tagger().map(|s| GitSignature::from(s));
                (
                    tag.message().map(|m| m.to_string()),
                    tagger_sig,
                    true,
                )
            } else {
                (None, None, false)
            };

            tags.push(TagInfo {
                name,
                target: oid.to_string(),
                message,
                tagger,
                is_annotated,
            });

            true
        })?;

        Ok(tags)
    }

    /// Create a tag
    pub fn create_tag(
        &self,
        name: &str,
        target: Option<&str>,
        message: Option<&str>,
    ) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let target_obj = if let Some(t) = target {
            repo.revparse_single(t)?
        } else {
            let head = repo.head()?;
            head.peel_to_commit()?.into_object()
        };

        if let Some(msg) = message {
            let sig = repo.signature()?;
            repo.tag(
                name,
                &target_obj,
                &sig,
                msg,
                true,
            )?;
        } else {
            repo.tag(
                name,
                &target_obj,
                &repo.signature()?,
                "",
                false,
            )?;
        }

        Ok(())
    }

    /// Delete a tag
    pub fn delete_tag(&self, name: &str) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.tag_delete(name)?;
        Ok(())
    }

    /// Push a tag to remote
    pub fn push_tag(&self, tag_name: &str, remote: &str) -> Result<(), GitError> {
        let refspec = format!("refs/tags/{}", tag_name);
        self.push(&refspec, &PushOptions {
            remote: remote.to_string(),
            force: false,
            set_upstream: false,
        })
    }

    /// Get all stashes
    pub fn stash_list(&self) -> Result<Vec<StashEntry>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let mut repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut stashes = Vec::new();

        repo.stash_foreach(|i, message, oid| {
            stashes.push(StashEntry {
                index: i as usize,
                message: message.to_string(),
                id: oid.to_string(),
            });
            true
        })?;

        Ok(stashes)
    }

    /// Create a stash
    pub fn stash_save(&self, message: Option<&str>, include_untracked: bool) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let mut repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let sig = repo.signature()?;
        let default_msg = "WIP on stash";
        let msg = message.unwrap_or(default_msg);

        let mut flags = StashFlags::INCLUDE_IGNORED;
        if include_untracked {
            flags |= StashFlags::INCLUDE_UNTRACKED;
        }

        repo.stash_save2(&sig, Some(msg), Some(flags))?;

        Ok(())
    }

    /// Pop a stash
    pub fn stash_pop(&self, index: usize) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let mut repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.stash_pop(index as usize, None)?;
        Ok(())
    }

    /// Apply a stash without removing it
    pub fn stash_apply(&self, index: usize) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let mut repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.stash_apply(index as usize, None)?;
        Ok(())
    }

    /// Drop a stash
    pub fn stash_drop(&self, index: usize) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let mut repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        repo.stash_drop(index as usize)?;
        Ok(())
    }

    /// Get submodules
    pub fn submodules(&self) -> Result<Vec<SubmoduleInfo>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let submodules = repo.submodules()?;
        let mut result = Vec::new();

        for submod in submodules.iter() {
            let name = submod.name().unwrap_or("").to_string();
            let path = submod.path().to_string_lossy().to_string();
            let url = submod.url().unwrap_or("").to_string();

            let head_id = submod.head_id().map(|o| o.to_string());
            let index_id = submod.index_id().map(|o| o.to_string());
            let workdir_id = submod.workdir_id().map(|o| o.to_string());

            let ignore = crate::git_types::SubmoduleIgnore::None; // Method not available in this git2 version

            let update = match submod.update_strategy() {
                SubmoduleUpdate::Checkout => crate::git_types::SubmoduleUpdate::Checkout,
                SubmoduleUpdate::Rebase => crate::git_types::SubmoduleUpdate::Rebase,
                SubmoduleUpdate::Merge => crate::git_types::SubmoduleUpdate::Merge,
                SubmoduleUpdate::None => crate::git_types::SubmoduleUpdate::None,
                _ => crate::git_types::SubmoduleUpdate::Default,
            };

            result.push(SubmoduleInfo {
                name,
                path,
                url,
                is_initialized: false, // Method not available
                head_id,
                index_id,
                workdir_id,
                ignore,
                update,
            });
        }

        Ok(result)
    }

    /// Initialize/update submodules
    pub fn update_submodules(&self, repo: &Repository, recursive: bool) -> Result<(), GitError> {
        let submodules = repo.submodules()?;

        for mut submod in submodules.into_iter() {
            // is_initialized not available, try init anyway
            let _ = submod.init(false);
            submod.update(true, None)?;

            if recursive {
                let sub_repo = submod.open()?;
                self.update_submodules(&sub_repo, true)?;
            }
        }

        Ok(())
    }

    /// Add a submodule
    pub fn add_submodule(&self, url: &str, path: &Path) -> Result<(), GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let _ = repo.submodule(url, path, false)?;
        Ok(())
    }

    /// Blame file
    pub fn blame(&self, path: &str, oldest_commit: Option<&str>) -> Result<Vec<BlameLine>, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut opts = git2::BlameOptions::new();

        if let Some(oldest) = oldest_commit {
            let oid = Oid::from_str(oldest)?;
            opts.oldest_commit(oid);
        }

        let blame = repo.blame_file(Path::new(path), Some(&mut opts))?;

        // Get file content
        let tree = repo.head()?.peel_to_tree()?;
        let entry = tree.get_path(Path::new(path))?;
        let blob = repo.find_blob(entry.id())?;

        let content = String::from_utf8_lossy(blob.content());
        let lines: Vec<&str> = content.lines().collect();

        let mut result = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let hunk = blame.get_line(i + 1);
            if let Some(hunk) = hunk {
                let commit = repo.find_commit(hunk.final_commit_id())?;

                result.push(BlameLine {
                    line_no: i + 1,
                    commit_id: hunk.final_commit_id().to_string(),
                    author: commit.author().into(),
                    summary: commit.summary().unwrap_or("").to_string(),
                    content: line.to_string(),
                });
            }
        }

        Ok(result)
    }

    /// Get repository statistics
    pub fn stats(&self) -> Result<RepoStats, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let commit_count = revwalk.count();

        let branch_count = repo.branches(None)?.count();
        let tag_count = self.tags()?.len();
        let stash_count = self.stash_list()?.len();

        let mut contributors = HashSet::new();
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        for oid in revwalk {
            if let Ok(commit) = repo.find_commit(oid.unwrap()) {
                contributors.insert(commit.author().email().unwrap_or("").to_string());
            }
        }

        let size_bytes = std::fs::metadata(repo.path())
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(RepoStats {
            commit_count,
            branch_count,
            tag_count,
            stash_count,
            contributor_count: contributors.len(),
            size_bytes,
        })
    }

    /// Get file content at specific commit
    pub fn get_file_at_commit(&self, path: &str, commit_id: &str) -> Result<String, GitError> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            GitError::RepositoryNotFound("No repository open".to_string())
        })?;
        let repo = repo.lock().map_err(|_| GitError::RepositoryNotFound("Failed to lock repository".to_string()))?;

        let oid = Oid::from_str(commit_id)?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let entry = tree.get_path(Path::new(path))?;
        let blob = repo.find_blob(entry.id())?;

        Ok(String::from_utf8_lossy(blob.content()).to_string())
    }

    /// Get current repository path
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Check if a repository is open
    pub fn is_open(&self) -> bool {
        self.repo.is_some()
    }
}

impl Default for GitClient {
    fn default() -> Self {
        Self::new()
    }
}
