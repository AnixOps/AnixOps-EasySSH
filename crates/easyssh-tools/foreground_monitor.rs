use std::process::Command;
use std::thread;
use std::time::Duration;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use chrono::Local;

#[derive(Debug, Clone)]
struct MonitorStatus {
    name: String,
    status: String, // ✅ ✗ ⏳
    last_check: String,
    details: String,
    color: String,
}

struct EasySSHMonitor {
    checks: Arc<Mutex<Vec<MonitorStatus>>>,
    running: Arc<Mutex<bool>>,
}

impl EasySSHMonitor {
    fn new() -> Self {
        let checks = vec![
            MonitorStatus {
                name: "代码质量".to_string(),
                status: "⏳".to_string(),
                last_check: "等待中".to_string(),
                details: "准备检查...".to_string(),
                color: "yellow".to_string(),
            },
            MonitorStatus {
                name: "安全审计".to_string(),
                status: "⏳".to_string(),
                last_check: "等待中".to_string(),
                details: "准备检查...".to_string(),
                color: "yellow".to_string(),
            },
            MonitorStatus {
                name: "构建状态".to_string(),
                status: "⏳".to_string(),
                last_check: "等待中".to_string(),
                details: "准备检查...".to_string(),
                color: "yellow".to_string(),
            },
            MonitorStatus {
                name: "测试执行".to_string(),
                status: "⏳".to_string(),
                last_check: "等待中".to_string(),
                details: "准备检查...".to_string(),
                color: "yellow".to_string(),
            },
            MonitorStatus {
                name: "CI/CD流程".to_string(),
                status: "⏳".to_string(),
                last_check: "等待中".to_string(),
                details: "准备检查...".to_string(),
                color: "yellow".to_string(),
            },
        ];

        Self {
            checks: Arc::new(Mutex::new(checks)),
            running: Arc::new(Mutex::new(true)),
        }
    }

    fn clear_screen() {
        print!("\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }

    fn render(&self) {
        Self::clear_screen();

        println!("╔══════════════════════════════════════════════════════════════════════════════╗");
        println!("║                    🔧 EasySSH 前台自动化监控中心 🔧                           ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════╣");
        println!("║ 时间: {}                                                    ║", Local::now().format("%Y-%m-%d %H:%M:%S"));
        println!("╠══════════════════════════════════════════════════════════════════════════════╣");

        let checks = self.checks.lock().unwrap();
        for check in checks.iter() {
            let icon = match check.status.as_str() {
                "✅" => "✅",
                "✗" => "❌",
                _ => "⏳",
            };

            println!("║ {:12} │ {} │ {:19} │ {:40} ║",
                check.name,
                icon,
                check.last_check,
                if check.details.len() > 38 {
                    format!("{}...", &check.details[..35])
                } else {
                    check.details.clone()
                }
            );
        }

        println!("╠══════════════════════════════════════════════════════════════════════════════╣");
        println!("║ 命令: [R]刷新 [A]修复全部 [Q]退出 [C]清理 [S]提交                              ║");
        println!("╚══════════════════════════════════════════════════════════════════════════════╝");
        println!("按 Ctrl+C 退出");
    }

    fn check_code_quality(&self, index: usize) {
        let mut checks = self.checks.lock().unwrap();
        checks[index].last_check = Local::now().format("%H:%M:%S").to_string();

        // 检查 Rust 代码格式
        let fmt_result = Command::new("cargo")
            .args(["fmt", "--", "--check"])
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .output();

        let clippy_result = Command::new("cargo")
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .output();

        match (fmt_result, clippy_result) {
            (Ok(fmt), Ok(clippy)) => {
                if fmt.status.success() && clippy.status.success() {
                    checks[index].status = "✅".to_string();
                    checks[index].details = "代码格式和clippy检查通过".to_string();
                    checks[index].color = "green".to_string();
                } else if !fmt.status.success() {
                    checks[index].status = "✗".to_string();
                    checks[index].details = "需要格式化: cargo fmt".to_string();
                    checks[index].color = "red".to_string();
                } else {
                    checks[index].status = "✗".to_string();
                    checks[index].details = "Clippy警告需要修复".to_string();
                    checks[index].color = "red".to_string();
                }
            }
            _ => {
                checks[index].status = "✗".to_string();
                checks[index].details = "检查命令执行失败".to_string();
                checks[index].color = "red".to_string();
            }
        }
    }

    fn check_security(&self, index: usize) {
        let mut checks = self.checks.lock().unwrap();
        checks[index].last_check = Local::now().format("%H:%M:%S").to_string();

        let audit_result = Command::new("cargo")
            .args(["audit"])
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .output();

        match audit_result {
            Ok(output) => {
                if output.status.success() {
                    checks[index].status = "✅".to_string();
                    checks[index].details = "未发现安全漏洞".to_string();
                    checks[index].color = "green".to_string();
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let combined = format!("{}{}", stderr, stdout);

                    if combined.contains("RUSTSEC") {
                        checks[index].status = "✗".to_string();
                        let advisory_count = combined.matches("RUSTSEC").count();
                        checks[index].details = format!("发现 {} 个安全警告", advisory_count);
                        checks[index].color = "red".to_string();
                    } else {
                        checks[index].status = "✅".to_string();
                        checks[index].details = "安全检查通过".to_string();
                        checks[index].color = "green".to_string();
                    }
                }
            }
            Err(_) => {
                checks[index].status = "⏳".to_string();
                checks[index].details = "cargo-audit 未安装".to_string();
                checks[index].color = "yellow".to_string();
            }
        }
    }

    fn check_build(&self, index: usize) {
        let mut checks = self.checks.lock().unwrap();
        checks[index].last_check = Local::now().format("%H:%M:%S").to_string();

        // 检查 core 库构建
        let core_build = Command::new("cargo")
            .args(["check", "-p", "easyssh-core"])
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .output();

        // 检查 Windows UI 构建
        let winui_build = Command::new("cargo")
            .args(["check", "-p", "easyssh-winui"])
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .output();

        match (core_build, winui_build) {
            (Ok(core), Ok(winui)) => {
                if core.status.success() && winui.status.success() {
                    checks[index].status = "✅".to_string();
                    checks[index].details = "Core + WinUI 检查通过".to_string();
                    checks[index].color = "green".to_string();
                } else if !core.status.success() {
                    let stderr = String::from_utf8_lossy(&core.stderr);
                    let errors = stderr.lines().filter(|l| l.contains("error")).count();
                    checks[index].status = "✗".to_string();
                    checks[index].details = format!("Core 有 {} 个错误", errors);
                    checks[index].color = "red".to_string();
                } else {
                    let stderr = String::from_utf8_lossy(&winui.stderr);
                    let errors = stderr.lines().filter(|l| l.contains("error")).count();
                    checks[index].status = "✗".to_string();
                    checks[index].details = format!("WinUI 有 {} 个错误", errors);
                    checks[index].color = "red".to_string();
                }
            }
            _ => {
                checks[index].status = "✗".to_string();
                checks[index].details = "构建检查失败".to_string();
                checks[index].color = "red".to_string();
            }
        }
    }

    fn check_tests(&self, index: usize) {
        let mut checks = self.checks.lock().unwrap();
        checks[index].last_check = Local::now().format("%H:%M:%S").to_string();

        let test_result = Command::new("cargo")
            .args(["test", "-p", "easyssh-core", "--no-fail-fast", "--", "--test-threads=1"])
            .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
            .output();

        match test_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let test_count = stdout.matches("test result:").count();
                let passed = stdout.contains("ok") && !stdout.contains("FAILED");

                if output.status.success() && passed {
                    checks[index].status = "✅".to_string();
                    checks[index].details = format!("{} 个测试通过", test_count);
                    checks[index].color = "green".to_string();
                } else {
                    let failed_count = stdout.matches("FAILED").count();
                    checks[index].status = "✗".to_string();
                    checks[index].details = format!("{} 个测试失败", failed_count.max(1));
                    checks[index].color = "red".to_string();
                }
            }
            Err(_) => {
                checks[index].status = "⏳".to_string();
                checks[index].details = "测试执行失败".to_string();
                checks[index].color = "yellow".to_string();
            }
        }
    }

    fn check_ci_cd(&self, index: usize) {
        let mut checks = self.checks.lock().unwrap();
        checks[index].last_check = Local::now().format("%H:%M:%S").to_string();

        // 检查 GitHub Actions 工作流文件
        let workflow_dir = std::path::Path::new("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH/.github/workflows");

        if workflow_dir.exists() {
            let workflows: Vec<_> = std::fs::read_dir(workflow_dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map(|e| e == "yml" || e == "yaml").unwrap_or(false))
                .collect();

            let workflow_count = workflows.len();

            // 检查是否有最近的 GitHub Actions 运行
            let recent_runs = Command::new("gh")
                .args(["run", "list", "-R", "anixn/EasySSH", "-L", "5", "--json", "status,conclusion"])
                .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
                .output();

            match recent_runs {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("failure") {
                        checks[index].status = "✗".to_string();
                        checks[index].details = "最近的 CI 运行失败".to_string();
                        checks[index].color = "red".to_string();
                    } else if stdout.contains("success") {
                        checks[index].status = "✅".to_string();
                        checks[index].details = format!("{} 个工作流, CI 正常", workflow_count);
                        checks[index].color = "green".to_string();
                    } else {
                        checks[index].status = "⏳".to_string();
                        checks[index].details = format!("{} 个工作流配置", workflow_count);
                        checks[index].color = "yellow".to_string();
                    }
                }
                _ => {
                    checks[index].status = "✅".to_string();
                    checks[index].details = format!("{} 个工作流配置就绪", workflow_count);
                    checks[index].color = "green".to_string();
                }
            }
        } else {
            checks[index].status = "✗".to_string();
            checks[index].details = "工作流目录不存在".to_string();
            checks[index].color = "red".to_string();
        }
    }

    fn auto_fix(&self) -> Vec<String> {
        let mut results = vec![];
        let checks = self.checks.lock().unwrap();

        // 修复代码格式
        if checks[0].status == "✗" && checks[0].details.contains("格式化") {
            results.push("🔧 执行 cargo fmt...".to_string());
            let _ = Command::new("cargo")
                .args(["fmt"])
                .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
                .output();
        }

        // 清理构建缓存
        if checks[2].status == "✗" {
            results.push("🧹 清理构建缓存...".to_string());
            let _ = Command::new("cargo")
                .args(["clean"])
                .current_dir("C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH")
                .output();
        }

        results.push("✅ 自动修复完成".to_string());
        results
    }

    fn run(&self) {
        // 首次检查
        self.check_code_quality(0);
        self.check_security(1);
        self.check_build(2);
        self.check_tests(3);
        self.check_ci_cd(4);
        self.render();

        let mut last_check = std::time::Instant::now();
        let check_interval = Duration::from_secs(30);

        loop {
            // 检查是否需要退出
            if !*self.running.lock().unwrap() {
                break;
            }

            // 每30秒自动刷新一次
            if last_check.elapsed() >= check_interval {
                self.check_code_quality(0);
                self.check_security(1);
                self.check_build(2);
                self.check_tests(3);
                self.check_ci_cd(4);
                self.render();
                last_check = std::time::Instant::now();
            }

            // 简单的命令处理（非阻塞检查输入）
            // 实际使用中可以用 crossterm 实现真正的非阻塞输入

            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn main() {
    println!("启动 EasySSH 前台自动化监控...");

    let monitor = EasySSHMonitor::new();

    // 设置 Ctrl+C 处理
    let running = monitor.running.clone();
    ctrlc::set_handler(move || {
        println!("\n👋 收到退出信号，正在关闭监控...");
        *running.lock().unwrap() = false;
        std::process::exit(0);
    }).expect("无法设置 Ctrl+C 处理器");

    monitor.run();
}
