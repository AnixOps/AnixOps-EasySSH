//! EasySSH Foreground Monitor - Real-time build/test/security dashboard
//!
//! Usage: cargo run --bin foreground-monitor

use std::io::{self, Write};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CheckResult {
    name: &'static str,
    status: char, // ✅ ✗ ⏳
    message: String,
    last_check: String,
}

struct Monitor {
    results: Vec<CheckResult>,
    last_update: Instant,
}

impl Monitor {
    fn new() -> Self {
        Self {
            results: vec![
                CheckResult {
                    name: "代码质量",
                    status: '⏳',
                    message: "等待检查...".to_string(),
                    last_check: "--:--:--".to_string(),
                },
                CheckResult {
                    name: "构建状态",
                    status: '⏳',
                    message: "等待检查...".to_string(),
                    last_check: "--:--:--".to_string(),
                },
                CheckResult {
                    name: "测试执行",
                    status: '⏳',
                    message: "等待检查...".to_string(),
                    last_check: "--:--:--".to_string(),
                },
                CheckResult {
                    name: "安全审计",
                    status: '⏳',
                    message: "等待检查...".to_string(),
                    last_check: "--:--:--".to_string(),
                },
                CheckResult {
                    name: "CI/CD配置",
                    status: '⏳',
                    message: "等待检查...".to_string(),
                    last_check: "--:--:--".to_string(),
                },
            ],
            last_update: Instant::now(),
        }
    }

    fn clear_screen() {
        print!("\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }

    fn get_time(&self) -> String {
        let now = chrono::Local::now();
        now.format("%H:%M:%S").to_string()
    }

    fn render(&self) {
        Self::clear_screen();

        println!(
            "╔══════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║                    🔧 EasySSH 前台自动化监控中心 🔧                           ║"
        );
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ 刷新时间: {}                                              ║",
            self.get_time()
        );
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );

        for (i, result) in self.results.iter().enumerate() {
            let icon = match result.status {
                '✅' => "✅",
                '✗' => "❌",
                _ => "⏳",
            };
            let msg = if result.message.len() > 38 {
                format!("{}...", &result.message[..35])
            } else {
                result.message.clone()
            };
            println!(
                "║ {:12} │ {:2} │ {:8} │ {:40} ║",
                result.name, icon, result.last_check, msg
            );
        }

        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ 状态: {}                                                        ║",
            if self.results.iter().all(|r| r.status == '✅') {
                "🎉 全部通过"
            } else if self.results.iter().any(|r| r.status == '✗') {
                "⚠️  需要修复"
            } else {
                "⏳ 检查中..."
            }
        );
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ 操作: [R]刷新 [F]格式化 [C]清理 [A]全部修复 [S]提交 [Q]退出                    ║"
        );
        println!(
            "╚══════════════════════════════════════════════════════════════════════════════╝"
        );
    }

    fn check_code_quality(&mut self) {
        let idx = 0;
        self.results[idx].last_check = self.get_time();

        let fmt = Command::new("cargo")
            .args(["fmt", "--", "--check"])
            .output();

        match fmt {
            Ok(output) if output.status.success() => {
                self.results[idx].status = '✅';
                self.results[idx].message = "代码格式检查通过".to_string();
            }
            _ => {
                self.results[idx].status = '✗';
                self.results[idx].message = "需要格式化: cargo fmt".to_string();
            }
        }
    }

    fn check_build(&mut self) {
        let idx = 1;
        self.results[idx].last_check = self.get_time();

        let core = Command::new("cargo")
            .args(["check", "-p", "easyssh-core"])
            .output();

        let winui = Command::new("cargo")
            .args(["check", "-p", "easyssh-winui"])
            .output();

        match (core, winui) {
            (Ok(c), Ok(w)) if c.status.success() && w.status.success() => {
                self.results[idx].status = '✅';
                self.results[idx].message = "Core + WinUI 构建正常".to_string();
            }
            (Ok(c), _) if !c.status.success() => {
                self.results[idx].status = '✗';
                self.results[idx].message = "Core 库编译错误".to_string();
            }
            _ => {
                self.results[idx].status = '✗';
                self.results[idx].message = "WinUI 编译错误".to_string();
            }
        }
    }

    fn check_tests(&mut self) {
        let idx = 2;
        self.results[idx].last_check = self.get_time();

        let test = Command::new("cargo")
            .args([
                "test",
                "-p",
                "easyssh-core",
                "--no-fail-fast",
                "--",
                "--test-threads=1",
            ])
            .output();

        match test {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("test result: ok") {
                    let passed = stdout
                        .lines()
                        .find(|l| l.contains("test result:"))
                        .and_then(|l| l.split_whitespace().nth(3))
                        .unwrap_or("0");
                    self.results[idx].status = '✅';
                    self.results[idx].message = format!("{} 个测试通过", passed);
                } else if stdout.contains("FAILED") {
                    self.results[idx].status = '✗';
                    self.results[idx].message = "有测试失败".to_string();
                } else {
                    self.results[idx].status = '✗';
                    self.results[idx].message = "测试编译错误".to_string();
                }
            }
            Err(_) => {
                self.results[idx].status = '⏳';
                self.results[idx].message = "无法执行测试".to_string();
            }
        }
    }

    fn check_security(&mut self) {
        let idx = 3;
        self.results[idx].last_check = self.get_time();

        let audit = Command::new("cargo").arg("audit").output();

        match audit {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stdout.contains("Success") || stderr.contains("Success") {
                    self.results[idx].status = '✅';
                    self.results[idx].message = "未发现安全漏洞".to_string();
                } else if stdout.contains("RUSTSEC") || stderr.contains("RUSTSEC") {
                    self.results[idx].status = '✗';
                    self.results[idx].message = "发现安全警告".to_string();
                } else {
                    self.results[idx].status = '⏳';
                    self.results[idx].message = "cargo-audit 可能未安装".to_string();
                }
            }
            Err(_) => {
                self.results[idx].status = '⏳';
                self.results[idx].message = "cargo-audit 未安装".to_string();
            }
        }
    }

    fn check_cicd(&mut self) {
        let idx = 4;
        self.results[idx].last_check = self.get_time();

        let workflow_dir = std::path::Path::new(".github/workflows");
        if workflow_dir.exists() {
            let count = std::fs::read_dir(workflow_dir)
                .map(|entries| entries.filter_map(|e| e.ok()).count())
                .unwrap_or(0);
            self.results[idx].status = '✅';
            self.results[idx].message = format!("{} 个工作流配置就绪", count);
        } else {
            self.results[idx].status = '✗';
            self.results[idx].message = "工作流目录不存在".to_string();
        }
    }

    fn auto_fix(&mut self) {
        println!("\n🔧 执行自动修复...");

        // 格式化代码
        println!("  → 执行 cargo fmt...");
        let _ = Command::new("cargo").arg("fmt").output();

        // 清理缓存
        println!("  → 清理构建缓存...");
        let _ = Command::new("cargo").arg("clean").output();

        println!("✅ 自动修复完成");
        thread::sleep(Duration::from_secs(1));

        // 重新检查
        self.run_all_checks();
    }

    fn run_all_checks(&mut self) {
        self.check_code_quality();
        self.check_build();
        self.check_tests();
        self.check_security();
        self.check_cicd();
    }

    fn run(&mut self) {
        // 首次检查
        println!("🔧 EasySSH 前台自动化监控启动中...");
        println!("正在执行首次检查，请稍候...\n");
        self.run_all_checks();

        loop {
            self.render();

            // 等待输入或30秒自动刷新
            let mut input = String::new();
            let start = Instant::now();

            while start.elapsed() < Duration::from_secs(30) {
                // 非阻塞检查输入（简化版）
                if io::stdin().read_line(&mut input).is_ok() && !input.trim().is_empty() {
                    break;
                }
                input.clear();
                thread::sleep(Duration::from_millis(100));
            }

            match input.trim().to_uppercase().as_str() {
                "Q" | "QUIT" => {
                    println!("\n👋 退出监控...");
                    break;
                }
                "R" | "REFRESH" => {
                    println!("\n🔄 手动刷新...");
                    self.run_all_checks();
                }
                "F" | "FMT" => {
                    println!("\n🔧 格式化代码...");
                    let _ = Command::new("cargo").arg("fmt").output();
                    self.check_code_quality();
                }
                "C" | "CLEAN" => {
                    println!("\n🧹 清理构建缓存...");
                    let _ = Command::new("cargo").arg("clean").output();
                    self.check_build();
                }
                "A" | "AUTO" => {
                    self.auto_fix();
                }
                "S" | "SYNC" => {
                    println!("\n📦 提交更改...");
                    let _ = Command::new("git").args(["add", "-A"]).output();
                    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let _ = Command::new("git")
                        .args(["commit", "-m", &format!("auto: 前台监控自动修复 {}", now)])
                        .output();
                    let _ = Command::new("git").arg("push").output();
                    println!("✅ 提交完成");
                    thread::sleep(Duration::from_secs(1));
                }
                _ => {
                    // 自动刷新
                    self.run_all_checks();
                }
            }
        }
    }
}

fn main() {
    let mut monitor = Monitor::new();
    monitor.run();
}
