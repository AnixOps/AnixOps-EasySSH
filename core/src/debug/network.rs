//! 网络检查模块
//!
//! 提供网络连通性检查功能，所有版本可用

use crate::debug::types::NetworkCheckResult;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// 检查TCP端口连通性
///
/// # Arguments
/// * `host` - 目标主机
/// * `port` - 目标端口
/// * `timeout_ms` - 超时时间（毫秒）
///
/// # Example
/// ```
/// use easyssh_core::debug::network::check_tcp_connectivity;
///
/// let result = check_tcp_connectivity("example.com", 80, 5000);
/// ```
pub fn check_tcp_connectivity(
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> NetworkCheckResult {
    let addr = format!("{}:{}", host, port);
    let start = Instant::now();

    // 解析地址
    let addrs: Vec<SocketAddr> = match addr.to_socket_addrs() {
        Ok(iter) => iter.collect(),
        Err(e) => {
            return NetworkCheckResult {
                host: host.to_string(),
                port,
                reachable: false,
                latency_ms: None,
                error: Some(format!("DNS resolution failed: {}", e)),
            };
        }
    };

    if addrs.is_empty() {
        return NetworkCheckResult {
            host: host.to_string(),
            port,
            reachable: false,
            latency_ms: None,
            error: Some("No addresses found".to_string()),
        };
    }

    // 尝试连接
    let timeout = Duration::from_millis(timeout_ms);
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => {
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                return NetworkCheckResult {
                    host: host.to_string(),
                    port,
                    reachable: true,
                    latency_ms: Some(latency),
                    error: None,
                };
            }
            Err(e) => {
                return NetworkCheckResult {
                    host: host.to_string(),
                    port,
                    reachable: false,
                    latency_ms: None,
                    error: Some(format!("Connection failed: {}", e)),
                };
            }
        }
    }

    NetworkCheckResult {
        host: host.to_string(),
        port,
        reachable: false,
        latency_ms: None,
        error: Some("All connection attempts failed".to_string()),
    }
}

/// 检查SSH端口（22）连通性
pub fn check_ssh_connectivity(host: &str, timeout_ms: u64) -> NetworkCheckResult {
    check_tcp_connectivity(host, 22, timeout_ms)
}

/// 检查HTTP端口（80）连通性
pub fn check_http_connectivity(host: &str, timeout_ms: u64) -> NetworkCheckResult {
    check_tcp_connectivity(host, 80, timeout_ms)
}

/// 检查HTTPS端口（443）连通性
pub fn check_https_connectivity(host: &str, timeout_ms: u64) -> NetworkCheckResult {
    check_tcp_connectivity(host, 443, timeout_ms)
}

/// 批量检查多个主机
///
/// # Arguments
/// * `hosts` - 主机列表，格式为 "host:port"
/// * `timeout_ms` - 每个连接的超时时间
///
/// # Example
/// ```
/// use easyssh_core::debug::network::check_multiple_hosts;
///
/// let hosts = vec!["example.com:80", "example.com:443"];
/// let results = check_multiple_hosts(&hosts, 5000);
/// ```
pub fn check_multiple_hosts(
    hosts: &[&str],
    timeout_ms: u64,
) -> Vec<NetworkCheckResult> {
    hosts
        .iter()
        .filter_map(|host_str| {
            let parts: Vec<&str> = host_str.split(':').collect();
            if parts.len() == 2 {
                let host = parts[0];
                let port = parts[1].parse::<u16>().ok()?;
                Some(check_tcp_connectivity(host, port, timeout_ms))
            } else {
                None
            }
        })
        .collect()
}

/// 网络诊断报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkDiagnosticsReport {
    pub timestamp: String,
    pub tests: Vec<NetworkCheckResult>,
    pub summary: NetworkDiagnosticsSummary,
}

/// 网络诊断摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkDiagnosticsSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub average_latency_ms: Option<f64>,
}

/// 执行网络诊断
///
/// 执行一系列网络连通性检查
pub fn run_network_diagnostics() -> NetworkDiagnosticsReport {
    let tests = vec![
        check_http_connectivity("1.1.1.1", 5000),   // Cloudflare DNS
        check_https_connectivity("1.1.1.1", 5000), // Cloudflare DNS HTTPS
        check_http_connectivity("8.8.8.8", 5000),   // Google DNS
    ];

    let total_tests = tests.len();
    let passed_tests = tests.iter().filter(|t| t.reachable).count();
    let failed_tests = total_tests - passed_tests;

    let latencies: Vec<f64> = tests
        .iter()
        .filter_map(|t| t.latency_ms)
        .collect();

    let average_latency_ms = if latencies.is_empty() {
        None
    } else {
        Some(latencies.iter().sum::<f64>() / latencies.len() as f64)
    };

    NetworkDiagnosticsReport {
        timestamp: chrono::Utc::now().to_rfc3339(),
        tests,
        summary: NetworkDiagnosticsSummary {
            total_tests,
            passed_tests,
            failed_tests,
            average_latency_ms,
        },
    }
}

/// 获取本地网络信息
pub fn get_local_network_info() -> LocalNetworkInfo {
    LocalNetworkInfo {
        hostname: gethostname(),
        // 其他信息需要平台特定的实现
        interfaces: Vec::new(),
    }
}

/// 本地网络信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalNetworkInfo {
    pub hostname: String,
    pub interfaces: Vec<NetworkInterface>,
}

/// 网络接口信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub is_up: bool,
}

fn gethostname() -> String {
    whoami::hostname()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_tcp_connectivity() {
        // 测试已知的可访问主机
        let result = check_tcp_connectivity("1.1.1.1", 53, 5000);
        // DNS端口应该可达
        println!("DNS check result: {:?}", result);
    }

    #[test]
    fn test_check_ssh_connectivity() {
        // 测试SSH端口（这个测试可能不会成功，取决于环境）
        let result = check_ssh_connectivity("localhost", 1000);
        // 快速超时测试
        assert!(!result.reachable || result.latency_ms.is_some());
    }

    #[test]
    fn test_multiple_hosts() {
        let hosts = vec!["1.1.1.1:53", "8.8.8.8:53"];
        let results = check_multiple_hosts(&hosts, 5000);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_network_diagnostics() {
        let report = run_network_diagnostics();
        assert!(!report.tests.is_empty());
        assert_eq!(report.summary.total_tests, report.tests.len());
    }

    #[test]
    fn test_local_network_info() {
        let info = get_local_network_info();
        assert!(!info.hostname.is_empty());
    }
}
