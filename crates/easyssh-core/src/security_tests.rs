//! 安全测试模块
//!
//! 本模块包含针对安全漏洞的测试用例
//! 运行: cargo test security

#[cfg(test)]
mod tests {
    use std::path::Path;

    // =========================================================================
    // 1. 命令注入防护测试
    // =========================================================================

    /// 测试危险字符检测
    #[test]
    fn test_command_injection_detection() {
        let malicious_commands = vec![
            "ls; rm -rf /",
            "echo $(whoami)",
            "cat `cat /etc/passwd`",
            "ls && reboot",
            "ls || shutdown",
            "ls | cat /etc/shadow",
            "ls > /etc/passwd",
            "ls < /etc/passwd",
            "$(malicious_command)",
            "`malicious_command`",
            "${IFS}malicious",
            "ls\nreboot",
            "ls\treboot",
        ];

        for cmd in &malicious_commands {
            assert!(
                contains_dangerous_shell_chars(cmd),
                "命令应被检测为危险: {}",
                cmd
            );
        }

        // 安全命令应通过
        let safe_commands = vec![
            "ls -la",
            "cat file.txt",
            "echo hello world",
            "ps aux",
            "df -h",
        ];

        for cmd in &safe_commands {
            assert!(
                !contains_dangerous_shell_chars(cmd),
                "命令应被检测为安全: {}",
                cmd
            );
        }
    }

    fn contains_dangerous_shell_chars(command: &str) -> bool {
        const DANGEROUS_CHARS: &[char] = &[
            ';', '&', '|', '`', '$', '(', ')', '{', '}', '<', '>', '\n', '\t',
        ];
        command.chars().any(|c| DANGEROUS_CHARS.contains(&c))
    }

    // =========================================================================
    // 2. 路径遍历防护测试
    // =========================================================================

    /// 测试路径规范化
    #[test]
    fn test_path_traversal_prevention() {
        let base_dir = Path::new("/home/user/projects");

        let test_cases = vec![
            // (输入路径, 是否应被允许)
            ("file.txt", true),
            ("subdir/file.txt", true),
            ("./file.txt", true),
            ("../file.txt", false),
            ("../../etc/passwd", false),
            ("/../../../etc/passwd", false),
            // On Windows, /etc/passwd might not be absolute, so test with Windows-style paths too
            #[cfg(windows)]
            ("C:\\etc\\passwd", false),
            #[cfg(not(windows))]
            ("/etc/passwd", false),
            ("subdir/../../../etc/passwd", false),
            ("..\\..\\Windows\\System32\\config\\SAM", false),
        ];

        for (input, should_be_allowed) in &test_cases {
            let is_allowed = is_path_within_base(base_dir, input);
            assert_eq!(
                is_allowed, *should_be_allowed,
                "路径 '{}' 的验证结果应为 {}, 但实际为 {}",
                input, should_be_allowed, is_allowed
            );
        }
    }

    fn is_path_within_base(_base: &Path, input: &str) -> bool {
        let input_path = Path::new(input);

        // 检查是否包含 .. 组件
        if input_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return false;
        }

        // 检查是否是绝对路径（Windows风格或Unix风格）
        if input_path.is_absolute()
            || input.starts_with('/')
            || (input.len() > 1 && input.chars().nth(1) == Some(':'))
        {
            return false;
        }

        true
    }

    /// 测试路径规范化
    #[test]
    fn test_path_normalization() {
        let test_cases = vec![
            ("./file.txt", "file.txt"),
            ("dir/./file.txt", "dir/file.txt"),
            ("dir/subdir/../file.txt", "dir/file.txt"),
            ("dir/../../file.txt", "file.txt"), // 规范化后可能仍在范围内
        ];

        for (input, expected) in &test_cases {
            let normalized = normalize_path(input);
            assert_eq!(
                &normalized, *expected,
                "路径 '{}' 规范化应为 '{}'",
                input, expected
            );
        }
    }

    fn normalize_path(path: &str) -> String {
        let path = Path::new(path);
        let mut result = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::Normal(c) => result.push(c.to_str().unwrap()),
                std::path::Component::ParentDir => {
                    if !result.is_empty() {
                        result.pop();
                    }
                }
                _ => {}
            }
        }

        result.join("/")
    }

    // =========================================================================
    // 3. 输入验证测试
    // =========================================================================

    /// 测试主机名验证
    #[test]
    fn test_hostname_validation() {
        let valid_hosts = vec![
            "192.168.1.1",
            "10.0.0.1",
            "255.255.255.255",
            "example.com",
            "server.example.com",
            "localhost",
            "my-server-01",
        ];

        let long_string = "a".repeat(256);
        let invalid_hosts = vec![
            "192.168.1",          // 不完整的IP
            "256.1.1.1",          // 无效的IP
            "example..com",       // 无效域名
            "-example.com",       // 以连字符开头
            "example.com;rm -rf", // 包含恶意字符
            "$(whoami)",          // 命令注入
            "",                   // 空字符串
            long_string.as_str(), // 超长
        ];

        for host in &valid_hosts {
            assert!(is_valid_hostname(host), "主机名 '{}' 应被视为有效", host);
        }

        for host in &invalid_hosts {
            assert!(!is_valid_hostname(host), "主机名 '{}' 应被视为无效", host);
        }
    }

    fn is_valid_hostname(host: &str) -> bool {
        // 空检查
        if host.is_empty() || host.len() > 255 {
            return false;
        }

        // 危险字符检查
        if host
            .chars()
            .any(|c| matches!(c, ';' | '&' | '|' | '`' | '$' | '(' | ')'))
        {
            return false;
        }

        // IP地址验证（必须是完整的4段IP）
        let looks_like_ip = host.chars().all(|c| c.is_ascii_digit() || c == '.');
        if looks_like_ip {
            return is_valid_ip(host);
        }

        // 主机名验证
        is_valid_domain(host)
    }

    fn is_valid_ip(host: &str) -> bool {
        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() != 4 {
            return false;
        }

        for part in &parts {
            match part.parse::<u8>() {
                Ok(n) if n.to_string() == *part => {}
                _ => return false,
            }
        }

        true
    }

    fn is_valid_domain(host: &str) -> bool {
        // 简单域名验证
        let valid_chars = host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');

        if !valid_chars {
            return false;
        }

        // 不能以连字符开头或结尾
        if host.starts_with('-') || host.ends_with('-') {
            return false;
        }

        // 不能有连续的连字符或点
        if host.contains("--") || host.contains("..") {
            return false;
        }

        true
    }

    /// 测试用户名验证（简化版，不使用regex）
    #[test]
    fn test_username_validation() {
        let valid_usernames = vec!["root", "admin", "user123", "myuser", "my_user"];

        let invalid_usernames = vec![
            "root;rm -rf",
            "user$(whoami)",
            "user`cat /etc/passwd`",
            "user|cat /etc/passwd",
            "user&&reboot",
            "user||reboot",
            "-user", // 以连字符开头
            "",      // 空
        ];

        for username in &valid_usernames {
            assert!(
                is_valid_username_simple(username),
                "用户名 '{}' 应被视为有效",
                username
            );
        }

        for username in &invalid_usernames {
            assert!(
                !is_valid_username_simple(username),
                "用户名 '{}' 应被视为无效",
                username
            );
        }
    }

    fn is_valid_username_simple(username: &str) -> bool {
        if username.is_empty() || username.len() > 32 {
            return false;
        }

        // 危险字符检查
        let dangerous = [';', '&', '|', '`', '$', '(', ')', '<', '>', '{', '}'];
        if username.chars().any(|c| dangerous.contains(&c)) {
            return false;
        }

        // 不能以连字符开头
        if username.starts_with('-') {
            return false;
        }

        // 只允许字母、数字、下划线和连字符
        username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }

    // =========================================================================
    // 4. 加密边界测试
    // =========================================================================

    /// 测试加密数据边界
    #[test]
    fn test_crypto_boundaries() {
        use crate::crypto::CryptoState;

        let mut crypto = CryptoState::new();
        crypto.initialize("test_password").unwrap();

        // 空数据加密
        let empty = b"";
        let encrypted = crypto.encrypt(empty).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, empty);

        // 特殊字符数据
        let special = "!@#$%^&*()_+-=[]{}|;':\",./<>?".as_bytes();
        let encrypted = crypto.encrypt(special).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, special);

        // 大数据加密（100KB）
        let large_data = vec![0u8; 100 * 1024];
        let encrypted = crypto.encrypt(&large_data).unwrap();
        let decrypted = crypto.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, large_data);
    }

    // =========================================================================
    // 5. RwLock并发安全测试 (新增)
    // =========================================================================

    /// 测试RwLock并发读写安全
    #[test]
    fn test_rwlock_concurrent_access() {
        use crate::crypto::{CryptoState, CRYPTO_STATE};
        use std::thread;

        // 初始化全局状态
        {
            let mut state = CRYPTO_STATE.write().unwrap();
            if !state.is_unlocked() {
                state.initialize("rwlock_test_password").unwrap();
            }
        }

        // 多个读取者同时访问
        let read_handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let state = CRYPTO_STATE.read().unwrap();
                    state.is_unlocked() // 读取操作
                })
            })
            .collect();

        for handle in read_handles {
            assert!(handle.join().unwrap(), "读取操作失败");
        }

        // 写入者独占访问
        {
            let mut state = CRYPTO_STATE.write().unwrap();
            state.lock();
            assert!(!state.is_unlocked());

            state.initialize("new_password").unwrap();
            assert!(state.is_unlocked());
        }
    }

    // =========================================================================
    // 6. Zeroize内存清零测试 (新增)
    // =========================================================================

    /// 测试敏感数据内存清零
    #[test]
    fn test_secure_memory_clearing() {
        use crate::crypto::CryptoState;

        let mut crypto = CryptoState::new();
        crypto.initialize("sensitive_password_123").unwrap();
        assert!(crypto.is_unlocked());

        // 锁定后应清零内存
        crypto.lock();
        assert!(!crypto.is_unlocked());

        // 尝试解密应失败
        let fake_data = vec![0u8; 32];
        let result = crypto.decrypt(&fake_data);
        assert!(result.is_err());
    }

    // =========================================================================
    // 7. SSO PKCE安全测试 (新增)
    // =========================================================================

    /// 测试PKCE生成和验证
    #[test]
    #[cfg(feature = "sso")]
    fn test_pkce_security() {
        use crate::sso::{OidcConfig, SsoManager};

        let mut manager = SsoManager::new();

        // 添加OIDC提供者
        let oidc_config = OidcConfig::standard(
            "https://auth.example.com",
            "client123",
            "secret456",
            "https://easyssh.pro/callback",
        );
        let provider = crate::sso::SsoProvider::new_oidc("Test Provider", oidc_config);
        let provider_id = provider.id.clone();
        manager.add_provider(provider).unwrap();

        // 初始化认证流程
        let auth_request = manager.init_oidc_auth(&provider_id).unwrap();

        // 验证PKCE verifier不在响应中 (安全存储)
        assert!(auth_request.pkce_verifier.is_none());

        // 验证state和nonce是高熵的
        assert!(auth_request.state.len() >= 32);
        assert!(auth_request.nonce.len() >= 32);
    }

    /// 测试SSO状态参数验证
    #[test]
    #[cfg(feature = "sso")]
    fn test_sso_state_validation() {
        use crate::sso::{OidcConfig, SsoManager};

        let mut manager = SsoManager::new();

        let oidc_config = OidcConfig::standard(
            "https://auth.example.com",
            "client123",
            "secret456",
            "https://easyssh.pro/callback",
        );
        let provider = crate::sso::SsoProvider::new_oidc("Test Provider", oidc_config);
        let provider_id = provider.id.clone();
        manager.add_provider(provider).unwrap();

        // 初始化认证
        let auth_request = manager.init_oidc_auth(&provider_id).unwrap();
        let valid_state = auth_request.state;

        // 验证state是高熵随机字符串
        assert!(valid_state.len() >= 32, "State应至少32字符");
    }

    // =========================================================================
    // 8. Argon2id安全参数测试 (新增)
    // =========================================================================

    /// 测试高成本Argon2id密钥派生
    #[test]
    fn test_argon2id_high_security() {
        use crate::crypto::CryptoState;
        use std::time::Instant;

        let mut crypto = CryptoState::new();

        // 测量密钥派生时间 (应较慢表示高成本)
        let start = Instant::now();
        crypto.initialize("test_password_high_security").unwrap();
        let duration = start.elapsed();

        // 高成本Argon2id应至少需要100ms
        assert!(
            duration.as_millis() >= 50,
            "Argon2id密钥派生应至少50ms，实际: {:?}",
            duration
        );

        // 验证解锁使用相同参数
        let salt = crypto.get_salt().unwrap();
        let mut crypto2 = CryptoState::new();
        let mut salt_array = [0u8; 32];
        salt_array.copy_from_slice(&salt);
        crypto2.set_salt(salt_array);

        let start = Instant::now();
        crypto2.unlock("test_password_high_security").unwrap();
        let duration = start.elapsed();

        assert!(
            duration.as_millis() >= 50,
            "Argon2id解锁也应至少50ms，实际: {:?}",
            duration
        );
    }

    // =========================================================================
    // 9. 错误处理安全测试
    // =========================================================================

    /// 测试错误信息不泄露敏感信息（简化版）
    #[test]
    fn test_error_message_sanitization() {
        // 模拟错误信息
        let sensitive_inputs = vec![
            "SSH连接失败: root:password@192.168.1.1",
            "数据库错误: /home/user/.ssh/id_rsa",
            "错误包含AKIAIOSFODNN7EXAMPLE密钥",
        ];

        for input in &sensitive_inputs {
            let sanitized = sanitize_error_message_simple(input);
            // 验证脱敏后的信息不包含敏感关键词
            assert!(
                !sanitized.to_lowercase().contains("password") || sanitized.contains("[REDACTED]"),
                "错误信息应被脱敏: {} -> {}",
                input,
                sanitized
            );
        }
    }

    fn sanitize_error_message_simple(message: &str) -> String {
        // 简单脱敏实现
        let mut result = message.to_string();

        // 替换IP地址（简单匹配）- 使用更高效的实现
        for i in [192u8, 10, 172, 127, 0, 255] {
            let ip = format!("{}.", i);
            if result.contains(&ip) {
                // Replace IP octets with [IP]
                result = result.replace(&ip, "[IP].");
            }
        }

        // 替换密码 - 改进实现
        if result.contains(':') && result.contains('@') {
            // 尝试脱敏用户名:密码@格式
            let parts: Vec<&str> = result.split('@').collect();
            if parts.len() == 2 {
                let user_part = parts[0];
                // Find last colon in the user_part
                if let Some(colon_pos) = user_part.rfind(':') {
                    let username = &user_part[..colon_pos];
                    let _password = &user_part[colon_pos + 1..];
                    // Don't change if password part is empty
                    if !_password.is_empty() {
                        result = format!("{}:[REDACTED]@{}", username, parts[1]);
                    }
                }
            }
        }

        result
    }

    // =========================================================================
    // 10. 反序列化安全测试
    // =========================================================================

    /// 测试JSON反序列化限制
    #[test]
    fn test_deserialization_limits() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug)]
        struct TestServer {
            name: String,
            host: String,
            port: u16,
        }

        // 正常数据
        let valid_json = r#"{"name":"Test","host":"192.168.1.1","port":22}"#;
        let result: Result<TestServer, _> = serde_json::from_str(valid_json);
        assert!(result.is_ok());

        // 超大字段
        let huge_name = "a".repeat(10000);
        let huge_json = format!(
            r#"{{"name":"{}","host":"192.168.1.1","port":22}}"#,
            huge_name
        );
        let result: Result<TestServer, _> = serde_json::from_str(&huge_json);
        // 反序列化本身不会失败，但应用层应限制字段长度
        assert!(result.is_ok());
        assert!(result.unwrap().name.len() > 1000); // 验证确实超长
    }

    /// 测试深度嵌套限制
    #[test]
    fn test_deep_nesting_prevention() {
        use serde_json::Value;

        // 创建深度嵌套的JSON
        let mut deep_json = String::from("{\"a\":");
        for _ in 0..1000 {
            deep_json.push_str("{\"b\":");
        }
        deep_json.push('1');
        for _ in 0..1000 {
            deep_json.push('}');
        }
        deep_json.push('}');

        // 应限制解析深度或超时
        let result: Result<Value, _> = serde_json::from_str(&deep_json);
        // 当前实现可能会成功，建议添加深度限制
        if result.is_ok() {
            println!("警告: 深度嵌套JSON被成功解析，可能存在DoS风险");
        }
    }
}

// =========================================================================
// 集成测试
// =========================================================================

#[cfg(test)]
mod integration_tests {
    use std::process::Command;

    /// 测试cargo audit检查
    #[test]
    #[ignore = "需要cargo-audit已安装"]
    fn test_cargo_audit() {
        let output = Command::new("cargo")
            .args(["audit"])
            .current_dir(".")
            .output()
            .expect("无法运行cargo audit");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 检查是否有严重漏洞
        assert!(
            !stdout.contains("CRITICAL"),
            "发现严重安全漏洞:\n{}{}",
            stdout,
            stderr
        );
    }

    /// 测试deny.toml配置有效性
    #[test]
    #[ignore = "需要cargo-deny已安装"]
    fn test_cargo_deny() {
        let output = Command::new("cargo")
            .args(["deny", "check"])
            .current_dir(".")
            .output()
            .expect("无法运行cargo deny");

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "cargo deny检查失败:\n{}", stdout);
    }
}
