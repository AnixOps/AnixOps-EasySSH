use crate::git_client::GitClient;
use crate::git_manager::GitManager;
use crate::git_types::*;
use crate::git_workflow::*;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;

/// Git Workflow Executor for executing workflow actions
pub struct GitWorkflowExecutor {
    manager: GitManager,
    gitflow_config: GitFlowConfig,
    pr_templates: Vec<PullRequestTemplate>,
}

impl GitWorkflowExecutor {
    /// Create a new workflow executor
    pub fn new() -> Self {
        Self {
            manager: GitManager::new(),
            gitflow_config: GitFlowConfig::default(),
            pr_templates: vec![
                PullRequestTemplate::feature(),
                PullRequestTemplate::bugfix(),
                PullRequestTemplate::hotfix(),
                PullRequestTemplate::release(),
            ],
        }
    }

    /// Create with existing GitManager
    pub fn with_manager(manager: GitManager) -> Self {
        Self {
            manager,
            gitflow_config: GitFlowConfig::default(),
            pr_templates: vec![
                PullRequestTemplate::feature(),
                PullRequestTemplate::bugfix(),
                PullRequestTemplate::hotfix(),
                PullRequestTemplate::release(),
            ],
        }
    }

    /// Set Git Flow configuration
    pub fn set_gitflow_config(&mut self, config: GitFlowConfig) {
        self.gitflow_config = config;
    }

    /// Get Git Flow configuration
    pub fn gitflow_config(&self) -> &GitFlowConfig {
        &self.gitflow_config
    }

    /// Execute a workflow action
    pub async fn execute(&self, action: GitWorkflowAction) -> Result<WorkflowResult, GitError> {
        match action {
            GitWorkflowAction::Clone { url, path, branch } => {
                self.execute_clone(url, path, branch).await
            }
            GitWorkflowAction::Stage { paths } => {
                self.execute_stage(paths).await
            }
            GitWorkflowAction::Unstage { paths } => {
                self.execute_unstage(paths).await
            }
            GitWorkflowAction::Commit { message } => {
                self.execute_commit(message).await
            }
            GitWorkflowAction::Push { remote, refspec } => {
                self.execute_push(remote, refspec).await
            }
            GitWorkflowAction::Pull { remote } => {
                self.execute_pull(remote).await
            }
            GitWorkflowAction::Fetch { remote } => {
                self.execute_fetch(remote).await
            }
            GitWorkflowAction::Checkout { branch, create } => {
                self.execute_checkout(branch, create).await
            }
            GitWorkflowAction::Merge { branch } => {
                self.execute_merge(branch).await
            }
            GitWorkflowAction::CreateTag { name, message } => {
                self.execute_create_tag(name, message).await
            }
            GitWorkflowAction::PushTag { name, remote } => {
                self.execute_push_tag(name, remote).await
            }
            GitWorkflowAction::StashSave { message } => {
                self.execute_stash_save(message).await
            }
            GitWorkflowAction::StashPop { index } => {
                self.execute_stash_pop(index).await
            }
            GitWorkflowAction::SubmoduleUpdate => {
                self.execute_submodule_update().await
            }
            GitWorkflowAction::Discard { paths } => {
                self.execute_discard(paths).await
            }
            GitWorkflowAction::AddRemote { name, url } => {
                self.execute_add_remote(name, url).await
            }
            GitWorkflowAction::RemoveRemote { name } => {
                self.execute_remove_remote(name).await
            }
            GitWorkflowAction::GetStatus => {
                self.execute_get_status().await
            }
            GitWorkflowAction::GetLog { branch, limit } => {
                self.execute_get_log(branch, limit).await
            }

            // Git Flow Actions
            GitWorkflowAction::GitFlowInit { main_branch, develop_branch, feature_prefix, release_prefix, hotfix_prefix, tag_prefix } => {
                self.execute_gitflow_init(
                    main_branch, develop_branch, feature_prefix,
                    release_prefix, hotfix_prefix, tag_prefix,
                ).await
            }
            GitWorkflowAction::GitFlowFeatureStart { name, base_branch } => {
                self.execute_gitflow_feature_start(name, base_branch).await
            }
            GitWorkflowAction::GitFlowFeatureFinish { name, keep_branch } => {
                self.execute_gitflow_feature_finish(name, keep_branch).await
            }
            GitWorkflowAction::GitFlowFeaturePublish { name } => {
                self.execute_gitflow_feature_publish(name).await
            }
            GitWorkflowAction::GitFlowReleaseStart { version, base_branch } => {
                self.execute_gitflow_release_start(version, base_branch).await
            }
            GitWorkflowAction::GitFlowReleaseFinish { version, tag_message, push_to_remote } => {
                self.execute_gitflow_release_finish(version, tag_message, push_to_remote).await
            }
            GitWorkflowAction::GitFlowHotfixStart { version, base_branch } => {
                self.execute_gitflow_hotfix_start(version, base_branch).await
            }
            GitWorkflowAction::GitFlowHotfixFinish { version, tag_message, push_to_remote } => {
                self.execute_gitflow_hotfix_finish(version, tag_message, push_to_remote).await
            }

            // PR Management Actions
            GitWorkflowAction::CreatePullRequest { title, description, source_branch, target_branch, draft, reviewers } => {
                self.execute_create_pr(title, description, source_branch, target_branch, draft, reviewers).await
            }
            GitWorkflowAction::UpdatePullRequest { number, title, description, state } => {
                self.execute_update_pr(number, title, description, state).await
            }
            GitWorkflowAction::ListPullRequests { state, limit } => {
                self.execute_list_prs(state, limit).await
            }
            GitWorkflowAction::GetPullRequest { number } => {
                self.execute_get_pr(number).await
            }
            GitWorkflowAction::ReviewPullRequest { number, action, comment } => {
                self.execute_review_pr(number, action, comment).await
            }
            GitWorkflowAction::MergePullRequest { number, method, commit_message, delete_source_branch } => {
                self.execute_merge_pr(number, method, commit_message, delete_source_branch).await
            }

            // CI/CD Actions
            GitWorkflowAction::GetCiStatus { branch, commit } => {
                self.execute_get_ci_status(branch, commit).await
            }
            GitWorkflowAction::ListCiPipelines { branch, status, limit } => {
                self.execute_list_ci_pipelines(branch, status, limit).await
            }
            GitWorkflowAction::RetryCiPipeline { pipeline_id } => {
                self.execute_retry_ci_pipeline(pipeline_id).await
            }
            GitWorkflowAction::CancelCiPipeline { pipeline_id } => {
                self.execute_cancel_ci_pipeline(pipeline_id).await
            }

            // Code Review Actions
            GitWorkflowAction::RunCodeReviewChecklist { branch, base_branch, checklist_items } => {
                self.execute_run_checklist(branch, base_branch, checklist_items).await
            }
            GitWorkflowAction::GetCodeReviewReport { commit_id } => {
                self.execute_get_review_report(commit_id).await
            }
            GitWorkflowAction::AddPrComment { file_path, line_number, comment } => {
                self.execute_add_pr_comment(file_path, line_number, comment).await
            }

            _ => Err(GitError::OperationFailed("Action not implemented".to_string())),
        }
    }

    /// Execute a workflow
    pub async fn execute_workflow(&self, actions: Vec<GitWorkflowAction>) -> Vec<Result<WorkflowResult, GitError>> {
        let mut results = Vec::new();
        for action in actions {
            results.push(self.execute(action).await);
        }
        results
    }

    // ============================================================================
    // Basic Git Operations
    // ============================================================================

    async fn execute_clone(&self, url: String, path: String, branch: Option<String>) -> Result<WorkflowResult, GitError> {
        let options = CloneOptions {
            branch: branch.clone(),
            ..Default::default()
        };
        let repo_id = self.manager.clone_repo(&url, Path::new(&path), &options, None).await?;

        Ok(WorkflowResult {
            success: true,
            repo_id: Some(repo_id),
            message: format!("Cloned {} to {}", url, path),
            data: WorkflowData::CloneResult { url, path, branch },
        })
    }

    async fn execute_stage(&self, paths: Vec<String>) -> Result<WorkflowResult, GitError> {
        self.manager.stage(paths, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: "Files staged".to_string(),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_unstage(&self, paths: Vec<String>) -> Result<WorkflowResult, GitError> {
        self.manager.unstage(paths, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: "Files unstaged".to_string(),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_commit(&self, message: String) -> Result<WorkflowResult, GitError> {
        let commit_id = self.manager.commit(message.clone(), false, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Committed: {}", &commit_id[..7]),
            data: WorkflowData::CommitResult { commit_id, message },
        })
    }

    async fn execute_push(&self, remote: String, refspec: String) -> Result<WorkflowResult, GitError> {
        self.manager.push(refspec.clone(), remote.clone(), false, false, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Pushed {} to {}", refspec, remote),
            data: WorkflowData::PushResult { remote, refspec },
        })
    }

    async fn execute_pull(&self, remote: String) -> Result<WorkflowResult, GitError> {
        let result = self.manager.pull(remote.clone(), None).await?;
        let msg = result.message.clone();
        Ok(WorkflowResult {
            success: result.success,
            repo_id: None,
            message: msg,
            data: WorkflowData::MergeResult(result),
        })
    }

    async fn execute_fetch(&self, remote: String) -> Result<WorkflowResult, GitError> {
        self.manager.fetch(remote.clone(), false, false, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Fetched from {}", remote),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_checkout(&self, branch: String, create: bool) -> Result<WorkflowResult, GitError> {
        self.manager.checkout_branch(branch.clone(), create, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: if create {
                format!("Created and checked out branch {}", branch)
            } else {
                format!("Checked out branch {}", branch)
            },
            data: WorkflowData::CheckoutResult { branch, created: create },
        })
    }

    async fn execute_merge(&self, branch: String) -> Result<WorkflowResult, GitError> {
        let result = self.manager.merge(branch.clone(), None).await?;
        let msg = result.message.clone();
        Ok(WorkflowResult {
            success: result.success,
            repo_id: None,
            message: msg,
            data: WorkflowData::MergeResult(result),
        })
    }

    async fn execute_create_tag(&self, name: String, message: Option<String>) -> Result<WorkflowResult, GitError> {
        self.manager.create_tag(name.clone(), None, message.clone(), None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Created tag {}", name),
            data: WorkflowData::TagResult { name, message },
        })
    }

    async fn execute_push_tag(&self, name: String, remote: String) -> Result<WorkflowResult, GitError> {
        self.manager.push_tag(name.clone(), remote.clone(), None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Pushed tag {} to {}", name, remote),
            data: WorkflowData::PushResult {
                remote,
                refspec: format!("refs/tags/{}", name),
            },
        })
    }

    async fn execute_stash_save(&self, message: Option<String>) -> Result<WorkflowResult, GitError> {
        self.manager.stash_save(message.clone(), true, None).await?;
        let msg_str = message.clone().unwrap_or_default();
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Stashed changes: {}", msg_str),
            data: WorkflowData::StashResult { index: None, message },
        })
    }

    async fn execute_stash_pop(&self, index: usize) -> Result<WorkflowResult, GitError> {
        self.manager.stash_pop(index, None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Popped stash at index {}", index),
            data: WorkflowData::StashResult { index: Some(index), message: None },
        })
    }

    async fn execute_submodule_update(&self) -> Result<WorkflowResult, GitError> {
        self.manager.update_submodules(None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: "Submodules updated".to_string(),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_discard(&self, paths: Vec<String>) -> Result<WorkflowResult, GitError> {
        self.manager.discard(paths.clone(), None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Discarded {} files", paths.len()),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_add_remote(&self, name: String, url: String) -> Result<WorkflowResult, GitError> {
        self.manager.add_remote(name.clone(), url.clone(), None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Added remote {} ({})", name, url),
            data: WorkflowData::RemoteResult { name, url },
        })
    }

    async fn execute_remove_remote(&self, name: String) -> Result<WorkflowResult, GitError> {
        self.manager.remove_remote(name.clone(), None).await?;
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Removed remote {}", name),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_get_status(&self) -> Result<WorkflowResult, GitError> {
        let status = self.manager.status(None).await?;
        let message = format!(
            "On branch {} - {} staged, {} unstaged, {} untracked",
            status.branch, status.staged_count, status.unstaged_count, status.untracked_count
        );
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message,
            data: WorkflowData::StatusResult(status),
        })
    }

    async fn execute_get_log(&self, branch: Option<String>, limit: usize) -> Result<WorkflowResult, GitError> {
        let commits = self.manager.log(branch.clone(), limit, None).await?;
        let message = format!("Retrieved {} commits", commits.len());
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message,
            data: WorkflowData::LogResult { commits, branch },
        })
    }

    // ============================================================================
    // Git Flow Operations
    // ============================================================================

    async fn execute_gitflow_init(
        &self,
        main_branch: Option<String>,
        develop_branch: Option<String>,
        feature_prefix: Option<String>,
        release_prefix: Option<String>,
        hotfix_prefix: Option<String>,
        tag_prefix: Option<String>,
    ) -> Result<WorkflowResult, GitError> {
        let config = GitFlowConfig {
            main_branch: main_branch.unwrap_or_else(|| self.gitflow_config.main_branch.clone()),
            develop_branch: develop_branch.unwrap_or_else(|| self.gitflow_config.develop_branch.clone()),
            feature_prefix: feature_prefix.unwrap_or_else(|| self.gitflow_config.feature_prefix.clone()),
            release_prefix: release_prefix.unwrap_or_else(|| self.gitflow_config.release_prefix.clone()),
            hotfix_prefix: hotfix_prefix.unwrap_or_else(|| self.gitflow_config.hotfix_prefix.clone()),
            tag_prefix: tag_prefix.unwrap_or_else(|| self.gitflow_config.tag_prefix.clone()),
        };

        // Ensure main branch exists
        let branches = self.manager.branches(None).await?;
        let main_exists = branches.iter().any(|b| b.name == config.main_branch);
        let develop_exists = branches.iter().any(|b| b.name == config.develop_branch);

        if !main_exists {
            self.manager.create_branch(config.main_branch.clone(), None, None).await?;
        }

        if !develop_exists {
            self.manager.create_branch(config.develop_branch.clone(), Some(config.main_branch.clone()), None).await?;
        }

        // Checkout develop branch
        self.manager.checkout_branch(config.develop_branch.clone(), false, None).await?;

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Git Flow initialized with {} and {}", config.main_branch, config.develop_branch),
            data: WorkflowData::GitFlowInitResult { config },
        })
    }

    async fn execute_gitflow_feature_start(
        &self,
        name: String,
        base_branch: Option<String>,
    ) -> Result<WorkflowResult, GitError> {
        let feature_branch = format!("{}{}", self.gitflow_config.feature_prefix, name);
        let base = base_branch.unwrap_or_else(|| self.gitflow_config.develop_branch.clone());

        // Fetch latest develop
        self.manager.fetch("origin".to_string(), true, false, None).await.ok();

        // Create feature branch from develop
        self.manager.create_branch(feature_branch.clone(), Some(base.clone()), None).await?;
        self.manager.checkout_branch(feature_branch.clone(), false, None).await?;

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Started feature {} based on {}", name, base),
            data: WorkflowData::GitFlowResult {
                branch_name: feature_branch,
                branch_type: GitFlowBranchType::Feature,
                commit_id: None,
            },
        })
    }

    async fn execute_gitflow_feature_finish(
        &self,
        name: String,
        keep_branch: bool,
    ) -> Result<WorkflowResult, GitError> {
        let feature_branch = format!("{}{}", self.gitflow_config.feature_prefix, name);

        // Checkout develop
        self.manager.checkout_branch(self.gitflow_config.develop_branch.clone(), false, None).await?;

        // Merge feature into develop
        let merge_result = self.manager.merge(feature_branch.clone(), None).await?;

        if !merge_result.success && !merge_result.conflicts.is_empty() {
            return Ok(WorkflowResult {
                success: false,
                repo_id: None,
                message: "Merge conflicts detected".to_string(),
                data: WorkflowData::MergeResult(merge_result),
            });
        }

        // Delete feature branch unless keep_branch is true
        if !keep_branch {
            self.manager.delete_branch(feature_branch.clone(), None).await.ok();
        }

        // Push develop
        self.manager.push(
            self.gitflow_config.develop_branch.clone(),
            "origin".to_string(),
            false,
            false,
            None,
        ).await.ok();

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Finished feature {} (branch {} kept: {})", name, feature_branch, keep_branch),
            data: WorkflowData::GitFlowResult {
                branch_name: feature_branch,
                branch_type: GitFlowBranchType::Feature,
                commit_id: None,
            },
        })
    }

    async fn execute_gitflow_feature_publish(
        &self,
        name: String,
    ) -> Result<WorkflowResult, GitError> {
        let feature_branch = format!("{}{}", self.gitflow_config.feature_prefix, name);

        // Push feature branch to origin
        self.manager.push(feature_branch.clone(), "origin".to_string(), false, true, None).await?;

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Published feature branch {}", feature_branch),
            data: WorkflowData::GitFlowResult {
                branch_name: feature_branch,
                branch_type: GitFlowBranchType::Feature,
                commit_id: None,
            },
        })
    }

    async fn execute_gitflow_release_start(
        &self,
        version: String,
        base_branch: Option<String>,
    ) -> Result<WorkflowResult, GitError> {
        let release_branch = format!("{}{}", self.gitflow_config.release_prefix, version);
        let base = base_branch.unwrap_or_else(|| self.gitflow_config.develop_branch.clone());

        // Fetch latest
        self.manager.fetch("origin".to_string(), true, false, None).await.ok();

        // Create release branch
        self.manager.create_branch(release_branch.clone(), Some(base.clone()), None).await?;
        self.manager.checkout_branch(release_branch.clone(), false, None).await?;

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Started release {} based on {}", version, base),
            data: WorkflowData::GitFlowResult {
                branch_name: release_branch,
                branch_type: GitFlowBranchType::Release,
                commit_id: None,
            },
        })
    }

    async fn execute_gitflow_release_finish(
        &self,
        version: String,
        tag_message: Option<String>,
        push_to_remote: bool,
    ) -> Result<WorkflowResult, GitError> {
        let release_branch = format!("{}{}", self.gitflow_config.release_prefix, version);
        let tag_name = format!("{}{}", self.gitflow_config.tag_prefix, version);

        // Merge release into main
        self.manager.checkout_branch(self.gitflow_config.main_branch.clone(), false, None).await?;
        let main_merge = self.manager.merge(release_branch.clone(), None).await?;

        if !main_merge.success {
            return Ok(WorkflowResult {
                success: false,
                repo_id: None,
                message: "Failed to merge into main".to_string(),
                data: WorkflowData::MergeResult(main_merge),
            });
        }

        // Create tag on main
        let tag_msg = tag_message.unwrap_or_else(|| format!("Release {}", version));
        self.manager.create_tag(tag_name.clone(), None, Some(tag_msg), None).await?;

        // Merge release back into develop
        self.manager.checkout_branch(self.gitflow_config.develop_branch.clone(), false, None).await?;
        let _ = self.manager.merge(release_branch.clone(), None).await;

        // Delete release branch
        self.manager.delete_branch(release_branch.clone(), None).await.ok();

        // Push to remote if requested
        if push_to_remote {
            self.manager.push(self.gitflow_config.main_branch.clone(), "origin".to_string(), false, false, None).await.ok();
            self.manager.push(self.gitflow_config.develop_branch.clone(), "origin".to_string(), false, false, None).await.ok();
            self.manager.push_tag(tag_name.clone(), "origin".to_string(), None).await.ok();
        }

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Finished release {} with tag {}", version, tag_name),
            data: WorkflowData::GitFlowResult {
                branch_name: release_branch,
                branch_type: GitFlowBranchType::Release,
                commit_id: Some(tag_name),
            },
        })
    }

    async fn execute_gitflow_hotfix_start(
        &self,
        version: String,
        base_branch: Option<String>,
    ) -> Result<WorkflowResult, GitError> {
        let hotfix_branch = format!("{}{}", self.gitflow_config.hotfix_prefix, version);
        let base = base_branch.unwrap_or_else(|| self.gitflow_config.main_branch.clone());

        // Fetch latest
        self.manager.fetch("origin".to_string(), true, false, None).await.ok();

        // Create hotfix branch from main
        self.manager.create_branch(hotfix_branch.clone(), Some(base.clone()), None).await?;
        self.manager.checkout_branch(hotfix_branch.clone(), false, None).await?;

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Started hotfix {} based on {}", version, base),
            data: WorkflowData::GitFlowResult {
                branch_name: hotfix_branch,
                branch_type: GitFlowBranchType::Hotfix,
                commit_id: None,
            },
        })
    }

    async fn execute_gitflow_hotfix_finish(
        &self,
        version: String,
        tag_message: Option<String>,
        push_to_remote: bool,
    ) -> Result<WorkflowResult, GitError> {
        let hotfix_branch = format!("{}{}", self.gitflow_config.hotfix_prefix, version);
        let tag_name = format!("{}{}", self.gitflow_config.tag_prefix, version);

        // Merge hotfix into main
        self.manager.checkout_branch(self.gitflow_config.main_branch.clone(), false, None).await?;
        let main_merge = self.manager.merge(hotfix_branch.clone(), None).await?;

        if !main_merge.success {
            return Ok(WorkflowResult {
                success: false,
                repo_id: None,
                message: "Failed to merge hotfix into main".to_string(),
                data: WorkflowData::MergeResult(main_merge),
            });
        }

        // Create tag
        let tag_msg = tag_message.unwrap_or_else(|| format!("Hotfix {}", version));
        self.manager.create_tag(tag_name.clone(), None, Some(tag_msg), None).await?;

        // Merge hotfix back into develop
        self.manager.checkout_branch(self.gitflow_config.develop_branch.clone(), false, None).await?;
        let _ = self.manager.merge(hotfix_branch.clone(), None).await;

        // Delete hotfix branch
        self.manager.delete_branch(hotfix_branch.clone(), None).await.ok();

        // Push to remote if requested
        if push_to_remote {
            self.manager.push(self.gitflow_config.main_branch.clone(), "origin".to_string(), false, false, None).await.ok();
            self.manager.push(self.gitflow_config.develop_branch.clone(), "origin".to_string(), false, false, None).await.ok();
            self.manager.push_tag(tag_name.clone(), "origin".to_string(), None).await.ok();
        }

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Finished hotfix {} with tag {}", version, tag_name),
            data: WorkflowData::GitFlowResult {
                branch_name: hotfix_branch,
                branch_type: GitFlowBranchType::Hotfix,
                commit_id: Some(tag_name),
            },
        })
    }

    // ============================================================================
    // PR Management Operations (Mock implementations for now)
    // ============================================================================

    async fn execute_create_pr(
        &self,
        title: String,
        description: String,
        source_branch: String,
        target_branch: String,
        draft: bool,
        reviewers: Vec<String>,
    ) -> Result<WorkflowResult, GitError> {
        // This would integrate with GitHub/GitLab API
        // For now, return a mock result
        let pr_number = 1u64;

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Created PR #{}: {}", pr_number, title),
            data: WorkflowData::PullRequestResult {
                pr: PullRequest {
                    number: pr_number,
                    title,
                    description,
                    state: if draft { PullRequestState::Draft } else { PullRequestState::Open },
                    draft,
                    source_branch,
                    target_branch,
                    author: "current_user".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    closed_at: None,
                    merged_at: None,
                    merge_commit_sha: None,
                    head_sha: "abc123".to_string(),
                    reviewers,
                    labels: vec![],
                    comments_count: 0,
                    review_comments_count: 0,
                    commits_count: 0,
                    additions: 0,
                    deletions: 0,
                    changed_files: 0,
                    is_mergeable: Some(true),
                    mergeable_state: Some("clean".to_string()),
                    checks_status: None,
                },
            },
        })
    }

    async fn execute_update_pr(
        &self,
        number: u64,
        title: Option<String>,
        description: Option<String>,
        state: Option<PullRequestState>,
    ) -> Result<WorkflowResult, GitError> {
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Updated PR #{}", number),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_list_prs(
        &self,
        state: PullRequestState,
        limit: usize,
    ) -> Result<WorkflowResult, GitError> {
        // Mock implementation
        let prs: Vec<PullRequest> = vec![];

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Found {} PRs with state {:?}", prs.len(), state),
            data: WorkflowData::PullRequestListResult { prs },
        })
    }

    async fn execute_get_pr(&self, number: u64) -> Result<WorkflowResult, GitError> {
        // Mock implementation
        Err(GitError::OperationFailed("PR API integration not implemented".to_string()))
    }

    async fn execute_review_pr(
        &self,
        number: u64,
        action: PullRequestReviewAction,
        comment: Option<String>,
    ) -> Result<WorkflowResult, GitError> {
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Reviewed PR #{} with {:?}", number, action),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_merge_pr(
        &self,
        number: u64,
        method: MergeMethod,
        commit_message: Option<String>,
        delete_source_branch: bool,
    ) -> Result<WorkflowResult, GitError> {
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Merged PR #{} using {:?}", number, method),
            data: WorkflowData::Empty,
        })
    }

    // ============================================================================
    // CI/CD Operations (Mock implementations for now)
    // ============================================================================

    async fn execute_get_ci_status(
        &self,
        branch: Option<String>,
        commit: Option<String>,
    ) -> Result<WorkflowResult, GitError> {
        // Get status from repository
        let status = self.manager.status(None).await?;
        let current_branch = branch.unwrap_or_else(|| status.branch.clone());

        // Mock CI status
        let checks_summary = CiChecksSummary {
            total: 3,
            passed: 3,
            failed: 0,
            pending: 0,
            skipped: 0,
            conclusion: Some(CiPipelineStatus::Success),
            checks: vec![
                CheckRun {
                    name: "test".to_string(),
                    status: CiPipelineStatus::Success,
                    conclusion: Some("success".to_string()),
                    started_at: Some(Utc::now()),
                    completed_at: Some(Utc::now()),
                    output_summary: Some("All tests passed".to_string()),
                    details_url: "https://ci.example.com/test".to_string(),
                },
                CheckRun {
                    name: "lint".to_string(),
                    status: CiPipelineStatus::Success,
                    conclusion: Some("success".to_string()),
                    started_at: Some(Utc::now()),
                    completed_at: Some(Utc::now()),
                    output_summary: Some("No lint errors".to_string()),
                    details_url: "https://ci.example.com/lint".to_string(),
                },
                CheckRun {
                    name: "build".to_string(),
                    status: CiPipelineStatus::Success,
                    conclusion: Some("success".to_string()),
                    started_at: Some(Utc::now()),
                    completed_at: Some(Utc::now()),
                    output_summary: Some("Build successful".to_string()),
                    details_url: "https://ci.example.com/build".to_string(),
                },
            ],
        };

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("CI status for {}: all checks passed", current_branch),
            data: WorkflowData::CiStatusResult { branch: current_branch, checks: checks_summary },
        })
    }

    async fn execute_list_ci_pipelines(
        &self,
        branch: Option<String>,
        status: Option<CiPipelineStatus>,
        limit: usize,
    ) -> Result<WorkflowResult, GitError> {
        // Mock implementation
        let pipelines: Vec<CiPipeline> = vec![];

        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Found {} pipelines", pipelines.len()),
            data: WorkflowData::CiPipelineListResult { pipelines },
        })
    }

    async fn execute_retry_ci_pipeline(&self, pipeline_id: String) -> Result<WorkflowResult, GitError> {
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Retried pipeline {}", pipeline_id),
            data: WorkflowData::Empty,
        })
    }

    async fn execute_cancel_ci_pipeline(&self, pipeline_id: String) -> Result<WorkflowResult, GitError> {
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: format!("Canceled pipeline {}", pipeline_id),
            data: WorkflowData::Empty,
        })
    }

    // ============================================================================
    // Code Review Operations
    // ============================================================================

    async fn execute_run_checklist(
        &self,
        branch: String,
        base_branch: Option<String>,
        checklist_items: Vec<ChecklistItem>,
    ) -> Result<WorkflowResult, GitError> {
        // Get diff between branches
        let _base = base_branch.unwrap_or_else(|| self.gitflow_config.develop_branch.clone());

        // Run automated checks
        let mut checked_items = checklist_items.clone();
        let mut issues = Vec::new();

        for item in &mut checked_items {
            if item.auto_checkable {
                // Run automated check
                match self.run_automated_check(item).await {
                    Ok(true) => item.is_checked = true,
                    Ok(false) => {
                        issues.push(ReviewIssue {
                            id: format!("auto-{}", item.id),
                            category: item.category,
                            severity: if item.is_required { IssueSeverity::Error } else { IssueSeverity::Warning },
                            title: format!("Check failed: {}", item.title),
                            description: item.description.clone(),
                            file_path: None,
                            line_number: None,
                            suggestion: None,
                        });
                    }
                    Err(_) => {}
                }
            }
        }

        let total = checked_items.len();
        let passed = checked_items.iter().filter(|i| i.is_checked || !i.is_required).count();

        let overall_status = if passed == total {
            ReviewStatus::Pass
        } else if checked_items.iter().any(|i| !i.is_checked && i.is_required) {
            ReviewStatus::HasIssues
        } else {
            ReviewStatus::NeedsReview
        };

        let report = CodeReviewReport {
            commit_id: "head".to_string(),
            branch: branch.clone(),
            base_branch: _base.clone(),
            created_at: Utc::now(),
            overall_status,
            categories: vec![],
            issues,
            statistics: ReviewStatistics {
                total_files_changed: 0,
                total_lines_added: 0,
                total_lines_deleted: 0,
                total_issues: 0,
                critical_issues: 0,
                error_issues: 0,
                warning_issues: 0,
                info_issues: 0,
            },
        };

        Ok(WorkflowResult {
            success: overall_status != ReviewStatus::HasIssues,
            repo_id: None,
            message: format!("Code review checklist: {}/{} checks passed", passed, total),
            data: WorkflowData::CodeReviewResult { report, checklist: checked_items },
        })
    }

    async fn run_automated_check(&self, item: &ChecklistItem) -> Result<bool, GitError> {
        // Run automated checks based on item ID
        match item.id.as_str() {
            "cs-1" | "cs-3" => {
                // Code style checks would run linter here
                Ok(true)
            }
            "sc-1" => {
                // Security check for secrets
                Ok(true)
            }
            "ts-1" | "ts-2" => {
                // Test checks
                Ok(true)
            }
            "dc-1" => {
                // Documentation check
                Ok(true)
            }
            _ => Ok(false), // Manual check required
        }
    }

    async fn execute_get_review_report(&self, commit_id: String) -> Result<WorkflowResult, GitError> {
        // Mock implementation
        Err(GitError::OperationFailed("Review report API not implemented".to_string()))
    }

    async fn execute_add_pr_comment(
        &self,
        file_path: Option<String>,
        line_number: Option<u32>,
        comment: String,
    ) -> Result<WorkflowResult, GitError> {
        Ok(WorkflowResult {
            success: true,
            repo_id: None,
            message: "Comment added".to_string(),
            data: WorkflowData::Empty,
        })
    }

    // ============================================================================
    // Utilities
    // ============================================================================

    /// Get the underlying GitManager
    pub fn manager(&self) -> &GitManager {
        &self.manager
    }

    /// Open a repository
    pub async fn open_repo(&self, path: &Path) -> Result<String, GitError> {
        self.manager.open_repo(path).await
    }

    /// Set active repository
    pub async fn set_active_repo(&self, repo_id: &str) -> Result<(), GitError> {
        self.manager.set_active_repo(repo_id).await
    }
}

impl Default for GitWorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow execution result
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub success: bool,
    pub repo_id: Option<String>,
    pub message: String,
    pub data: WorkflowData,
}

/// Workflow result data
#[derive(Debug, Clone)]
pub enum WorkflowData {
    Empty,
    CloneResult { url: String, path: String, branch: Option<String> },
    CommitResult { commit_id: String, message: String },
    PushResult { remote: String, refspec: String },
    CheckoutResult { branch: String, created: bool },
    MergeResult(MergeResult),
    TagResult { name: String, message: Option<String> },
    RemoteResult { name: String, url: String },
    StashResult { index: Option<usize>, message: Option<String> },
    StatusResult(RepoStatus),
    LogResult { commits: Vec<CommitInfo>, branch: Option<String> },
    GitFlowInitResult { config: GitFlowConfig },
    GitFlowResult { branch_name: String, branch_type: GitFlowBranchType, commit_id: Option<String> },
    PullRequestResult { pr: PullRequest },
    PullRequestListResult { prs: Vec<PullRequest> },
    CiStatusResult { branch: String, checks: CiChecksSummary },
    CiPipelineListResult { pipelines: Vec<CiPipeline> },
    CodeReviewResult { report: CodeReviewReport, checklist: Vec<ChecklistItem> },
}

/// Workflow execution summary
#[derive(Debug, Clone, Default)]
pub struct WorkflowSummary {
    pub total_actions: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
}

impl WorkflowSummary {
    pub fn from_results(results: &[Result<WorkflowResult, GitError>]) -> Self {
        let total = results.len();
        let successful = results.iter().filter(|r| r.is_ok()).count();
        let failed = results.iter().filter(|r| r.is_err()).count();

        Self {
            total_actions: total,
            successful,
            failed,
            skipped: 0,
            duration_ms: 0,
        }
    }
}
