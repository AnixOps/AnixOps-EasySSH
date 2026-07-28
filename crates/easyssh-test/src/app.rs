use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
pub struct AppStatus {
    pub success: bool,
    pub run_id: String,
    pub pid: u32,
    pub state: String,
    pub window: Option<WindowInfo>,
    pub log_file: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub struct ManagedApp {
    pub status: AppStatus,
    root: PathBuf,
    token: String,
    child: Child,
}

pub fn launch_ui_test(workspace: &Path, timeout: Duration) -> Result<ManagedApp> {
    let run_id = format!(
        "run-{}-{}",
        Utc::now().format("%Y%m%d%H%M%S%3f"),
        std::process::id()
    );
    let root = workspace.join("artifacts").join("runs").join(&run_id);
    fs::create_dir_all(&root)?;
    let target = workspace.join("target").join("easyssh-test-run");
    let build = Command::new("cargo")
        .args(["build", "-p", "easyssh-desktop", "--features", "ui-test"])
        .current_dir(workspace)
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .context("failed to build ui-test application")?;
    if !build.status.success() {
        bail!("ui-test application build failed");
    }
    let executable = target.join("debug").join(if cfg!(windows) {
        "easyssh.exe"
    } else {
        "easyssh"
    });
    let token = format!(
        "easyssh-ui-test-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    fs::create_dir_all(root.join("logs"))?;
    let stdout = File::create(root.join("logs").join("app.stdout.log"))?;
    let stderr = File::create(root.join("logs").join("app.stderr.log"))?;
    let child = Command::new(&executable)
        .args([
            "--ui-test-root",
            root.to_string_lossy().as_ref(),
            "--ui-test-token",
            &token,
        ])
        .current_dir(workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .with_context(|| format!("failed to launch {}", executable.display()))?;
    let pid = child.id();
    let mut app = ManagedApp {
        status: AppStatus {
            success: false,
            run_id,
            pid,
            state: "starting".into(),
            window: None,
            log_file: relative(workspace, &root.join("logs").join("app.log")),
            summary: "application starting".into(),
        },
        root,
        token,
        child,
    };
    let started = Instant::now();
    while started.elapsed() < timeout {
        if app.child.try_wait()?.is_some() {
            app.status.state = "crashed".into();
            app.status.summary = "application exited before ready".into();
            return Ok(app);
        }
        if let Some(window) = app.read_ready()? {
            app.status.success = true;
            app.status.state = "ready".into();
            app.status.window = Some(window);
            app.status.summary = "application ready".into();
            return Ok(app);
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = app.child.kill();
    app.status.state = "timed_out".into();
    app.status.summary = "application did not become ready before timeout".into();
    Ok(app)
}

impl ManagedApp {
    pub fn get_status(&mut self) -> Result<AppStatus> {
        if self.child.try_wait()?.is_some() && self.status.state == "ready" {
            self.status.success = false;
            self.status.state = "crashed".into();
            self.status.summary = "application exited unexpectedly".into();
        }
        Ok(self.status.clone())
    }

    pub fn stop(&mut self, timeout: Duration) -> Result<AppStatus> {
        if matches!(
            self.status.state.as_str(),
            "stopped" | "crashed" | "timed_out"
        ) {
            return Ok(self.status.clone());
        }
        fs::write(self.root.join("stop.request"), &self.token)?;
        let started = Instant::now();
        while started.elapsed() < timeout {
            if self.child.try_wait()?.is_some() {
                self.status.success = true;
                self.status.state = "stopped".into();
                self.status.summary = "application stopped gracefully".into();
                return Ok(self.status.clone());
            }
            thread::sleep(Duration::from_millis(50));
        }
        self.child.kill()?;
        self.status.success = true;
        self.status.state = "stopped".into();
        self.status.summary = "application force-stopped after graceful timeout".into();
        Ok(self.status.clone())
    }

    fn read_ready(&self) -> Result<Option<WindowInfo>> {
        let path = self.root.join("metadata.json");
        if !path.is_file() {
            return Ok(None);
        }
        let value: Value = serde_json::from_slice(&fs::read(path)?)?;
        if value["state"] != "ready" || value["token"].as_str() != Some(&self.token) {
            return Ok(None);
        }
        Ok(Some(WindowInfo {
            title: value["title"]
                .as_str()
                .unwrap_or("EasySSH [UI Test]")
                .into(),
            width: value["width"].as_u64().unwrap_or(1280) as u32,
            height: value["height"].as_u64().unwrap_or(800) as u32,
        }))
    }
}

fn relative(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
