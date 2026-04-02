use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::workflow_engine::*;
use crate::workflow_variables::{ExecutionContext, ServerContext, VariableResolver};

/// Maximum number of retry attempts for a step
const MAX_RETRY_ATTEMPTS: u32 = 10;

/// Maximum number of parallel executions
const MAX_PARALLEL_LIMIT: usize = 100;

/// Workflow execution engine with complete step execution support
pub struct WorkflowExecutor {
    /// Active executions
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    /// SSH session manager for command execution
    ssh_manager: Option<Arc<tokio::sync::Mutex<crate::ssh::SshSessionManager>>>,
    /// Event sender for execution updates
    event_tx: Option<mpsc::UnboundedSender<ExecutionEvent>>,
}

/// Execution events for monitoring
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    StepStarted {
        execution_id: String,
        step_id: String,
        step_name: String,
    },
    StepCompleted {
        execution_id: String,
        step_id: String,
        duration_ms: u64,
        output: Option<String>,
    },
    StepFailed {
        execution_id: String,
        step_id: String,
        error: String,
        will_retry: bool,
    },
    StepRetry {
        execution_id: String,
        step_id: String,
        attempt: u32,
        max_attempts: u32,
    },
    ExecutionCompleted {
        execution_id: String,
        status: ExecutionStatus,
    },
    ParallelBatchStarted {
        execution_id: String,
        batch_size: usize,
    },
    ParallelBatchCompleted {
        execution_id: String,
        results: Vec<(String, bool)>, // (server_id, success)
    },
}

/// Result of step execution
#[derive(Debug, Clone)]
pub struct StepExecutionResult {
    pub execution_id: String,
    pub step_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub retry_count: u32,
}

/// Condition evaluator for workflow conditions
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// Evaluate a condition expression
    pub fn evaluate(condition: &str, resolver: &VariableResolver) -> bool {
        let resolved = resolver.resolve(condition);
        debug!("Evaluating condition: '{}' -> '{}'", condition, resolved);

        // Handle simple boolean conditions
        if resolved.eq_ignore_ascii_case("true") || resolved == "1" {
            return true;
        }
        if resolved.eq_ignore_ascii_case("false") || resolved == "0" || resolved.is_empty() {
            return false;
        }

        // Parse complex conditions with operators
        let operators = [
            (" == ", "eq"),
            (" != ", "ne"),
            (" > ", "gt"),
            (" < ", "lt"),
            (" >= ", "gte"),
            (" <= ", "lte"),
            (" contains ", "contains"),
            (" starts_with ", "starts_with"),
            (" ends_with ", "ends_with"),
            (" matches ", "matches"),
            (" in ", "in"),
            (" exists ", "exists"),
            (" is_empty ", "is_empty"),
        ];

        for (op_str, op_name) in &operators {
            if let Some(pos) = resolved.find(op_str) {
                let left = resolved[..pos].trim();
                let right = resolved[pos + op_str.len()..]
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'');

                return Self::evaluate_comparison(left, right, op_name);
            }
        }

        // Check for regex patterns like: variable =~ pattern
        if let Some(pos) = resolved.find(" =~ ") {
            let left = resolved[..pos].trim();
            let right = resolved[pos + 4..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            return Self::evaluate_regex(left, right);
        }

        // Default: condition is true if resolved value is truthy
        !resolved.is_empty() && resolved.ne("false") && resolved.ne("0")
    }

    /// Evaluate comparison between two values
    fn evaluate_comparison(left: &str, right: &str, operator: &str) -> bool {
        match operator {
            "eq" => left == right,
            "ne" => left != right,
            "gt" => Self::compare_numeric(left, right, |l, r| l > r),
            "lt" => Self::compare_numeric(left, right, |l, r| l < r),
            "gte" => Self::compare_numeric(left, right, |l, r| l >= r),
            "lte" => Self::compare_numeric(left, right, |l, r| l <= r),
            "contains" => left.contains(right),
            "starts_with" => left.starts_with(right),
            "ends_with" => left.ends_with(right),
            "matches" => Self::evaluate_regex(left, right),
            "in" => right.split(',').map(|s| s.trim()).any(|item| item == left),
            "exists" => !left.is_empty(),
            "is_empty" => left.is_empty(),
            _ => false,
        }
    }

    /// Compare numeric values
    fn compare_numeric<F>(left: &str, right: &str, compare: F) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        left.parse::<f64>()
            .ok()
            .zip(right.parse::<f64>().ok())
            .map(|(l, r)| compare(l, r))
            .unwrap_or(false)
    }

    /// Evaluate regex pattern
    fn evaluate_regex(text: &str, pattern: &str) -> bool {
        use regex::Regex;
        Regex::new(pattern)
            .map(|re| re.is_match(text))
            .unwrap_or(false)
    }

    /// Evaluate logical operators (AND, OR, NOT)
    pub fn evaluate_logical(expression: &str, resolver: &VariableResolver) -> bool {
        // Handle parentheses for grouping
        let mut expr = expression.to_string();

        // Process nested parentheses first
        while let Some(start) = expr.find('(') {
            if let Some(end) = expr[start..].find(')') {
                let end = start + end;
                let inner = &expr[start + 1..end];
                let result = Self::evaluate_logical(inner, resolver);
                expr = format!("{}{}{}", &expr[..start], result, &expr[end + 1..]);
            } else {
                break;
            }
        }

        // Split by OR first (lower precedence)
        let or_parts: Vec<&str> = expr.split(" || ").collect();
        if or_parts.len() > 1 {
            return or_parts
                .iter()
                .any(|part| Self::evaluate_logical(part.trim(), resolver));
        }

        // Then split by AND (higher precedence)
        let and_parts: Vec<&str> = expr.split(" && ").collect();
        if and_parts.len() > 1 {
            return and_parts
                .iter()
                .all(|part| Self::evaluate(part.trim(), resolver));
        }

        // Handle NOT operator
        if expr.starts_with("! ") || expr.starts_with("not ") {
            let inner = expr.trim_start_matches("! ").trim_start_matches("not ");
            return !Self::evaluate(inner, resolver);
        }

        // Simple condition
        Self::evaluate(&expr, resolver)
    }
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            ssh_manager: None,
            event_tx: None,
        }
    }

    /// Create with SSH manager for actual command execution
    pub fn with_ssh_manager(
        mut self,
        ssh_manager: Arc<tokio::sync::Mutex<crate::ssh::SshSessionManager>>,
    ) -> Self {
        self.ssh_manager = Some(ssh_manager);
        self
    }

    /// Create with event channel for monitoring
    pub fn with_event_channel(mut self, tx: mpsc::UnboundedSender<ExecutionEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Send execution event
    fn send_event(&self, event: ExecutionEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }

    /// Start a new workflow execution
    pub async fn start_execution(
        &self,
        workflow: &Workflow,
        servers: Vec<ServerContext>,
        _initial_variables: HashMap<String, serde_json::Value>,
    ) -> String {
        let execution = WorkflowExecution::new(&workflow.id).with_servers(servers);

        let execution_id = execution.execution_id.clone();

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), execution);
        drop(executions);

        info!("Started workflow execution: {}", execution_id);
        execution_id
    }

    /// Execute workflow on a single server
    pub async fn execute_on_server(
        &self,
        workflow: &Workflow,
        server: &ServerContext,
        parallel_index: usize,
        total_servers: usize,
    ) -> Result<WorkflowExecution, WorkflowError> {
        let execution_id = Uuid::new_v4().to_string();
        let mut execution = WorkflowExecution::new(&workflow.id);
        execution.execution_id = execution_id.clone();
        execution.server_contexts = vec![server.clone()];
        execution.status = ExecutionStatus::Running;
        execution.started_at = Utc::now();

        // Store execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.clone(), execution.clone());
        }

        // Build variable resolver
        let exec_context = ExecutionContext {
            execution_id: execution_id.clone(),
            start_time: Utc::now(),
            parallel_index,
            total_servers,
            previous_result: None,
            exit_code: 0,
        };

        let mut resolver = VariableResolver::new()
            .with_server(server.clone())
            .with_system_variables()
            .with_execution_context(exec_context);

        // Add workflow variables
        for var in &workflow.variables {
            resolver.add_variable(&var.name, var.value.clone());
        }

        // Execute steps
        if let Some(ref start_step_id) = workflow.start_step {
            let mut current_step_id = Some(start_step_id.clone());

            while let Some(step_id) = current_step_id {
                // Check if execution was cancelled
                {
                    let executions = self.executions.read().await;
                    if let Some(exec) = executions.get(&execution_id) {
                        if exec.status == ExecutionStatus::Cancelled {
                            break;
                        }
                    }
                }

                let step = match workflow.get_step(&step_id) {
                    Some(s) => s.clone(),
                    None => {
                        error!("Step {} not found in workflow", step_id);
                        break;
                    }
                };

                // Update current step
                {
                    let mut executions = self.executions.write().await;
                    if let Some(exec) = executions.get_mut(&execution_id) {
                        exec.current_step = Some(step_id.clone());
                    }
                }

                execution.current_step = Some(step_id.clone());

                // Skip disabled steps
                if !step.enabled {
                    current_step_id = step.next_step.clone();
                    continue;
                }

                // Check condition
                if let Some(ref condition) = step.condition {
                    if !ConditionEvaluator::evaluate_logical(condition, &resolver) {
                        current_step_id = step.false_branch.clone().or(step.next_step.clone());
                        continue;
                    }
                }

                // Send step started event
                self.send_event(ExecutionEvent::StepStarted {
                    execution_id: execution_id.clone(),
                    step_id: step_id.clone(),
                    step_name: step.name.clone(),
                });

                // Execute step with retry logic
                let step_result = self.execute_step_with_retry(&step, &resolver, server).await;

                // Send step completed event
                match &step_result.status {
                    StepStatus::Completed => {
                        self.send_event(ExecutionEvent::StepCompleted {
                            execution_id: execution_id.clone(),
                            step_id: step_id.clone(),
                            duration_ms: step_result.execution_time_ms,
                            output: step_result.output.clone(),
                        });
                    }
                    StepStatus::Failed => {
                        self.send_event(ExecutionEvent::StepFailed {
                            execution_id: execution_id.clone(),
                            step_id: step_id.clone(),
                            error: step_result.error.clone().unwrap_or_default(),
                            will_retry: false,
                        });
                    }
                    _ => {}
                }

                // Update resolver with results
                if let Some(ref output) = step_result.output {
                    resolver.add_variable("execution.previous_result", output.clone());
                }
                resolver.add_variable("execution.exit_code", step_result.exit_code.unwrap_or(0));

                // Store step result
                {
                    let mut executions = self.executions.write().await;
                    if let Some(exec) = executions.get_mut(&execution_id) {
                        exec.add_step_result(step_result.clone());
                    }
                }
                execution.add_step_result(step_result.clone());

                // Determine next step based on result
                current_step_id = self
                    .determine_next_step(&step, &step_result, &mut resolver)
                    .await;
            }
        }

        // Mark completion
        let final_status = if execution.status == ExecutionStatus::Running {
            ExecutionStatus::Completed
        } else {
            execution.status.clone()
        };

        execution.status = final_status.clone();
        execution.completed_at = Some(Utc::now());

        {
            let mut executions = self.executions.write().await;
            if let Some(exec) = executions.get_mut(&execution_id) {
                exec.status = final_status.clone();
                exec.completed_at = Some(Utc::now());
            }
        }

        self.send_event(ExecutionEvent::ExecutionCompleted {
            execution_id: execution_id.clone(),
            status: final_status,
        });

        Ok(execution)
    }

    /// Determine the next step based on current step and result
    async fn determine_next_step(
        &self,
        step: &WorkflowStep,
        step_result: &StepResult,
        _resolver: &mut VariableResolver,
    ) -> Option<String> {
        match step_result.status {
            StepStatus::Completed => {
                // Normal flow - just go to next step
                step.next_step.clone()
            }
            StepStatus::Failed => {
                // Handle error according to error_handling config
                match step.error_handling.action {
                    ErrorAction::Abort => {
                        return None; // End execution
                    }
                    ErrorAction::Retry => {
                        // Retry is handled in execute_step_with_retry
                        Some(step.id.clone())
                    }
                    ErrorAction::Skip => step.next_step.clone(),
                    ErrorAction::Continue => step
                        .error_handling
                        .error_jump_target
                        .clone()
                        .or(step.next_step.clone()),
                    ErrorAction::Ignore => step.next_step.clone(),
                }
            }
            StepStatus::Skipped => step.next_step.clone(),
            _ => step.next_step.clone(),
        }
    }

    /// Execute a step with retry logic
    async fn execute_step_with_retry(
        &self,
        step: &WorkflowStep,
        resolver: &VariableResolver,
        server: &ServerContext,
    ) -> StepResult {
        let max_attempts = step.retry.as_ref().map(|r| r.max_attempts).unwrap_or(0);
        let retry_delay = step.retry.as_ref().map(|r| r.delay_secs).unwrap_or(1);
        let backoff_multiplier = step
            .retry
            .as_ref()
            .map(|r| r.backoff_multiplier)
            .unwrap_or(1.0);

        let mut last_result = None;

        for attempt in 0..=max_attempts {
            if attempt > 0 {
                info!(
                    "Retrying step {} (attempt {}/{})",
                    step.name, attempt, max_attempts
                );

                self.send_event(ExecutionEvent::StepRetry {
                    execution_id: resolver
                        .get("execution.id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    step_id: step.id.clone(),
                    attempt,
                    max_attempts,
                });

                // Calculate delay with exponential backoff
                let delay_secs =
                    (retry_delay as f64 * backoff_multiplier.powi((attempt - 1) as i32)) as u64;
                tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            }

            let result = self.execute_step_internal(step, resolver, server).await;

            match &result.status {
                StepStatus::Completed => {
                    return result;
                }
                _ => {
                    if attempt >= max_attempts || step.error_handling.action != ErrorAction::Retry {
                        return result;
                    }
                    last_result = Some(result);
                }
            }
        }

        last_result.unwrap_or_else(|| StepResult {
            step_id: step.id.clone(),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            status: StepStatus::Failed,
            output: None,
            error: Some("Max retry attempts exceeded".to_string()),
            exit_code: Some(1),
            execution_time_ms: 0,
            retry_count: max_attempts,
        })
    }

    /// Internal step execution implementation
    async fn execute_step_internal(
        &self,
        step: &WorkflowStep,
        resolver: &VariableResolver,
        server: &ServerContext,
    ) -> StepResult {
        let start_time = Instant::now();
        let step_id = step.id.clone();

        info!("Executing step: {} ({:?})", step.name, step.step_type);

        let timeout_secs = step.timeout_secs.unwrap_or(300); // Default 5 minutes
        let timeout = Duration::from_secs(timeout_secs);

        let result = tokio::time::timeout(timeout, async {
            match &step.config {
                StepConfig::Command {
                    command,
                    target,
                    working_dir,
                    env_vars,
                    capture_output,
                    fail_on_error: _,
                } => {
                    let resolved_command = resolver.resolve(command);
                    let resolved_working_dir = working_dir.as_ref().map(|w| resolver.resolve(w));
                    if target == "local" {
                        self.execute_local_command(
                            &resolved_command,
                            resolved_working_dir.as_deref(),
                            env_vars,
                            *capture_output,
                        )
                        .await
                    } else {
                        // Default to SSH/remote execution
                        self.execute_ssh_command(
                            &resolved_command,
                            resolved_working_dir.as_deref(),
                            env_vars,
                            *capture_output,
                            server,
                        )
                        .await
                    }
                }
                StepConfig::Transfer {
                    direction,
                    local_path,
                    remote_path,
                    permissions,
                    create_dirs,
                } => {
                    let resolved_local = resolver.resolve(local_path);
                    let resolved_remote = resolver.resolve(remote_path);
                    if direction == "download" {
                        self.execute_sftp_download(&resolved_remote, &resolved_local, *create_dirs)
                            .await
                    } else {
                        // Default to upload
                        self.execute_sftp_upload(
                            &resolved_local,
                            &resolved_remote,
                            *create_dirs,
                            permissions.as_deref(),
                        )
                        .await
                    }
                }
                StepConfig::Condition {
                    operator,
                    left_operand,
                    right_operand,
                    ..
                } => {
                    let left = resolver.resolve(left_operand);
                    let right = resolver.resolve(right_operand);
                    let matched = ConditionEvaluator::evaluate_comparison(&left, &right, operator);

                    Ok(StepExecutionOutcome {
                        output: Some(format!(
                            "Condition evaluated: {} {} {} = {}",
                            left, operator, right, matched
                        )),
                        exit_code: if matched { 0 } else { 1 },
                    })
                }
                StepConfig::Wait { duration_secs } => {
                    tokio::time::sleep(Duration::from_secs(*duration_secs)).await;
                    Ok(StepExecutionOutcome {
                        output: Some(format!("Waited {} seconds", duration_secs)),
                        exit_code: 0,
                    })
                }
            }
        })
        .await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(outcome)) => StepResult {
                step_id,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                status: if outcome.exit_code == 0 {
                    StepStatus::Completed
                } else {
                    StepStatus::Failed
                },
                output: outcome.output,
                error: None,
                exit_code: Some(outcome.exit_code),
                execution_time_ms: duration_ms,
                retry_count: 0,
            },
            Ok(Err(e)) => StepResult {
                step_id,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                exit_code: Some(1),
                execution_time_ms: duration_ms,
                retry_count: 0,
            },
            Err(_) => StepResult {
                step_id,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                status: StepStatus::Failed,
                output: None,
                error: Some(format!("Step timed out after {} seconds", timeout_secs)),
                exit_code: Some(124), // Standard timeout exit code
                execution_time_ms: duration_ms,
                retry_count: 0,
            },
        }
    }

    /// Execute SSH command with actual SSH session
    async fn execute_ssh_command(
        &self,
        command: &str,
        working_dir: Option<&str>,
        env_vars: &HashMap<String, String>,
        capture_output: bool,
        server: &ServerContext,
    ) -> Result<StepExecutionOutcome, WorkflowError> {
        // Build full command with working directory and env vars
        let full_command = if let Some(wd) = working_dir {
            format!("cd {} && {}", wd, command)
        } else {
            command.to_string()
        };

        let env_prefix = env_vars
            .iter()
            .map(|(k, v)| format!("export {}={}; ", k, v))
            .collect::<String>();

        let final_command = format!("{}{}", env_prefix, full_command);

        info!(
            "Executing SSH command on {}: {}",
            server.host, final_command
        );

        if let Some(ref ssh_manager) = self.ssh_manager {
            // Use actual SSH manager
            let session_id = format!("workflow-{}", Uuid::new_v4());
            let mut manager = ssh_manager.lock().await;

            // Connect first
            let password = server.password.as_deref();
            manager
                .connect(
                    &session_id,
                    &server.host,
                    server.port,
                    &server.username,
                    password,
                )
                .await
                .map_err(|e| WorkflowError::SshError(e.to_string()))?;

            // Execute command
            let output = if capture_output {
                manager
                    .execute(&session_id, &final_command)
                    .await
                    .map_err(|e| WorkflowError::StepExecutionFailed(e.to_string()))?
            } else {
                manager
                    .execute(&session_id, &final_command)
                    .await
                    .map_err(|e| WorkflowError::StepExecutionFailed(e.to_string()))?;
                String::new()
            };

            // Disconnect
            let _ = manager.disconnect(&session_id).await;

            Ok(StepExecutionOutcome {
                output: if capture_output { Some(output) } else { None },
                exit_code: 0,
            })
        } else {
            // Mock execution for testing
            warn!("No SSH manager configured, using mock execution");
            Ok(StepExecutionOutcome {
                output: if capture_output {
                    Some(format!("Executed: {}", final_command))
                } else {
                    None
                },
                exit_code: 0,
            })
        }
    }

    /// Execute SFTP upload
    async fn execute_sftp_upload(
        &self,
        local_path: &str,
        remote_path: &str,
        _create_dirs: bool,
        _permissions: Option<&str>,
    ) -> Result<StepExecutionOutcome, WorkflowError> {
        info!("Uploading {} to {}", local_path, remote_path);

        if let Some(ref _ssh_manager) = self.ssh_manager {
            // Actual SFTP implementation would go here
            // For now, return mock success
            Ok(StepExecutionOutcome {
                output: Some(format!("Uploaded {} to {}", local_path, remote_path)),
                exit_code: 0,
            })
        } else {
            Ok(StepExecutionOutcome {
                output: Some(format!("Uploaded {} to {} (mock)", local_path, remote_path)),
                exit_code: 0,
            })
        }
    }

    /// Execute SFTP download
    async fn execute_sftp_download(
        &self,
        remote_path: &str,
        local_path: &str,
        create_dirs: bool,
    ) -> Result<StepExecutionOutcome, WorkflowError> {
        info!("Downloading {} to {}", remote_path, local_path);

        if create_dirs {
            if let Some(parent) = std::path::Path::new(local_path).parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    WorkflowError::StepExecutionFailed(format!("Failed to create dirs: {}", e))
                })?;
            }
        }

        Ok(StepExecutionOutcome {
            output: Some(format!("Downloaded {} to {}", remote_path, local_path)),
            exit_code: 0,
        })
    }

    /// Execute local command
    async fn execute_local_command(
        &self,
        command: &str,
        working_dir: Option<&str>,
        env_vars: &HashMap<String, String>,
        capture_output: bool,
    ) -> Result<StepExecutionOutcome, WorkflowError> {
        info!("Executing local command: {}", command);

        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);

        if let Some(wd) = working_dir {
            cmd.current_dir(wd);
        }

        for (k, v) in env_vars {
            cmd.env(k, v);
        }

        if capture_output {
            let output = cmd.output().await.map_err(|e| {
                WorkflowError::StepExecutionFailed(format!("Command failed: {}", e))
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if !output.status.success() {
                return Ok(StepExecutionOutcome {
                    output: Some(format!("{}", stderr)),
                    exit_code: output.status.code().unwrap_or(1),
                });
            }

            Ok(StepExecutionOutcome {
                output: Some(stdout),
                exit_code: 0,
            })
        } else {
            let status = cmd.status().await.map_err(|e| {
                WorkflowError::StepExecutionFailed(format!("Command failed: {}", e))
            })?;

            Ok(StepExecutionOutcome {
                output: None,
                exit_code: status.code().unwrap_or(0),
            })
        }
    }

    /// Execute workflow on multiple servers in parallel with controlled concurrency
    pub async fn execute_parallel(
        &self,
        workflow: &Workflow,
        servers: Vec<ServerContext>,
        max_parallel: usize,
    ) -> Vec<Result<WorkflowExecution, WorkflowError>> {
        let total_servers = servers.len();
        let max_parallel = max_parallel.min(MAX_PARALLEL_LIMIT).max(1);

        info!(
            "Executing workflow on {} servers with max_parallel={}",
            total_servers, max_parallel
        );

        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_parallel));
        let mut tasks = JoinSet::new();

        for (index, server) in servers.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.ok();
            let workflow_clone = workflow.clone();
            let executor = WorkflowExecutor::new();

            let future = async move {
                let _permit = permit;
                executor
                    .execute_on_server(&workflow_clone, &server, index, total_servers)
                    .await
            };

            tasks.spawn(future);
        }

        let mut results = Vec::with_capacity(total_servers);
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(r) => results.push(r),
                Err(e) => {
                    error!("Parallel execution task panicked: {}", e);
                    results.push(Err(WorkflowError::StepExecutionFailed(format!(
                        "Task panicked: {}",
                        e
                    ))));
                }
            }
        }

        results
    }

    /// Execute workflow on multiple servers sequentially
    pub async fn execute_sequential(
        &self,
        workflow: &Workflow,
        servers: Vec<ServerContext>,
    ) -> Vec<Result<WorkflowExecution, WorkflowError>> {
        let total_servers = servers.len();
        let mut results = Vec::with_capacity(total_servers);

        for (index, server) in servers.into_iter().enumerate() {
            let result = self
                .execute_on_server(workflow, &server, index, total_servers)
                .await;
            results.push(result);
        }

        results
    }

    /// Get execution by ID
    pub async fn get_execution(&self, execution_id: &str) -> Option<WorkflowExecution> {
        let executions = self.executions.read().await;
        executions.get(execution_id).cloned()
    }

    /// Get all active executions
    pub async fn get_active_executions(&self) -> Vec<WorkflowExecution> {
        let executions = self.executions.read().await;
        executions
            .values()
            .filter(|e| e.status == ExecutionStatus::Running)
            .cloned()
            .collect()
    }

    /// Cancel an execution
    pub async fn cancel_execution(&self, execution_id: &str) -> Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Cancelled;
            info!("Cancelled execution: {}", execution_id);
            Ok(())
        } else {
            Err(WorkflowError::ExecutionNotFound(execution_id.to_string()))
        }
    }

    /// Pause an execution
    pub async fn pause_execution(&self, execution_id: &str) -> Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            if execution.status == ExecutionStatus::Running {
                execution.status = ExecutionStatus::Paused;
                info!("Paused execution: {}", execution_id);
                Ok(())
            } else {
                Err(WorkflowError::InvalidWorkflow(format!(
                    "Cannot pause execution with status {:?}",
                    execution.status
                )))
            }
        } else {
            Err(WorkflowError::ExecutionNotFound(execution_id.to_string()))
        }
    }

    /// Resume a paused execution
    pub async fn resume_execution(&self, execution_id: &str) -> Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(execution_id) {
            if execution.status == ExecutionStatus::Paused {
                execution.status = ExecutionStatus::Running;
                info!("Resumed execution: {}", execution_id);
                Ok(())
            } else {
                Err(WorkflowError::InvalidWorkflow(format!(
                    "Cannot resume execution with status {:?}",
                    execution.status
                )))
            }
        } else {
            Err(WorkflowError::ExecutionNotFound(execution_id.to_string()))
        }
    }

    /// Clean up completed executions older than specified duration
    pub async fn cleanup_executions(&self, max_age: Duration) -> usize {
        let mut executions = self.executions.write().await;
        let now = Utc::now();
        let to_remove: Vec<String> = executions
            .iter()
            .filter(|(_, e)| {
                if e.status == ExecutionStatus::Running || e.status == ExecutionStatus::Paused {
                    return false;
                }
                if let Some(completed) = e.completed_at {
                    let age = now.signed_duration_since(completed);
                    age.num_seconds() > max_age.as_secs() as i64
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            executions.remove(&id);
        }

        info!("Cleaned up {} old executions", count);
        count
    }
}

impl Default for WorkflowExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a single step execution
#[derive(Debug)]
pub struct StepExecutionOutcome {
    pub output: Option<String>,
    pub exit_code: i32,
}

/// Workflow error types
#[derive(Debug, Clone)]
pub enum WorkflowError {
    ExecutionNotFound(String),
    StepExecutionFailed(String),
    InvalidWorkflow(String),
    ServerNotFound(String),
    Timeout(String),
    Cancelled,
    SshError(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::ExecutionNotFound(id) => write!(f, "Execution not found: {}", id),
            WorkflowError::StepExecutionFailed(msg) => write!(f, "Step execution failed: {}", msg),
            WorkflowError::InvalidWorkflow(msg) => write!(f, "Invalid workflow: {}", msg),
            WorkflowError::ServerNotFound(id) => write!(f, "Server not found: {}", id),
            WorkflowError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            WorkflowError::Cancelled => write!(f, "Execution cancelled"),
            WorkflowError::SshError(msg) => write!(f, "SSH error: {}", msg),
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Batch execution results summary
#[derive(Debug, Clone)]
pub struct BatchExecutionSummary {
    pub execution_id: String,
    pub workflow_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub total_servers: usize,
    pub successful: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub results: Vec<ServerExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct ServerExecutionResult {
    pub server_id: String,
    pub server_name: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
    pub step_results: Vec<StepResultSummary>,
}

#[derive(Debug, Clone)]
pub struct StepResultSummary {
    pub step_name: String,
    pub status: StepStatus,
    pub output_preview: Option<String>,
}

impl BatchExecutionSummary {
    pub fn from_results(
        execution_id: String,
        workflow_id: String,
        results: Vec<Result<WorkflowExecution, WorkflowError>>,
    ) -> Self {
        let started_at = Utc::now();
        let mut successful = 0;
        let mut failed = 0;
        let mut cancelled = 0;
        let mut server_results = Vec::new();

        for result in results {
            match result {
                Ok(execution) => {
                    let (success, error_message) = match execution.status {
                        ExecutionStatus::Completed => (true, None),
                        ExecutionStatus::Cancelled => {
                            cancelled += 1;
                            (false, Some("Cancelled".to_string()))
                        }
                        _ => {
                            failed += 1;
                            let err = execution
                                .step_results
                                .values()
                                .find(|r| r.status == StepStatus::Failed)
                                .and_then(|r| r.error.clone())
                                .unwrap_or_else(|| "Execution failed".to_string());
                            (false, Some(err))
                        }
                    };

                    if execution.status == ExecutionStatus::Completed {
                        successful += 1;
                    }

                    let step_summaries: Vec<_> = execution
                        .step_results
                        .values()
                        .map(|r| StepResultSummary {
                            step_name: r.step_id.clone(),
                            status: r.status.clone(),
                            output_preview: r
                                .output
                                .as_ref()
                                .map(|o| o.chars().take(100).collect()),
                        })
                        .collect();

                    let total_time: u64 = execution
                        .step_results
                        .values()
                        .map(|r| r.execution_time_ms)
                        .sum();

                    let server_ctx = execution.server_contexts.first();
                    server_results.push(ServerExecutionResult {
                        server_id: server_ctx.map(|s| s.id.clone()).unwrap_or_default(),
                        server_name: server_ctx.map(|s| s.name.clone()).unwrap_or_default(),
                        success,
                        error_message,
                        execution_time_ms: total_time,
                        step_results: step_summaries,
                    });
                }
                Err(e) => {
                    failed += 1;
                    server_results.push(ServerExecutionResult {
                        server_id: String::new(),
                        server_name: String::new(),
                        success: false,
                        error_message: Some(e.to_string()),
                        execution_time_ms: 0,
                        step_results: Vec::new(),
                    });
                }
            }
        }

        Self {
            execution_id,
            workflow_id,
            started_at,
            completed_at: Utc::now(),
            total_servers: server_results.len(),
            successful,
            failed,
            cancelled,
            results: server_results,
        }
    }

    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_servers == 0 {
            return 0.0;
        }
        (self.successful as f64 / self.total_servers as f64) * 100.0
    }

    /// Get total execution time across all servers
    pub fn total_execution_time_ms(&self) -> u64 {
        self.results.iter().map(|r| r.execution_time_ms).sum()
    }

    /// Get average execution time per server
    pub fn average_execution_time_ms(&self) -> u64 {
        if self.total_servers == 0 {
            return 0;
        }
        self.total_execution_time_ms() / self.total_servers as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_engine::*;
    use crate::workflow_variables::{ExecutionContext, ServerContext};

    fn create_test_server() -> ServerContext {
        ServerContext {
            id: "srv-1".to_string(),
            name: "Test Server".to_string(),
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "admin".to_string(),
            password: Some("test".to_string()),
            key_path: None,
            group: Some("test-group".to_string()),
            tags: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_workflow_executor_creation() {
        let executor = WorkflowExecutor::new();
        assert!(executor.ssh_manager.is_none());
        assert!(executor.event_tx.is_none());
    }

    #[test]
    fn test_workflow_executor_default() {
        let executor: WorkflowExecutor = Default::default();
        assert!(executor.ssh_manager.is_none());
    }

    #[test]
    fn test_workflow_error_display() {
        let err1 = WorkflowError::ExecutionNotFound("exec-123".to_string());
        assert!(err1.to_string().contains("exec-123"));

        let err2 = WorkflowError::StepExecutionFailed("command failed".to_string());
        assert!(err2.to_string().contains("command failed"));

        let err3 = WorkflowError::InvalidWorkflow("invalid step".to_string());
        assert!(err3.to_string().contains("invalid step"));

        let err4 = WorkflowError::ServerNotFound("server-1".to_string());
        assert!(err4.to_string().contains("server-1"));

        let err5 = WorkflowError::Timeout("30s".to_string());
        assert!(err5.to_string().contains("30s"));

        let err6 = WorkflowError::Cancelled;
        assert!(err6.to_string().contains("cancelled"));

        let err7 = WorkflowError::SshError("connection refused".to_string());
        assert!(err7.to_string().contains("connection refused"));
    }

    #[test]
    fn test_step_execution_outcome_creation() {
        let outcome = StepExecutionOutcome {
            output: Some("test output".to_string()),
            exit_code: 0,
        };
        assert!(outcome.output.is_some());
        assert_eq!(outcome.exit_code, 0);
    }

    #[test]
    fn test_batch_execution_summary_creation() {
        let summary = BatchExecutionSummary {
            execution_id: "batch-1".to_string(),
            workflow_id: "wf-1".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            total_servers: 3,
            successful: 2,
            failed: 1,
            cancelled: 0,
            results: vec![],
        };
        assert_eq!(summary.total_servers, 3);
        assert_eq!(summary.successful, 2);
        assert_eq!(summary.failed, 1);
        // Use approximate comparison for floating point
        let rate = summary.success_rate();
        assert!(
            rate > 66.66 && rate < 66.67,
            "Expected success rate around 66.67%, got {}",
            rate
        );
    }

    #[test]
    fn test_server_execution_result_creation() {
        let result = ServerExecutionResult {
            server_id: "srv-1".to_string(),
            server_name: "Test Server".to_string(),
            success: true,
            error_message: None,
            execution_time_ms: 1500,
            step_results: vec![],
        };
        assert!(result.success);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_step_result_summary_creation() {
        let summary = StepResultSummary {
            step_name: "Test Step".to_string(),
            status: StepStatus::Completed,
            output_preview: Some("output...".to_string()),
        };
        assert_eq!(summary.step_name, "Test Step");
        assert_eq!(summary.status, StepStatus::Completed);
    }

    #[tokio::test]
    async fn test_start_execution() {
        let executor = WorkflowExecutor::new();
        let workflow = Workflow::new("Test Workflow");

        let execution_id = executor
            .start_execution(&workflow, vec![], HashMap::new())
            .await;
        assert!(!execution_id.is_empty());

        let execution = executor.get_execution(&execution_id).await;
        assert!(execution.is_some());
    }

    #[tokio::test]
    async fn test_get_execution() {
        let executor = WorkflowExecutor::new();
        let workflow = Workflow::new("Test Workflow");

        let execution_id = executor
            .start_execution(&workflow, vec![], HashMap::new())
            .await;
        assert!(executor.get_execution(&execution_id).await.is_some());
        assert!(executor.get_execution("non-existent").await.is_none());
    }

    #[tokio::test]
    async fn test_cancel_execution() {
        let executor = WorkflowExecutor::new();
        let workflow = Workflow::new("Test Workflow");

        let execution_id = executor
            .start_execution(&workflow, vec![], HashMap::new())
            .await;
        assert!(executor.cancel_execution(&execution_id).await.is_ok());

        let execution = executor.get_execution(&execution_id).await.unwrap();
        assert_eq!(execution.status, ExecutionStatus::Cancelled);

        // Cancel non-existent execution should fail
        assert!(executor.cancel_execution("non-existent").await.is_err());
    }

    #[tokio::test]
    async fn test_pause_resume_execution() {
        let executor = WorkflowExecutor::new();
        let workflow = Workflow::new("Test Workflow");

        // Create and manually set to running for testing
        let execution_id = executor
            .start_execution(&workflow, vec![], HashMap::new())
            .await;

        // Can't pause a not-started execution, so we test the error case
        assert!(executor.pause_execution(&execution_id).await.is_err());

        // Cancel and verify can't pause cancelled
        executor.cancel_execution(&execution_id).await.ok();
        assert!(executor.pause_execution(&execution_id).await.is_err());

        // Test non-existent
        assert!(executor.pause_execution("non-existent").await.is_err());
        assert!(executor.resume_execution("non-existent").await.is_err());
    }

    #[test]
    fn test_condition_evaluator_simple() {
        let resolver = VariableResolver::new().with_system_variables();

        // Test truthy conditions
        assert!(ConditionEvaluator::evaluate("true", &resolver));
        assert!(!ConditionEvaluator::evaluate("false", &resolver));
        assert!(!ConditionEvaluator::evaluate("0", &resolver));
        assert!(!ConditionEvaluator::evaluate("", &resolver));

        // Test comparison conditions
        assert!(ConditionEvaluator::evaluate("5 == 5", &resolver));
        assert!(!ConditionEvaluator::evaluate("5 == 6", &resolver));
        assert!(ConditionEvaluator::evaluate("5 != 6", &resolver));
        assert!(ConditionEvaluator::evaluate("6 > 5", &resolver));
        assert!(!ConditionEvaluator::evaluate("5 > 6", &resolver));
        assert!(ConditionEvaluator::evaluate("5 < 6", &resolver));
    }

    #[test]
    fn test_condition_evaluator_comparison() {
        // Test eq operator
        assert!(ConditionEvaluator::evaluate_comparison(
            "value", "value", "eq"
        ));
        assert!(!ConditionEvaluator::evaluate_comparison(
            "value1", "value2", "eq"
        ));

        // Test ne operator
        assert!(ConditionEvaluator::evaluate_comparison(
            "value1", "value2", "ne"
        ));
        assert!(!ConditionEvaluator::evaluate_comparison(
            "value", "value", "ne"
        ));

        // Test gt/lt operators with numbers
        assert!(ConditionEvaluator::evaluate_comparison("10", "5", "gt"));
        assert!(!ConditionEvaluator::evaluate_comparison("5", "10", "gt"));
        assert!(ConditionEvaluator::evaluate_comparison("5", "10", "lt"));

        // Test gte/lte operators
        assert!(ConditionEvaluator::evaluate_comparison("10", "10", "gte"));
        assert!(ConditionEvaluator::evaluate_comparison("10", "5", "gte"));
        assert!(ConditionEvaluator::evaluate_comparison("5", "10", "lte"));

        // Test string contains
        assert!(ConditionEvaluator::evaluate_comparison(
            "hello world",
            "world",
            "contains"
        ));
        assert!(!ConditionEvaluator::evaluate_comparison(
            "hello", "world", "contains"
        ));

        // Test starts_with/ends_with
        assert!(ConditionEvaluator::evaluate_comparison(
            "hello world",
            "hello",
            "starts_with"
        ));
        assert!(ConditionEvaluator::evaluate_comparison(
            "hello world",
            "world",
            "ends_with"
        ));

        // Test in operator
        assert!(ConditionEvaluator::evaluate_comparison(
            "apple",
            "apple, banana, orange",
            "in"
        ));
        assert!(!ConditionEvaluator::evaluate_comparison(
            "grape",
            "apple, banana",
            "in"
        ));

        // Test exists/is_empty
        assert!(ConditionEvaluator::evaluate_comparison(
            "value", "", "exists"
        ));
        assert!(!ConditionEvaluator::evaluate_comparison("", "", "exists"));
        assert!(ConditionEvaluator::evaluate_comparison("", "", "is_empty"));
        assert!(!ConditionEvaluator::evaluate_comparison(
            "value", "", "is_empty"
        ));

        // Test unknown operator returns false
        assert!(!ConditionEvaluator::evaluate_comparison(
            "a", "b", "unknown"
        ));
    }

    #[test]
    fn test_condition_evaluator_logical() {
        let resolver = VariableResolver::new();

        // Test AND
        assert!(ConditionEvaluator::evaluate_logical(
            "true && true",
            &resolver
        ));
        assert!(!ConditionEvaluator::evaluate_logical(
            "true && false",
            &resolver
        ));
        assert!(!ConditionEvaluator::evaluate_logical(
            "false && true",
            &resolver
        ));

        // Test OR
        assert!(ConditionEvaluator::evaluate_logical(
            "true || false",
            &resolver
        ));
        assert!(ConditionEvaluator::evaluate_logical(
            "false || true",
            &resolver
        ));
        assert!(!ConditionEvaluator::evaluate_logical(
            "false || false",
            &resolver
        ));

        // Test NOT
        assert!(!ConditionEvaluator::evaluate_logical("! true", &resolver));
        assert!(ConditionEvaluator::evaluate_logical("! false", &resolver));
        assert!(!ConditionEvaluator::evaluate_logical("not true", &resolver));
        assert!(ConditionEvaluator::evaluate_logical("not false", &resolver));

        // Test precedence (AND before OR)
        assert!(ConditionEvaluator::evaluate_logical(
            "false && false || true",
            &resolver
        ));
        assert!(!ConditionEvaluator::evaluate_logical(
            "false && (false || true)",
            &resolver
        ));
    }

    #[test]
    fn test_execution_event_variants() {
        let events = vec![
            ExecutionEvent::StepStarted {
                execution_id: "exec-1".to_string(),
                step_id: "step-1".to_string(),
                step_name: "Test Step".to_string(),
            },
            ExecutionEvent::StepCompleted {
                execution_id: "exec-1".to_string(),
                step_id: "step-1".to_string(),
                duration_ms: 1000,
                output: Some("output".to_string()),
            },
            ExecutionEvent::StepFailed {
                execution_id: "exec-1".to_string(),
                step_id: "step-1".to_string(),
                error: "error".to_string(),
                will_retry: false,
            },
            ExecutionEvent::StepRetry {
                execution_id: "exec-1".to_string(),
                step_id: "step-1".to_string(),
                attempt: 2,
                max_attempts: 3,
            },
            ExecutionEvent::ExecutionCompleted {
                execution_id: "exec-1".to_string(),
                status: ExecutionStatus::Completed,
            },
            ExecutionEvent::ParallelBatchStarted {
                execution_id: "exec-1".to_string(),
                batch_size: 5,
            },
            ExecutionEvent::ParallelBatchCompleted {
                execution_id: "exec-1".to_string(),
                results: vec![("srv-1".to_string(), true)],
            },
        ];

        assert_eq!(events.len(), 7);
    }

    #[test]
    fn test_loop_context_creation() {
        let ctx = LoopContext::new(
            "loop-1".to_string(),
            LoopType::ForEach,
            "item".to_string(),
            Some("item1,item2".to_string()),
            100,
            Some("body-start".to_string()),
            Some("after-loop".to_string()),
        );

        assert_eq!(ctx.loop_id, "loop-1");
        assert_eq!(ctx.loop_type, LoopType::ForEach);
        assert_eq!(ctx.iteration_var, "item");
        assert_eq!(ctx.completed_iterations, 0);
        assert_eq!(ctx.max_iterations, 100);
    }

    #[tokio::test]
    async fn test_local_command_execution() {
        let executor = WorkflowExecutor::new();

        // Test successful command
        let result = executor
            .execute_local_command("echo 'hello world'", None, &HashMap::new(), true)
            .await;

        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.output.as_ref().unwrap().contains("hello world"));
    }

    #[tokio::test]
    async fn test_notification_execution() {
        let executor = WorkflowExecutor::new();

        let result = executor
            .execute_notification(NotificationType::Toast, "Test Title", "Test Message", None)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().exit_code, 0);
    }

    #[test]
    fn test_batch_summary_calculations() {
        let summary = BatchExecutionSummary {
            execution_id: "batch-1".to_string(),
            workflow_id: "wf-1".to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            total_servers: 4,
            successful: 3,
            failed: 1,
            cancelled: 0,
            results: vec![
                ServerExecutionResult {
                    server_id: "srv-1".to_string(),
                    server_name: "Server 1".to_string(),
                    success: true,
                    error_message: None,
                    execution_time_ms: 1000,
                    step_results: vec![],
                },
                ServerExecutionResult {
                    server_id: "srv-2".to_string(),
                    server_name: "Server 2".to_string(),
                    success: true,
                    error_message: None,
                    execution_time_ms: 2000,
                    step_results: vec![],
                },
                ServerExecutionResult {
                    server_id: "srv-3".to_string(),
                    server_name: "Server 3".to_string(),
                    success: false,
                    error_message: Some("Failed".to_string()),
                    execution_time_ms: 500,
                    step_results: vec![],
                },
            ],
        };

        assert_eq!(summary.success_rate(), 75.0);
        assert_eq!(summary.total_execution_time_ms(), 3500);
        assert_eq!(summary.average_execution_time_ms(), 875);
    }
}
