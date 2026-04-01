use crate::git_client::GitClient;
use crate::git_types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task;

/// Async Git manager for handling multiple repositories
pub struct GitManager {
    clients: Arc<RwLock<HashMap<String, Arc<Mutex<GitClient>>>>>,
    active_repo: Arc<RwLock<Option<String>>>,
}

impl GitManager {
    /// Create a new Git manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            active_repo: Arc::new(RwLock::new(None)),
        }
    }

    /// Open a repository and add it to the manager
    pub async fn open_repo(&self, path: &Path) -> Result<String, GitError> {
        let path_str = path.to_string_lossy().to_string();

        let path_buf = path.to_path_buf();
        let client = task::spawn_blocking(move || {
            let mut client = GitClient::new();
            client.open(&path_buf)?;
            Ok::<GitClient, GitError>(client)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))??;

        let id = uuid::Uuid::new_v4().to_string();
        let mut clients = self.clients.write().await;
        clients.insert(id.clone(), Arc::new(Mutex::new(client)));

        let mut active = self.active_repo.write().await;
        *active = Some(id.clone());

        Ok(id)
    }

    /// Clone a repository
    pub async fn clone_repo(
        &self,
        url: &str,
        path: &Path,
        options: &CloneOptions,
        credentials: Option<CredentialType>,
    ) -> Result<String, GitError> {
        let url = url.to_string();
        let path_buf = path.to_path_buf();
        let options = options.clone();
        let creds = credentials;

        let client = task::spawn_blocking(move || {
            let mut client = GitClient::new();
            if let Some(c) = creds {
                client.set_credentials(c);
            }
            client.clone(&url, &path_buf, &options)?;
            Ok::<GitClient, GitError>(client)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))??;

        let id = uuid::Uuid::new_v4().to_string();
        let mut clients = self.clients.write().await;
        clients.insert(id.clone(), Arc::new(Mutex::new(client)));

        let mut active = self.active_repo.write().await;
        *active = Some(id.clone());

        Ok(id)
    }

    /// Initialize a new repository
    pub async fn init_repo(&self, path: &Path, bare: bool) -> Result<String, GitError> {
        let path_buf = path.to_path_buf();

        let client = task::spawn_blocking(move || {
            let mut client = GitClient::new();
            client.init(&path_buf, bare)?;
            Ok::<GitClient, GitError>(client)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))??;

        let id = uuid::Uuid::new_v4().to_string();
        let mut clients = self.clients.write().await;
        clients.insert(id.clone(), Arc::new(Mutex::new(client)));

        let mut active = self.active_repo.write().await;
        *active = Some(id.clone());

        Ok(id)
    }

    /// Get the active repository client
    async fn get_client(&self, repo_id: Option<&str>) -> Result<Arc<Mutex<GitClient>>, GitError> {
        let id = if let Some(id) = repo_id {
            id.to_string()
        } else {
            let active = self.active_repo.read().await;
            active
                .as_ref()
                .ok_or_else(|| GitError::RepositoryNotFound("No active repository".to_string()))?
                .clone()
        };

        let clients = self.clients.read().await;
        clients
            .get(&id)
            .cloned()
            .ok_or_else(|| GitError::RepositoryNotFound(format!("Repository {} not found", id)))
    }

    /// Set the active repository
    pub async fn set_active_repo(&self, repo_id: &str) -> Result<(), GitError> {
        let clients = self.clients.read().await;
        if !clients.contains_key(repo_id) {
            return Err(GitError::RepositoryNotFound(format!(
                "Repository {} not found",
                repo_id
            )));
        }

        let mut active = self.active_repo.write().await;
        *active = Some(repo_id.to_string());
        Ok(())
    }

    /// Get repository status
    pub async fn status(&self, repo_id: Option<&str>) -> Result<RepoStatus, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.status()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get file statuses
    pub async fn file_statuses(&self, repo_id: Option<&str>) -> Result<Vec<FileEntry>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.file_statuses()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Stage files
    pub async fn stage(&self, paths: Vec<String>, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            client.stage(&refs)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Unstage files
    pub async fn unstage(&self, paths: Vec<String>, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            client.unstage(&refs)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Discard changes
    pub async fn discard(&self, paths: Vec<String>, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
            client.discard(&refs)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Commit changes
    pub async fn commit(
        &self,
        message: String,
        amend: bool,
        repo_id: Option<&str>,
    ) -> Result<String, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.commit(&message, amend)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get commit log
    pub async fn log(
        &self,
        branch: Option<String>,
        limit: usize,
        repo_id: Option<&str>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.log(branch.as_deref(), limit)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get diff for commit
    pub async fn diff_commit(
        &self,
        commit_id: String,
        repo_id: Option<&str>,
    ) -> Result<Vec<DiffEntry>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.diff_commit(&commit_id)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get working directory diff
    pub async fn diff_workdir(&self, repo_id: Option<&str>) -> Result<Vec<DiffEntry>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.diff_workdir()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get staged diff
    pub async fn diff_staged(&self, repo_id: Option<&str>) -> Result<Vec<DiffEntry>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.diff_staged()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get branches
    pub async fn branches(&self, repo_id: Option<&str>) -> Result<Vec<BranchInfo>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.branches()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Create branch
    pub async fn create_branch(
        &self,
        name: String,
        start_point: Option<String>,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.create_branch(&name, start_point.as_deref())
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Checkout branch
    pub async fn checkout_branch(
        &self,
        name: String,
        create: bool,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.checkout_branch(&name, create)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Delete branch
    pub async fn delete_branch(&self, name: String, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.delete_branch(&name)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Merge branch
    pub async fn merge(
        &self,
        branch_name: String,
        repo_id: Option<&str>,
    ) -> Result<MergeResult, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.merge(&branch_name)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get conflicts
    pub async fn get_conflicts(
        &self,
        repo_id: Option<&str>,
    ) -> Result<Vec<ConflictInfo>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.get_conflicts()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Resolve conflict
    pub async fn resolve_conflict(
        &self,
        path: String,
        content: String,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.resolve_conflict(&path, &content)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Abort merge
    pub async fn abort_merge(&self, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.abort_merge()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get remotes
    pub async fn remotes(&self, repo_id: Option<&str>) -> Result<Vec<RemoteInfo>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.remotes()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Add remote
    pub async fn add_remote(
        &self,
        name: String,
        url: String,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.add_remote(&name, &url)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Remove remote
    pub async fn remove_remote(&self, name: String, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.remove_remote(&name)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Fetch from remote
    pub async fn fetch(
        &self,
        remote: String,
        prune: bool,
        tags: bool,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.fetch(&FetchOptions {
                remote,
                prune,
                tags,
            })
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Pull from remote
    pub async fn pull(
        &self,
        remote: String,
        repo_id: Option<&str>,
    ) -> Result<MergeResult, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.pull(&remote)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Push to remote
    pub async fn push(
        &self,
        refspec: String,
        remote: String,
        force: bool,
        set_upstream: bool,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.push(
                &refspec,
                &PushOptions {
                    remote,
                    force,
                    set_upstream,
                },
            )
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get tags
    pub async fn tags(&self, repo_id: Option<&str>) -> Result<Vec<TagInfo>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.tags()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Create tag
    pub async fn create_tag(
        &self,
        name: String,
        target: Option<String>,
        message: Option<String>,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.create_tag(&name, target.as_deref(), message.as_deref())
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Delete tag
    pub async fn delete_tag(&self, name: String, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.delete_tag(&name)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Push tag
    pub async fn push_tag(
        &self,
        tag_name: String,
        remote: String,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.push_tag(&tag_name, &remote)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get stash list
    pub async fn stash_list(&self, repo_id: Option<&str>) -> Result<Vec<StashEntry>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.stash_list()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Save stash
    pub async fn stash_save(
        &self,
        message: Option<String>,
        include_untracked: bool,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.stash_save(message.as_deref(), include_untracked)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Pop stash
    pub async fn stash_pop(&self, index: usize, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.stash_pop(index)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Apply stash
    pub async fn stash_apply(&self, index: usize, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.stash_apply(index)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Drop stash
    pub async fn stash_drop(&self, index: usize, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.stash_drop(index)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get submodules
    pub async fn submodules(&self, repo_id: Option<&str>) -> Result<Vec<SubmoduleInfo>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.submodules()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Add submodule
    pub async fn add_submodule(
        &self,
        url: String,
        path: PathBuf,
        repo_id: Option<&str>,
    ) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.add_submodule(&url, &path)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Update submodules
    pub async fn update_submodules(&self, repo_id: Option<&str>) -> Result<(), GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            if let Some(ref repo) = client.repo {
                let repo_guard = repo
                    .lock()
                    .map_err(|_| GitError::RepositoryNotFound("Lock poisoned".to_string()))?;
                client.update_submodules(&*repo_guard, true)
            } else {
                Err(GitError::RepositoryNotFound("No repository".to_string()))
            }
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Blame file
    pub async fn blame(
        &self,
        path: String,
        oldest_commit: Option<String>,
        repo_id: Option<&str>,
    ) -> Result<Vec<BlameLine>, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.blame(&path, oldest_commit.as_deref())
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get repository stats
    pub async fn stats(&self, repo_id: Option<&str>) -> Result<RepoStats, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.stats()
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Get file at commit
    pub async fn get_file_at_commit(
        &self,
        path: String,
        commit_id: String,
        repo_id: Option<&str>,
    ) -> Result<String, GitError> {
        let client = self.get_client(repo_id).await?;

        task::spawn_blocking(move || {
            let client = client.blocking_lock();
            client.get_file_at_commit(&path, &commit_id)
        })
        .await
        .map_err(|e| GitError::IoError(e.to_string()))?
    }

    /// Set credentials for a repository
    pub async fn set_credentials(
        &self,
        repo_id: &str,
        creds: CredentialType,
    ) -> Result<(), GitError> {
        let clients = self.clients.read().await;
        let client = clients
            .get(repo_id)
            .ok_or_else(|| {
                GitError::RepositoryNotFound(format!("Repository {} not found", repo_id))
            })?
            .clone();

        let mut client = client.lock().await;
        client.set_credentials(creds);
        Ok(())
    }

    /// List all managed repositories
    pub async fn list_repos(&self) -> Vec<(String, Option<PathBuf>)> {
        let clients = self.clients.read().await;
        let mut result = Vec::new();

        for (id, client) in clients.iter() {
            let client = client.lock().await;
            result.push((id.clone(), client.path().map(|p| p.to_path_buf())));
        }

        result
    }

    /// Close a repository
    pub async fn close_repo(&self, repo_id: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(repo_id);

        let mut active = self.active_repo.write().await;
        if active.as_deref() == Some(repo_id) {
            *active = clients.keys().next().cloned();
        }
    }

    /// Close all repositories
    pub async fn close_all(&self) {
        let mut clients = self.clients.write().await;
        clients.clear();

        let mut active = self.active_repo.write().await;
        *active = None;
    }
}

impl Default for GitManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tauri")]
pub mod tauri_commands {
    use super::*;
    use tauri::State;

    pub struct GitState(pub Arc<GitManager>);

    #[tauri::command]
    pub async fn git_open_repo(path: String, state: State<'_, GitState>) -> Result<String, String> {
        let path = PathBuf::from(path);
        state.0.open_repo(&path).await.map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_clone_repo(
        url: String,
        path: String,
        options: CloneOptions,
        state: State<'_, GitState>,
    ) -> Result<String, String> {
        let path = PathBuf::from(path);
        state
            .0
            .clone_repo(&url, &path, &options, None)
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_init_repo(
        path: String,
        bare: bool,
        state: State<'_, GitState>,
    ) -> Result<String, String> {
        let path = PathBuf::from(path);
        state
            .0
            .init_repo(&path, bare)
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_status(
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<RepoStatus, String> {
        state
            .0
            .status(repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_stage(
        paths: Vec<String>,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<(), String> {
        state
            .0
            .stage(paths, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_unstage(
        paths: Vec<String>,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<(), String> {
        state
            .0
            .unstage(paths, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_commit(
        message: String,
        amend: bool,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<String, String> {
        state
            .0
            .commit(message, amend, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_log(
        branch: Option<String>,
        limit: usize,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<Vec<CommitInfo>, String> {
        state
            .0
            .log(branch, limit, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_branches(
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<Vec<BranchInfo>, String> {
        state
            .0
            .branches(repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_create_branch(
        name: String,
        start_point: Option<String>,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<(), String> {
        state
            .0
            .create_branch(name, start_point, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_checkout_branch(
        name: String,
        create: bool,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<(), String> {
        state
            .0
            .checkout_branch(name, create, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_fetch(
        remote: String,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<(), String> {
        state
            .0
            .fetch(remote, false, false, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_pull(
        remote: String,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<MergeResult, String> {
        state
            .0
            .pull(remote, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn git_push(
        refspec: String,
        remote: String,
        force: bool,
        repo_id: Option<String>,
        state: State<'_, GitState>,
    ) -> Result<(), String> {
        state
            .0
            .push(refspec, remote, force, false, repo_id.as_deref())
            .await
            .map_err(|e| e.to_string())
    }
}
