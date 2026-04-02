use std::process::{Command, Stdio};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::Utc;

/// Build log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildLogEntry {
    pub timestamp: String,
    pub level: String,
    pub file: String,
    pub line: u32,
    pub error_code: String,
    pub message: String,
    pub category: ErrorCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCategory {
    BorrowChecker,    // E0500, E0502, etc.
    TypeMismatch,     // E0308
    MissingField,     // E0063
    MethodNotFound,   // E0599
    TraitNotImpl,     // E0277
    ImportError,      // E0432
    GenericError,     // Other E0xxx
    Warning,          // Wxxxx
    Unknown,
}

/// Build report for a single compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildReport {
    pub version: String,
    pub features: Vec<String>,
    pub timestamp: String,
    pub duration_ms: u64,
    pub success: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub errors: Vec<BuildLogEntry>,
    pub fix_attempts: Vec<FixAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixAttempt {
    pub error_index: usize,
    pub agent_id: String,
    pub strategy: String,
    pub success: bool,
    pub iterations: u32,
}

/// Automated build system with self-healing capabilities
pub struct AutoBuildSystem {
    logs_dir: String,
    reports: Arc<Mutex<Vec<BuildReport>>>,
    max_iterations: u32,
}

impl AutoBuildSystem {
    pub fn new(logs_dir: &str) -> Self {
        fs::create_dir_all(logs_dir).ok();
        Self {
            logs_dir: logs_dir.to_string(),
            reports: Arc::new(Mutex::new(Vec::new())),
            max_iterations: 5,
        }
    }

    /// Build a specific version with auto-fix
    pub fn build_version(&self, version: &str, features: &[String]) -> BuildReport {
        let start = Instant::now();
        let timestamp = Utc::now().to_rfc3339();

        println!("🚀 Starting build for {} with features: {:?}", version, features);

        let mut report = BuildReport {
            version: version.to_string(),
            features: features.to_vec(),
            timestamp: timestamp.clone(),
            duration_ms: 0,
            success: false,
            error_count: 0,
            warning_count: 0,
            errors: Vec::new(),
            fix_attempts: Vec::new(),
        };

        // Attempt build with retries
        for iteration in 0..self.max_iterations {
            println!("📦 Build attempt {}/{}...", iteration + 1, self.max_iterations);

            let output = self.run_cargo_build(version, features);
            let (errors, warnings, success) = self.parse_build_output(&output);

            report.error_count = errors.len();
            report.warning_count = warnings.len();
            report.success = success;
            report.errors = errors.clone();

            if success {
                println!("✅ Build successful!");
                break;
            }

            println!("❌ Build failed with {} errors", errors.len());

            // Save build log for analysis
            self.save_build_log(version, iteration, &output);

            // Trigger auto-fix agents
            if iteration < self.max_iterations - 1 {
                println!("🔧 Triggering auto-fix agents...");
                let fixed = self.trigger_auto_fix(&errors, version, iteration);
                if !fixed {
                    println!("⚠️  Auto-fix could not resolve all errors");
                }
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;

        // Save final report
        self.save_report(&report);

        report
    }

    fn run_cargo_build(&self, version: &str, features: &[String]) -> String {
        let features_str = if features.is_empty() {
            "--no-default-features".to_string()
        } else {
            format!("--features={}", features.join(","))
        };

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--release")
            .arg("--bin")
            .arg(version)
            .arg(&features_str)
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().expect("Failed to execute cargo build");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        format!("{}\n{}", stdout, stderr)
    }

    fn parse_build_output(&self, output: &str) -> (Vec<BuildLogEntry>, Vec<BuildLogEntry>, bool) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut success = false;

        for line in output.lines() {
            // Check for success
            if line.contains("Finished") && line.contains("release") {
                success = true;
            }

            // Parse error lines
            // Format: error[E0xxx]: message at file.rs:123:45
            if line.starts_with("error[") {
                if let Some(entry) = self.parse_error_line(line) {
                    errors.push(entry);
                }
            }

            // Parse warning lines
            if line.starts_with("warning:") {
                if let Some(entry) = self.parse_warning_line(line) {
                    warnings.push(entry);
                }
            }
        }

        (errors, warnings, success)
    }

    fn parse_error_line(&self, line: &str) -> Option<BuildLogEntry> {
        // Parse: error[E0502]: cannot borrow... --> file.rs:123:45
        let error_code = line.split('[').nth(1)?.split(']').next()?.to_string();
        let message = line.split("]: ").nth(1)?.split(" -->").next()?.to_string();

        let category = match error_code.as_str() {
            "E0500" | "E0502" | "E0501" | "E0499" => ErrorCategory::BorrowChecker,
            "E0308" => ErrorCategory::TypeMismatch,
            "E0063" => ErrorCategory::MissingField,
            "E0599" => ErrorCategory::MethodNotFound,
            "E0277" => ErrorCategory::TraitNotImpl,
            "E0432" => ErrorCategory::ImportError,
            _ if error_code.starts_with('E') => ErrorCategory::GenericError,
            _ => ErrorCategory::Unknown,
        };

        Some(BuildLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: "error".to_string(),
            file: "unknown".to_string(),
            line: 0,
            error_code,
            message,
            category,
        })
    }

    fn parse_warning_line(&self, line: &str) -> Option<BuildLogEntry> {
        Some(BuildLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: "warning".to_string(),
            file: "unknown".to_string(),
            line: 0,
            error_code: "W0000".to_string(),
            message: line.to_string(),
            category: ErrorCategory::Warning,
        })
    }

    fn save_build_log(&self, version: &str, iteration: u32, output: &str) {
        let filename = format!("{}/build_{}_iter_{}.log", self.logs_dir, version, iteration);
        fs::write(&filename, output).ok();
    }

    fn save_report(&self, report: &BuildReport) {
        let filename = format!("{}/report_{}_{}.json",
            self.logs_dir,
            report.version.replace(" ", "_"),
            Utc::now().timestamp()
        );
        let json = serde_json::to_string_pretty(report).unwrap();
        fs::write(&filename, json).ok();
    }

    fn trigger_auto_fix(&self, errors: &[BuildLogEntry], version: &str, iteration: u32) -> bool {
        // Group errors by category for targeted fixes
        let mut by_category: HashMap<ErrorCategory, Vec<&BuildLogEntry>> = HashMap::new();
        for error in errors {
            by_category.entry(error.category.clone()).or_default().push(error);
        }

        println!("  Error breakdown:");
        for (cat, errs) in &by_category {
            println!("    - {:?}: {} errors", cat, errs.len());
        }

        // In a real system, this would spawn agents
        // For now, return false to indicate manual intervention needed
        false
    }

    /// Build all three versions: Lite, Standard, Pro
    pub fn build_all_versions(&self) -> Vec<BuildReport> {
        println!("🏗️  Building all EasySSH versions...\n");

        let mut reports = Vec::new();

        // Lite version
        println!("╔════════════════════════════════════╗");
        println!("║     Building EasySSH Lite          ║");
        println!("╚════════════════════════════════════╝");
        let lite_report = self.build_version("EasySSH-Lite", &[]);
        reports.push(lite_report);
        println!();

        // Standard version
        println!("╔════════════════════════════════════╗");
        println!("║    Building EasySSH Standard       ║");
        println!("╚════════════════════════════════════╝");
        let standard_features = vec![
            "embedded-terminal".to_string(),
            "split-screen".to_string(),
            "sftp".to_string(),
            "monitoring".to_string(),
        ];
        let standard_report = self.build_version("EasySSH-Standard", &standard_features);
        reports.push(standard_report);
        println!();

        // Pro version
        println!("╔════════════════════════════════════╗");
        println!("║     Building EasySSH Pro           ║");
        println!("╚════════════════════════════════════╝");
        let pro_features = vec![
            "standard".to_string(),
            "team".to_string(),
            "audit".to_string(),
            "sync".to_string(),
        ];
        let pro_report = self.build_version("EasySSH-Pro", &pro_features);
        reports.push(pro_report);
        println!();

        // Print summary
        println!("╔════════════════════════════════════════════════╗");
        println!("║           BUILD SUMMARY                        ║");
        println!("╠════════════════════════════════════════════════╣");
        for report in &reports {
            let status = if report.success { "✅ PASS" } else { "❌ FAIL" };
            let errors = if report.success { 0 } else { report.error_count };
            println!("║ {:20} | {:8} | {:4} errors ║",
                report.version, status, errors);
        }
        println!("╚════════════════════════════════════════════════╝");

        reports
    }
}

/// Agent task for fixing specific error categories
pub struct FixAgent {
    pub agent_id: String,
    pub specialization: ErrorCategory,
    pub max_iterations: u32,
}

impl FixAgent {
    pub fn new(id: &str, spec: ErrorCategory) -> Self {
        Self {
            agent_id: id.to_string(),
            specialization: spec,
            max_iterations: 3,
        }
    }

    pub fn attempt_fix(&self, error: &BuildLogEntry, _version: &str) -> bool {
        println!("  🤖 Agent {} fixing {:?} error in {}",
            self.agent_id, self.specialization, error.file);

        // In a real system, this would:
        // 1. Read the source file
        // 2. Analyze the error context
        // 3. Apply the appropriate fix pattern
        // 4. Verify the fix compiles

        match self.specialization {
            ErrorCategory::BorrowChecker => {
                // Apply borrow checker fix patterns
                println!("     → Applying borrow checker fix pattern");
                true
            }
            ErrorCategory::TypeMismatch => {
                // Apply type fix patterns
                println!("     → Applying type mismatch fix pattern");
                true
            }
            _ => {
                println!("     → No automated fix available");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let system = AutoBuildSystem::new("./test_logs");

        let borrow_error = "error[E0502]: cannot borrow...".to_string();
        let entry = system.parse_error_line(&borrow_error);

        assert!(entry.is_some());
        assert_eq!(entry.unwrap().category, ErrorCategory::BorrowChecker);
    }
}
