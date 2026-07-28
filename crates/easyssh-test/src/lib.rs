use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECONDS: u64 = 600;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: String,
    pub code: Option<String>,
    pub message: String,
    pub related: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ResultDocument {
    pub success: bool,
    pub operation: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub summary: String,
    pub diagnostics: Vec<Diagnostic>,
    pub artifacts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub json: bool,
    pub timeout: Duration,
    pub artifact_dir: PathBuf,
}

impl RunOptions {
    pub fn default_for_workspace(root: &Path) -> Self {
        Self {
            json: false,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
            artifact_dir: root.join("artifacts"),
        }
    }
}

pub fn workspace_root() -> Result<PathBuf> {
    let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .context("easyssh-test manifest has no workspace parent")?
        .canonicalize()?;
    let mut current = std::env::current_dir()?.canonicalize()?;
    loop {
        if current == expected {
            return Ok(expected);
        }
        current = current
            .parent()
            .context("not inside the EasySSH workspace")?
            .to_path_buf();
    }
}

pub fn validate_options(root: &Path, options: &mut RunOptions) -> Result<()> {
    if options.timeout.is_zero() {
        bail!("timeout must be greater than zero seconds");
    }
    let requested = if options.artifact_dir.is_absolute() {
        options.artifact_dir.clone()
    } else {
        root.join(&options.artifact_dir)
    };
    if !requested.starts_with(root)
        || requested
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("artifact directory must be inside the EasySSH workspace");
    }
    fs::create_dir_all(&requested)?;
    let root = root.canonicalize()?;
    let artifact_dir = requested.canonicalize()?;
    if !artifact_dir.starts_with(&root) {
        bail!("artifact directory must be inside the EasySSH workspace");
    }
    options.artifact_dir = artifact_dir;
    Ok(())
}

pub fn inspect(options: &RunOptions) -> Result<ResultDocument> {
    let root = workspace_root()?;
    let started = Instant::now();
    let metadata = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()
        .context("failed to run cargo metadata")?;
    let log = write_log(
        &options.artifact_dir,
        "inspect",
        &metadata.stdout,
        &metadata.stderr,
    )?;
    let package_count = serde_json::from_slice::<Value>(&metadata.stdout)
        .ok()
        .and_then(|value| value["packages"].as_array().map(Vec::len))
        .unwrap_or(0);
    Ok(ResultDocument {
        success: metadata.status.success(),
        operation: "project_inspect".into(),
        exit_code: metadata.status.code().unwrap_or(1),
        duration_ms: started.elapsed().as_millis(),
        summary: format!("EasySSH workspace inspected ({package_count} packages)"),
        diagnostics: parse_cargo_messages(&String::from_utf8_lossy(&metadata.stdout)),
        artifacts: vec![safe_path(&root, &log)],
        warnings: vec![],
    })
}

pub fn run(operation: &str, options: &RunOptions) -> Result<ResultDocument> {
    let root = workspace_root()?;
    let (args, cargo_json) = match operation {
        "format_check" => (vec!["fmt", "--all", "--check"], false),
        "clippy" => (
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--message-format=json-diagnostic-rendered-ansi",
                "--",
                "-D",
                "warnings",
            ],
            true,
        ),
        "unit_tests" => (
            vec![
                "test",
                "--workspace",
                "--all-features",
                "--message-format=json-diagnostic-rendered-ansi",
            ],
            true,
        ),
        "build_debug" => (
            vec![
                "build",
                "--workspace",
                "--message-format=json-diagnostic-rendered-ansi",
            ],
            true,
        ),
        "build_release" => (
            vec![
                "build",
                "--workspace",
                "--release",
                "--message-format=json-diagnostic-rendered-ansi",
            ],
            true,
        ),
        _ => bail!("operation is not allowlisted"),
    };
    let started = Instant::now();
    let output = execute_cargo(&root, &args, options.timeout)?;
    let log = write_log(
        &options.artifact_dir,
        operation,
        &output.stdout,
        &output.stderr,
    )?;
    let diagnostics = if cargo_json {
        parse_cargo_messages(&String::from_utf8_lossy(&output.stdout))
    } else {
        parse_plain_failure(&String::from_utf8_lossy(&output.stderr))
    };
    let timed_out = output.timed_out;
    Ok(ResultDocument {
        success: output.success && !timed_out,
        operation: operation.into(),
        exit_code: if timed_out { 124 } else { output.exit_code },
        duration_ms: started.elapsed().as_millis(),
        summary: if timed_out {
            format!("{operation} timed out")
        } else if output.success {
            format!("{operation} completed")
        } else {
            format!("{operation} failed")
        },
        diagnostics,
        artifacts: vec![safe_path(&root, &log)],
        warnings: if timed_out {
            vec!["subprocess terminated after timeout".into()]
        } else {
            vec![]
        },
    })
}

struct ProcessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    success: bool,
    exit_code: i32,
    timed_out: bool,
}

fn execute_cargo(root: &Path, args: &[&str], timeout: Duration) -> Result<ProcessOutput> {
    let child = Command::new("cargo")
        .args(args)
        .current_dir(root)
        // On Windows the running CLI executable is locked. A dedicated target
        // directory prevents Cargo from trying to replace that executable.
        .env(
            "CARGO_TARGET_DIR",
            root.join("target").join("easyssh-test-run"),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start allowlisted cargo command")?;
    wait_for_child(child, timeout)
}

fn wait_for_child(mut child: Child, timeout: Duration) -> Result<ProcessOutput> {
    let mut stdout = child.stdout.take().context("child stdout unavailable")?;
    let mut stderr = child.stderr.take().context("child stderr unavailable")?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            terminate_child(&mut child)?;
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(ProcessOutput {
        stdout: stdout_reader.join().unwrap_or_default(),
        stderr: stderr_reader.join().unwrap_or_default(),
        success: status.success(),
        exit_code: status.code().unwrap_or(1),
        timed_out,
    })
}

fn terminate_child(child: &mut Child) -> Result<()> {
    #[cfg(windows)]
    {
        // `taskkill /T` also stops any Cargo compiler children spawned by this controlled process.
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    Ok(())
}

fn write_log(dir: &Path, operation: &str, stdout: &[u8], stderr: &[u8]) -> Result<PathBuf> {
    let path = dir.join(format!(
        "{}-{}.log",
        operation,
        Utc::now().format("%Y%m%d-%H%M%S%.3f")
    ));
    fs::write(
        &path,
        format!(
            "{}\n{}",
            redact(&String::from_utf8_lossy(stdout)),
            redact(&String::from_utf8_lossy(stderr))
        ),
    )?;
    Ok(path)
}

pub fn redact(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if [
                "password",
                "private key",
                "token",
                "authorization",
                "connection string",
            ]
            .iter()
            .any(|term| lower.contains(term))
            {
                "[redacted]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_cargo_messages(output: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for line in output.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value["reason"] != "compiler-message" {
            continue;
        }
        let message = &value["message"];
        let level = message["level"].as_str().unwrap_or("error");
        let primary = message["spans"]
            .as_array()
            .and_then(|spans| spans.iter().find(|span| span["is_primary"] == true));
        let related = message["children"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|child| child["message"].as_str())
            .map(str::to_owned)
            .collect();
        diagnostics.push(Diagnostic {
            file: primary
                .and_then(|span| span["file_name"].as_str())
                .map(str::to_owned),
            line: primary
                .and_then(|span| span["line_start"].as_u64())
                .map(|n| n as u32),
            column: primary
                .and_then(|span| span["column_start"].as_u64())
                .map(|n| n as u32),
            severity: level.into(),
            code: message["code"]["code"].as_str().map(str::to_owned),
            message: message["message"]
                .as_str()
                .unwrap_or("Cargo diagnostic")
                .into(),
            related,
        });
    }
    diagnostics
}

fn parse_plain_failure(stderr: &str) -> Vec<Diagnostic> {
    stderr
        .lines()
        .filter(|line| line.contains("error") || line.contains("warning"))
        .take(50)
        .map(|line| Diagnostic {
            file: None,
            line: None,
            column: None,
            severity: if line.contains("error") {
                "error"
            } else {
                "warning"
            }
            .into(),
            code: None,
            message: line.trim().into(),
            related: vec![],
        })
        .collect()
}

fn safe_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPILER_FAILURE_FIXTURE: &str = r#"{"reason":"compiler-message","message":{"message":"cannot find value `missing` in this scope","code":{"code":"E0425"},"level":"error","spans":[{"file_name":"fixtures/broken.rs","line_start":7,"column_start":9,"is_primary":true}],"children":[{"message":"not found in this scope"}]}}"#;

    #[test]
    fn compiler_failure_fixture_has_location_and_code() {
        let diagnostics = parse_cargo_messages(COMPILER_FAILURE_FIXTURE);
        assert_eq!(diagnostics[0].file.as_deref(), Some("fixtures/broken.rs"));
        assert_eq!(diagnostics[0].line, Some(7));
        assert_eq!(diagnostics[0].code.as_deref(), Some("E0425"));
        assert_eq!(diagnostics[0].related, ["not found in this scope"]);
    }

    #[test]
    fn redaction_removes_sensitive_log_lines() {
        let result = redact("ok\npassword=secret\nprivate key: data\nToken: abc");
        assert!(!result.contains("secret"));
        assert!(!result.contains("data"));
        assert!(!result.contains("abc"));
    }

    #[test]
    fn timeout_terminates_a_child_process() {
        #[cfg(windows)]
        let child = Command::new("cmd")
            .args(["/C", "ping -n 10 127.0.0.1 > NUL"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        #[cfg(not(windows))]
        let child = Command::new("sh")
            .args(["-c", "sleep 10"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let output = wait_for_child(child, Duration::from_millis(50)).unwrap();
        assert!(output.timed_out);
    }

    #[test]
    fn artifact_directory_outside_workspace_is_rejected() {
        let root = workspace_root().unwrap();
        let mut options = RunOptions {
            json: true,
            timeout: Duration::from_secs(1),
            artifact_dir: root.parent().unwrap().join("outside-artifacts"),
        };
        assert!(validate_options(&root, &mut options).is_err());
    }
}
