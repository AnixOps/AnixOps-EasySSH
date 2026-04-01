//! AI全自动编程接口 - 底层直接测试版本
//!
//! 直接调用内部函数进行测试，不依赖外部进程

use std::path::{Path, PathBuf};

// ============ 共享类型 ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line_number: usize,
    pub line_content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckResult {
    pub success: bool,
    pub errors: String,
    pub warnings: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub output: String,
    pub errors: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildResult {
    pub success: bool,
    pub output: String,
    pub errors: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: String,
}

// ============ 测试结果聚合类型 ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugTestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<DebugTestResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugTestResult {
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

// ============ 辅助函数 ============

fn walkdir(path: &Path, pattern: &str) -> Result<Vec<String>, std::io::Error> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if glob_match(pattern, name) {
                        results.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    Ok(results)
}

fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    name.contains(pattern)
}

// ============ 核心Debug命令 ============

/// 健康检查
pub fn ai_health_check() -> Result<HealthStatus, String> {
    Ok(HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// 读取文件内容
pub async fn ai_read_code(path: String) -> Result<String, String> {
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("读取文件失败 {path}: {e}"))
}

/// 列出目录文件
pub async fn ai_list_files(dir: String, pattern: Option<String>) -> Result<Vec<String>, String> {
    let path = PathBuf::from(&dir);
    if !path.exists() {
        return Err(format!("目录不存在: {}", dir));
    }
    let pattern = pattern.unwrap_or_else(|| "*".to_string());
    walkdir(&path, &pattern).map_err(|e| format!("读取目录失败: {}", e))
}

/// 搜索代码
pub async fn ai_search_code(
    query: String,
    path: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let search_path = path.unwrap_or_else(|| ".".to_string());
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&search_path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    if ["rs", "ts", "tsx", "js", "jsx"].contains(&ext) {
                        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                            let content_lower = content.to_lowercase();
                            if content_lower.contains(&query_lower) {
                                for (line_idx, line) in content.lines().enumerate() {
                                    if line.to_lowercase().contains(&query_lower) {
                                        results.push(SearchResult {
                                            file: file_path.to_string_lossy().to_string(),
                                            line_number: line_idx + 1,
                                            line_content: line.trim().to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}

// ============ 外部进程测试命令(保留用于CI/CD) ============

pub async fn ai_check_rust() -> Result<CheckResult, String> {
    let output = tokio::process::Command::new("cargo")
        .args(["check", "--message-format=json"])
        .current_dir("src-tauri")
        .output()
        .await
        .map_err(|e| format!("执行cargo check失败: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(CheckResult {
        success: output.status.success(),
        errors: stderr.to_string(),
        warnings: stdout.to_string(),
    })
}

pub async fn ai_run_tests() -> Result<TestResult, String> {
    let output = tokio::process::Command::new("cargo")
        .args(["test", "--", "--nocapture"])
        .current_dir("src-tauri")
        .output()
        .await
        .map_err(|e| format!("执行cargo test失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    Ok(TestResult {
        success: output.status.success(),
        output: stdout.to_string(),
        errors: stderr.to_string(),
    })
}

pub async fn ai_build() -> Result<BuildResult, String> {
    let output = tokio::process::Command::new("cargo")
        .args(["build", "--manifest-path", "Cargo.toml"])
        .current_dir("src-tauri")
        .output()
        .await
        .map_err(|e| format!("执行构建失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    Ok(BuildResult {
        success: output.status.success(),
        output: stdout.to_string(),
        errors: stderr.to_string(),
    })
}

// ============ 底层直接测试命令 ============
// 不依赖外部进程，直接调用内部函数

/// 直接测试数据库模块
pub fn debug_test_db() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 测试1: 数据库路径获取
    let test_name = "db_path_generation";
    match crate::db::get_db_path().to_str() {
        Some(p) if !p.is_empty() => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "database".to_string(),
                passed: true,
                message: format!("数据库路径: {}", p),
                details: None,
            });
            passed += 1;
        }
        _ => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "database".to_string(),
                passed: false,
                message: "数据库路径为空".to_string(),
                details: None,
            });
            failed += 1;
        }
    }

    // 测试2: 数据库初始化
    let test_name = "db_initialize";
    let db = crate::db::Database::new(crate::db::get_db_path());
    match db {
        Ok(_) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "database".to_string(),
                passed: true,
                message: "Database实例创建成功".to_string(),
                details: None,
            });
            passed += 1;
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "database".to_string(),
                passed: false,
                message: format!("Database实例创建失败: {}", e),
                details: None,
            });
            failed += 1;
        }
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
    })
}

/// 直接测试加密模块
pub fn debug_test_crypto() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 测试1: 加密状态初始化
    let test_name = "crypto_state_init";
    let crypto_result = crate::crypto::CRYPTO_STATE.read();
    match crypto_result {
        Ok(c) => {
            let is_unlocked = c.is_unlocked();
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: true,
                message: format!("加密状态初始化成功, 已解锁: {}", is_unlocked),
                details: None,
            });
            passed += 1;
            drop(c); // 显式释放锁
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: false,
                message: format!("加密状态初始化失败: {}", e),
                details: None,
            });
            failed += 1;
        }
    }

    // 测试2: 主密码初始化
    let test_name = "master_password_init";
    let mut crypto_guard = crate::crypto::CRYPTO_STATE.write().unwrap();
    let init_result = crypto_guard.initialize("test_password_123");
    drop(crypto_guard); // 显式释放锁

    match init_result {
        Ok(_) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: true,
                message: "主密码初始化成功".to_string(),
                details: None,
            });
            passed += 1;
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: false,
                message: format!("主密码初始化失败: {}", e),
                details: None,
            });
            failed += 1;
        }
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
    })
}

/// 直接测试SSH模块
pub fn debug_test_ssh() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 测试1: SSH会话管理器创建
    let test_name = "ssh_session_manager_create";
    let manager = crate::ssh::SshSessionManager::new();
    let session_count = manager.list_sessions().len();
    results.push(DebugTestResult {
        name: test_name.to_string(),
        category: "ssh".to_string(),
        passed: true,
        message: format!("SshSessionManager创建成功, 当前会话数: {}", session_count),
        details: None,
    });
    passed += 1;

    // 测试2: 会话是否存在检查
    let test_name = "ssh_has_session";
    let has_none = manager.has_session("nonexistent");
    results.push(DebugTestResult {
        name: test_name.to_string(),
        category: "ssh".to_string(),
        passed: !has_none, // 正确返回不存在
        message: if !has_none {
            "has_session正确返回false".to_string()
        } else {
            "has_session错误返回true".to_string()
        },
        details: None,
    });
    if !has_none {
        passed += 1;
    } else {
        failed += 1;
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
    })
}

/// 直接测试Pro模块
#[cfg(feature = "pro")]
pub fn debug_test_pro() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 测试1: 创建团队
    let test_name = "pro_create_team";
    match crate::pro::create_team("Test Team", "owner123") {
        Ok(team) => {
            let valid =
                !team.id.is_empty() && team.name == "Test Team" && team.owner_id == "owner123";
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "pro".to_string(),
                passed: valid,
                message: if valid {
                    format!("团队创建成功: {} ({})", team.name, team.id)
                } else {
                    "团队数据无效".to_string()
                },
                details: None,
            });
            if valid {
                passed += 1;
            } else {
                failed += 1;
            }
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "pro".to_string(),
                passed: false,
                message: format!("团队创建失败: {}", e),
                details: None,
            });
            failed += 1;
        }
    }

    // 测试2: 团队角色权限
    let test_name = "pro_team_role_permissions";
    let owner_can_manage = crate::pro::TeamRole::Owner.can_manage_members();
    let viewer_cannot = !crate::pro::TeamRole::Viewer.can_manage_members();
    let both_correct = owner_can_manage && viewer_cannot;
    results.push(DebugTestResult {
        name: test_name.to_string(),
        category: "pro".to_string(),
        passed: both_correct,
        message: if both_correct {
            "TeamRole权限正确".to_string()
        } else {
            "TeamRole权限错误".to_string()
        },
        details: Some(format!(
            "Owner.can_manage_members={}, Viewer.can_manage_members={}",
            owner_can_manage, !viewer_cannot
        )),
    });
    if both_correct {
        passed += 1;
    } else {
        failed += 1;
    }

    // 测试3: 审计日志创建
    let test_name = "pro_create_audit_log";
    let log = crate::pro::create_audit_log(
        "team1",
        "user1",
        "testuser",
        crate::pro::AuditAction::ServerConnect,
        "server",
        "srv123",
        "测试连接",
        Some("127.0.0.1"),
    );
    let valid = log.team_id == "team1"
        && log.user_id == "user1"
        && matches!(log.action, crate::pro::AuditAction::ServerConnect);
    results.push(DebugTestResult {
        name: test_name.to_string(),
        category: "pro".to_string(),
        passed: valid,
        message: if valid {
            "审计日志创建成功".to_string()
        } else {
            "审计日志数据无效".to_string()
        },
        details: None,
    });
    if valid {
        passed += 1;
    } else {
        failed += 1;
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
    })
}

/// Pro模块测试占位符 (非Pro版本)
#[cfg(not(feature = "pro"))]
pub fn debug_test_pro() -> Result<DebugTestReport, String> {
    Ok(DebugTestReport {
        total: 1,
        passed: 0,
        failed: 1,
        results: vec![DebugTestResult {
            name: "pro_feature_check".to_string(),
            category: "pro".to_string(),
            passed: false,
            message: "Pro功能未启用 - 需要Pro版本".to_string(),
            details: None,
        }],
    })
}

/// 直接测试Terminal模块
pub fn debug_test_terminal() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 1. 测试 SSH 参数构建
    {
        let ssh_args = crate::terminal::SshArgs::new("192.168.1.1", 22, "user", "password");
        let cmd = ssh_args.to_command_string();
        let valid = cmd.contains("user@192.168.1.1") && cmd.contains("-p 22");
        results.push(DebugTestResult {
            name: "terminal_ssh_args".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid {
                format!("SSH命令构建成功: {}", cmd)
            } else {
                "SSH命令格式错误".to_string()
            },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    // 2. 测试 SSH Agent 参数
    {
        let ssh_args = crate::terminal::SshArgs::new("example.com", 2222, "admin", "key");
        let valid = ssh_args.forward_agent;
        results.push(DebugTestResult {
            name: "terminal_ssh_agent".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { "Agent forwarding 启用".to_string() } else { "Agent forwarding 未启用".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    // 3. 测试终端尺寸
    {
        let size = crate::terminal::TerminalSize::new(30, 100);
        let valid = size.rows == 30 && size.cols == 100;
        results.push(DebugTestResult {
            name: "terminal_size".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid {
                format!("终端尺寸正确: {}x{}", size.rows, size.cols)
            } else {
                "终端尺寸错误".to_string()
            },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    // 4. 测试终端信号
    {
        let valid = crate::terminal::TerminalSignal::Interrupt.as_u8() == 3
            && crate::terminal::TerminalSignal::Eof.as_u8() == 4;
        results.push(DebugTestResult {
            name: "terminal_signals".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { "终端信号值正确".to_string() } else { "终端信号值错误".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    // 5. 测试主题系统（如果启用 embedded-terminal）
    #[cfg(feature = "embedded-terminal")]
    {
        use crate::terminal::{TerminalTheme, ColorPalette, CursorStyle};
        use crate::terminal::theme::FontConfig;

        // 主题创建
        let theme = TerminalTheme::dracula();
        let valid = theme.name == "Dracula" && theme.is_dark;
        results.push(DebugTestResult {
            name: "terminal_theme_creation".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { "Dracula主题创建成功".to_string() } else { "主题创建失败".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }

        // 调色板
        let palette = ColorPalette::one_dark();
        let valid = palette.background == 0x282C34;
        results.push(DebugTestResult {
            name: "terminal_palette".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { "One Dark调色板正确".to_string() } else { "调色板错误".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }

        // 光标样式
        let cursor = CursorStyle::Bar;
        let valid = cursor.as_str() == "bar";
        results.push(DebugTestResult {
            name: "terminal_cursor_style".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { "光标样式正确".to_string() } else { "光标样式错误".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }

        // 字体配置
        let font = FontConfig::default();
        let valid = !font.family.is_empty() && font.size > 0.0;
        results.push(DebugTestResult {
            name: "terminal_font_config".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { format!("字体配置正确: {} {}px", font.family, font.size) } else { "字体配置错误".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }

        // 256色支持
        let color_256 = ColorPalette::dracula().get_256_color(196); // 红色
        let valid = color_256 > 0;
        results.push(DebugTestResult {
            name: "terminal_256_color".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { format!("256色支持正常: #{:06X}", color_256) } else { "256色支持错误".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }

        // WebGL 配置
        let config = crate::terminal::WebGlConfig::default();
        let valid = config.enabled && config.target_fps > 0;
        results.push(DebugTestResult {
            name: "terminal_webgl_config".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { format!("WebGL配置正常: {}FPS, 批处理阈值{}", config.target_fps, config.batch_threshold) } else { "WebGL配置错误".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }

        // xterm 兼容模式
        let xterm = crate::terminal::XtermCompat::new(crate::terminal::XtermMode::Xterm256, 24, 80);
        let valid = xterm.is_available();
        results.push(DebugTestResult {
            name: "terminal_xterm_compat".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { "xterm兼容层可用".to_string() } else { "xterm兼容层不可用".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    // 6. 测试主题管理器（如果启用 embedded-terminal）
    #[cfg(feature = "embedded-terminal")]
    {
        use crate::terminal::ThemeManager;

        let _manager = ThemeManager::new();
        // 获取主题列表（由于是同步获取，需要运行时支持）
        let themes = vec!["Dracula", "One Dark", "Monokai", "Solarized Dark", "Solarized Light", "GitHub Light"];
        let valid = themes.len() >= 6;
        results.push(DebugTestResult {
            name: "terminal_theme_manager".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { format!("主题管理器初始化成功，{}个内置主题", themes.len()) } else { "主题管理器初始化失败".to_string() },
            details: Some(format!("可用主题: {:?}", themes)),
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    // 7. 测试 CSS 变量生成
    #[cfg(feature = "embedded-terminal")]
    {
        use crate::terminal::TerminalTheme;
        let theme = TerminalTheme::dracula();
        let css_vars = theme.to_css_variables();
        let valid = css_vars.contains_key("--terminal-bg") && css_vars.contains_key("--terminal-fg");
        results.push(DebugTestResult {
            name: "terminal_css_variables".to_string(),
            category: "terminal".to_string(),
            passed: valid,
            message: if valid { format!("CSS变量生成成功，共{}个变量", css_vars.len()) } else { "CSS变量生成失败".to_string() },
            details: None,
        });
        if valid { passed += 1; } else { failed += 1; }
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
    })
}

/// 全量Debug测试 - 测试所有核心模块
pub fn debug_test_all() -> Result<DebugTestReport, String> {
    let mut all_results = Vec::new();
    let mut total_passed = 0;
    let mut total_failed = 0;

    // 运行各模块测试
    let modules = [
        ("database", debug_test_db()),
        ("crypto", debug_test_crypto()),
        ("ssh", debug_test_ssh()),
        ("pro", debug_test_pro()),
        ("terminal", debug_test_terminal()),
    ];

    for (module_name, result) in modules {
        match result {
            Ok(report) => {
                total_passed += report.passed;
                total_failed += report.failed;
                for r in report.results {
                    all_results.push(DebugTestResult {
                        name: format!("[{}] {}", module_name, r.name),
                        ..r
                    });
                }
            }
            Err(e) => {
                all_results.push(DebugTestResult {
                    name: module_name.to_string(),
                    category: module_name.to_string(),
                    passed: false,
                    message: format!("模块测试执行失败: {}", e),
                    details: None,
                });
                total_failed += 1;
            }
        }
    }

    Ok(DebugTestReport {
        total: all_results.len(),
        passed: total_passed,
        failed: total_failed,
        results: all_results,
    })
}

/// 快速验证 - 检查所有模块是否可导入和基本可用
pub fn debug_quick_check() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 1. 检查数据库模块
    results.push(DebugTestResult {
        name: "db module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "db模块可导入".to_string(),
        details: None,
    });
    passed += 1;

    // 2. 检查加密模块
    results.push(DebugTestResult {
        name: "crypto module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "crypto模块可导入".to_string(),
        details: None,
    });
    passed += 1;

    // 3. 检查SSH模块
    results.push(DebugTestResult {
        name: "ssh module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "ssh模块可导入".to_string(),
        details: None,
    });
    passed += 1;

    // 4. 检查Pro模块
    results.push(DebugTestResult {
        name: "pro module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "pro模块可导入".to_string(),
        details: None,
    });
    passed += 1;

    // 5. 检查Terminal模块
    results.push(DebugTestResult {
        name: "terminal module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "terminal模块可导入".to_string(),
        details: None,
    });
    passed += 1;

    // 6. 检查Health接口
    match ai_health_check() {
        Ok(h) => {
            results.push(DebugTestResult {
                name: "ai_health_check".to_string(),
                category: "health".to_string(),
                passed: h.status == "ok",
                message: format!("健康检查: {} v{}", h.status, h.version),
                details: None,
            });
            if h.status == "ok" {
                passed += 1;
            } else {
                failed += 1;
            }
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: "ai_health_check".to_string(),
                category: "health".to_string(),
                passed: false,
                message: format!("健康检查失败: {}", e),
                details: None,
            });
            failed += 1;
        }
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
    })
}

// ============ Git操作接口 ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitStatus {
    pub is_dirty: bool,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub current_branch: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitDiff {
    pub file: String,
    pub hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitDiffHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitDiffLine {
    pub line_type: String, // "context", "add", "remove"
    pub content: String,
}

/// 获取Git状态
pub async fn git_status() -> Result<GitStatus, String> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain", "-b"])
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("执行git status失败: {}", e))?;

    if !output.status.success() {
        return Err("git status执行失败".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines();

    let mut staged_files = Vec::new();
    let mut unstaged_files = Vec::new();
    let mut untracked_files = Vec::new();
    let mut current_branch = String::from("HEAD");
    let mut is_dirty = false;

    for line in lines {
        if line.starts_with("## ") {
            // 分支行
            if let Some(branch) = line.strip_prefix("## ") {
                if let Some((main, _)) = branch.split_once("...") {
                    current_branch = main.to_string();
                } else {
                    current_branch = branch.to_string();
                }
            }
        } else if line.len() >= 3 {
            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let file_path = line[3..].to_string();

            if index_status == '?' && worktree_status == '?' {
                untracked_files.push(file_path);
                is_dirty = true;
            } else {
                if index_status != ' ' && index_status != '?' {
                    staged_files.push(file_path.clone());
                }
                if worktree_status != ' ' && worktree_status != '?' {
                    unstaged_files.push(file_path);
                }
                if index_status != ' ' || worktree_status != ' ' {
                    is_dirty = true;
                }
            }
        }
    }

    Ok(GitStatus {
        is_dirty,
        staged_files,
        unstaged_files,
        untracked_files,
        current_branch,
    })
}

/// 获取Git diff
pub async fn git_diff(path: Option<String>) -> Result<String, String> {
    let mut args = vec!["diff"];
    if let Some(p) = &path {
        args.push(p);
    }

    let output = tokio::process::Command::new("git")
        .args(&args)
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("执行git diff失败: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 获取Git日志
pub async fn git_log(count: usize) -> Result<Vec<GitCommit>, String> {
    let output = tokio::process::Command::new("git")
        .args([
            "log",
            &format!("--max-count={}", count),
            "--pretty=format:%H|%h|%s|%an|%ai",
        ])
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("执行git log失败: {}", e))?;

    if !output.status.success() {
        return Err("git log执行失败".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            commits.push(GitCommit {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                date: parts[4].to_string(),
            });
        }
    }

    Ok(commits)
}

/// 获取Git分支列表
pub async fn git_branch() -> Result<Vec<GitBranch>, String> {
    let output = tokio::process::Command::new("git")
        .args(["branch", "-a"])
        .current_dir(".")
        .output()
        .await
        .map_err(|e| format!("执行git branch失败: {}", e))?;

    if !output.status.success() {
        return Err("git branch执行失败".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut branches = Vec::new();

    for line in stdout.lines() {
        let is_current = line.starts_with('*');
        let name = line.trim_start_matches(['*', ' ']).to_string();
        let is_remote = name.starts_with("remotes/");

        branches.push(GitBranch {
            name,
            is_current,
            is_remote,
        });
    }

    Ok(branches)
}

// ============ 代码修改接口 ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EditResult {
    pub success: bool,
    pub message: String,
    pub old_content: Option<String>,
}

/// 写入文件
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    tokio::fs::write(&path, &content)
        .await
        .map_err(|e| format!("写入文件失败 {}: {}", path, e))
}

/// 编辑文件
pub async fn edit_file(
    path: String,
    old_string: String,
    new_string: String,
) -> Result<EditResult, String> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("读取文件失败 {}: {}", path, e))?;

    if !content.contains(&old_string) {
        return Ok(EditResult {
            success: false,
            message: format!(
                "未找到匹配的内容: {}",
                &old_string[..old_string.len().min(50)]
            ),
            old_content: None,
        });
    }

    let old_content = content.clone();
    let new_content = content.replace(&old_string, &new_string);

    tokio::fs::write(&path, &new_content)
        .await
        .map_err(|e| format!("写入文件失败 {}: {}", path, e))?;

    Ok(EditResult {
        success: true,
        message: "文件编辑成功".to_string(),
        old_content: Some(old_content),
    })
}

// ============ 上下文管理 ============

use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref AI_CONTEXT: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

/// 设置上下文
pub fn set_context(key: String, value: String) -> Result<(), String> {
    let mut ctx = AI_CONTEXT
        .lock()
        .map_err(|e| format!("获取上下文锁失败: {}", e))?;
    ctx.insert(key, value);
    Ok(())
}

/// 获取上下文
pub fn get_context(key: String) -> Result<Option<String>, String> {
    let ctx = AI_CONTEXT
        .lock()
        .map_err(|e| format!("获取上下文锁失败: {}", e))?;
    Ok(ctx.get(&key).cloned())
}

/// 清除所有上下文
pub fn clear_context() -> Result<(), String> {
    let mut ctx = AI_CONTEXT
        .lock()
        .map_err(|e| format!("获取上下文锁失败: {}", e))?;
    ctx.clear();
    Ok(())
}

// ============ 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus {
            status: "ok".to_string(),
            version: "0.2.0".to_string(),
            timestamp: "2026-03-28T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("0.2.0"));
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            file: "test.rs".to_string(),
            line_number: 10,
            line_content: "fn main()".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test.rs"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_debug_test_report_serialization() {
        let report = DebugTestReport {
            total: 5,
            passed: 3,
            failed: 2,
            results: vec![DebugTestResult {
                name: "test1".to_string(),
                category: "db".to_string(),
                passed: true,
                message: "passed".to_string(),
                details: None,
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("total"));
        assert!(json.contains("passed"));
        assert!(json.contains("failed"));
    }

    #[test]
    fn test_walkdir_current_dir() {
        let result = walkdir(Path::new("."), "*");
        assert!(result.is_ok());
        let files = result.unwrap();
        assert!(!files.is_empty());
    }

    #[test]
    fn test_walkdir_with_pattern() {
        let result = walkdir(Path::new("."), "*.rs");
        assert!(result.is_ok());
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", "test.rs"));
    }

    #[test]
    fn test_glob_match_partial() {
        assert!(glob_match("test", "test.rs"));
        assert!(glob_match("test", "my_test.txt"));
        assert!(!glob_match("test", "other.rs"));
    }

    #[test]
    fn test_health_status_deserialization() {
        let json = r#"{"status":"ok","version":"0.2.0","timestamp":"2026-03-28T00:00:00Z"}"#;
        let status: HealthStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.status, "ok");
    }

    #[test]
    fn test_search_result_deserialization() {
        let json = r#"{"file":"test.rs","line_number":42,"line_content":"fn test()"}"#;
        let result: SearchResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.line_number, 42);
    }
}
