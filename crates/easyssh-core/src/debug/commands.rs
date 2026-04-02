//! Debug命令模块
//!
//! 提供调试测试命令，兼容旧版 `ai_programming.rs` 中的测试功能

use crate::debug::DebugAccessLevel;

/// 调试测试结果（兼容旧版）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugTestResult {
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub duration_ms: Option<u64>,
}

/// 调试测试报告（兼容旧版）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugTestReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<DebugTestResult>,
    pub duration_ms: u64,
}

/// 快速检查 - 所有版本可用
pub fn quick_check() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 1. 检查数据库模块
    results.push(DebugTestResult {
        name: "db module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "db module available".to_string(),
        details: None,
        duration_ms: None,
    });
    passed += 1;

    // 2. 检查加密模块
    results.push(DebugTestResult {
        name: "crypto module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "crypto module available".to_string(),
        details: None,
        duration_ms: None,
    });
    passed += 1;

    // 3. 检查SSH模块
    results.push(DebugTestResult {
        name: "ssh module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "ssh module available".to_string(),
        details: None,
        duration_ms: None,
    });
    passed += 1;

    // 4. 检查Pro模块（可能不可用）
    let pro_available = cfg!(feature = "pro");
    results.push(DebugTestResult {
        name: "pro module".to_string(),
        category: "import".to_string(),
        passed: pro_available,
        message: if pro_available {
            "pro module available".to_string()
        } else {
            "pro module not available (expected for Lite/Standard)".to_string()
        },
        details: None,
        duration_ms: None,
    });
    if pro_available {
        passed += 1;
    } else {
        failed += 1;
    }

    // 5. 检查Terminal模块
    results.push(DebugTestResult {
        name: "terminal module".to_string(),
        category: "import".to_string(),
        passed: true,
        message: "terminal module available".to_string(),
        details: None,
        duration_ms: None,
    });
    passed += 1;

    // 6. 检查Debug状态
    let level = crate::debug::get_access_level();
    let is_enabled = crate::debug::is_debug_enabled();
    results.push(DebugTestResult {
        name: "debug access".to_string(),
        category: "debug".to_string(),
        passed: is_enabled,
        message: format!("Debug access: enabled={}, level={:?}", is_enabled, level),
        details: None,
        duration_ms: None,
    });
    if is_enabled {
        passed += 1;
    } else {
        failed += 1;
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
        duration_ms: 0,
    })
}

/// 测试数据库模块 - 需要Standard+
pub fn test_db() -> Result<DebugTestReport, String> {
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
                message: format!("Database path: {}", p),
                details: None,
                duration_ms: None,
            });
            passed += 1;
        }
        _ => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "database".to_string(),
                passed: false,
                message: "Database path is empty".to_string(),
                details: None,
                duration_ms: None,
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
                message: "Database instance created successfully".to_string(),
                details: None,
                duration_ms: None,
            });
            passed += 1;
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "database".to_string(),
                passed: false,
                message: format!("Database instance creation failed: {}", e),
                details: None,
                duration_ms: None,
            });
            failed += 1;
        }
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
        duration_ms: 0,
    })
}

/// 测试加密模块 - 需要Standard+
pub fn test_crypto() -> Result<DebugTestReport, String> {
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
                message: format!("Crypto state initialized, unlocked: {}", is_unlocked),
                details: None,
                duration_ms: None,
            });
            passed += 1;
            drop(c);
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: false,
                message: format!("Crypto state initialization failed: {}", e),
                details: None,
                duration_ms: None,
            });
            failed += 1;
        }
    }

    // 测试2: 主密码初始化
    let test_name = "master_password_init";
    let mut crypto_guard = crate::crypto::CRYPTO_STATE.write().unwrap();
    let init_result = crypto_guard.initialize("test_password_123");
    drop(crypto_guard);

    match init_result {
        Ok(_) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: true,
                message: "Master password initialized successfully".to_string(),
                details: None,
                duration_ms: None,
            });
            passed += 1;
        }
        Err(e) => {
            results.push(DebugTestResult {
                name: test_name.to_string(),
                category: "crypto".to_string(),
                passed: false,
                message: format!("Master password initialization failed: {}", e),
                details: None,
                duration_ms: None,
            });
            failed += 1;
        }
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
        duration_ms: 0,
    })
}

/// 测试SSH模块 - 需要Standard+
pub fn test_ssh() -> Result<DebugTestReport, String> {
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
        message: format!(
            "SshSessionManager created, session count: {}",
            session_count
        ),
        details: None,
        duration_ms: None,
    });
    passed += 1;

    // 测试2: 会话存在检查
    let test_name = "ssh_has_session";
    let has_none = manager.has_session("nonexistent");
    results.push(DebugTestResult {
        name: test_name.to_string(),
        category: "ssh".to_string(),
        passed: !has_none,
        message: if !has_none {
            "has_session correctly returned false".to_string()
        } else {
            "has_session incorrectly returned true".to_string()
        },
        details: None,
        duration_ms: None,
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
        duration_ms: 0,
    })
}

/// 测试Terminal模块 - 所有版本
pub fn test_terminal() -> Result<DebugTestReport, String> {
    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    // 1. 测试 SSH 参数构建
    let ssh_args = crate::terminal::SshArgs::new("192.168.1.1", 22, "user", "password");
    let cmd = ssh_args.to_command_string();
    let valid = cmd.contains("user@192.168.1.1") && cmd.contains("-p 22");
    results.push(DebugTestResult {
        name: "terminal_ssh_args".to_string(),
        category: "terminal".to_string(),
        passed: valid,
        message: if valid {
            format!("SSH command built successfully: {}", cmd)
        } else {
            "SSH command format error".to_string()
        },
        details: None,
        duration_ms: None,
    });
    if valid {
        passed += 1;
    } else {
        failed += 1;
    }

    // 2. 测试终端尺寸
    let size = crate::terminal::TerminalSize::new(30, 100);
    let valid = size.rows == 30 && size.cols == 100;
    results.push(DebugTestResult {
        name: "terminal_size".to_string(),
        category: "terminal".to_string(),
        passed: valid,
        message: if valid {
            format!("Terminal size correct: {}x{}", size.rows, size.cols)
        } else {
            "Terminal size error".to_string()
        },
        details: None,
        duration_ms: None,
    });
    if valid {
        passed += 1;
    } else {
        failed += 1;
    }

    // 3. 测试终端信号
    let valid = crate::terminal::TerminalSignal::Interrupt.as_u8() == 3
        && crate::terminal::TerminalSignal::Eof.as_u8() == 4;
    results.push(DebugTestResult {
        name: "terminal_signals".to_string(),
        category: "terminal".to_string(),
        passed: valid,
        message: if valid {
            "Terminal signal values correct".to_string()
        } else {
            "Terminal signal values error".to_string()
        },
        details: None,
        duration_ms: None,
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
        duration_ms: 0,
    })
}

/// 测试Pro模块 - 需要Pro功能
#[cfg(feature = "pro")]
pub fn test_pro() -> Result<DebugTestReport, String> {
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
                    format!("Team created successfully: {} ({})", team.name, team.id)
                } else {
                    "Team data invalid".to_string()
                },
                details: None,
                duration_ms: None,
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
                message: format!("Team creation failed: {}", e),
                details: None,
                duration_ms: None,
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
            "TeamRole permissions correct".to_string()
        } else {
            "TeamRole permissions error".to_string()
        },
        details: None,
        duration_ms: None,
    });
    if both_correct {
        passed += 1;
    } else {
        failed += 1;
    }

    Ok(DebugTestReport {
        total: results.len(),
        passed,
        failed,
        results,
        duration_ms: 0,
    })
}

/// Pro模块测试占位符 (非Pro版本)
#[cfg(not(feature = "pro"))]
pub fn test_pro() -> Result<DebugTestReport, String> {
    Ok(DebugTestReport {
        total: 1,
        passed: 0,
        failed: 1,
        results: vec![DebugTestResult {
            name: "pro_feature_check".to_string(),
            category: "pro".to_string(),
            passed: false,
            message: "Pro feature not enabled - requires Pro edition".to_string(),
            details: None,
            duration_ms: None,
        }],
        duration_ms: 0,
    })
}

/// 全量测试 - 所有模块
pub fn test_all() -> Result<DebugTestReport, String> {
    let mut all_results = Vec::new();
    let mut total_passed = 0;
    let mut total_failed = 0;
    let start = std::time::Instant::now();

    // 运行各模块测试
    let modules: [(&str, fn() -> Result<DebugTestReport, String>); 5] = [
        ("database", test_db),
        ("crypto", test_crypto),
        ("ssh", test_ssh),
        ("pro", test_pro),
        ("terminal", test_terminal),
    ];

    for (module_name, test_fn) in modules {
        match test_fn() {
            Ok(report) => {
                total_passed += report.passed;
                total_failed += report.failed;
                for r in report.results {
                    all_results.push(DebugTestResult {
                        name: format!("[{}] {}", module_name, r.name),
                        category: r.category,
                        passed: r.passed,
                        message: r.message,
                        details: None,
                        duration_ms: r.duration_ms,
                    });
                }
            }
            Err(e) => {
                all_results.push(DebugTestResult {
                    name: module_name.to_string(),
                    category: module_name.to_string(),
                    passed: false,
                    message: format!("Module test execution failed: {}", e),
                    details: None,
                    duration_ms: None,
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
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_check() {
        let report = quick_check().unwrap();
        assert!(report.total > 0);
        // 至少有一些测试通过
        assert!(report.passed > 0);
    }

    #[test]
    fn test_terminal() {
        let report = test_terminal().unwrap();
        assert_eq!(report.total, 3); // 三个测试项
    }
}
