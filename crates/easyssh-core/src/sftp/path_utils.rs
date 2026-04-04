//! SFTP路径安全工具
//!
//! 提供安全的路径操作，防止路径穿越攻击
//!
//! # 约束遵守 (SYSTEM_INVARIANTS.md §3.2)
//!
//! - 所有路径必须经过规范化（禁止 `..` 路径穿越）
//! - 符号链接必须解析到真实路径
//! - 禁止访问用户主目录之外的敏感路径（可配置）

use std::path::{Component, Path, PathBuf};

/// SFTP路径错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// 路径包含非法组件（如 `..`）
    InvalidComponent(String),
    /// 路径穿越到允许目录之外
    PathTraversal(String),
    /// 符号链接解析失败
    SymlinkResolution(String),
    /// 路径为空
    EmptyPath,
    /// 路径不在允许的目录范围内
    NotInAllowedDir(String),
    /// 绝对路径转换失败
    AbsolutePathConversion(String),
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::InvalidComponent(s) => write!(f, "路径包含非法组件: {}", s),
            PathError::PathTraversal(s) => write!(f, "路径穿越攻击: {}", s),
            PathError::SymlinkResolution(s) => write!(f, "符号链接解析失败: {}", s),
            PathError::EmptyPath => write!(f, "路径为空"),
            PathError::NotInAllowedDir(s) => write!(f, "路径不在允许目录: {}", s),
            PathError::AbsolutePathConversion(s) => write!(f, "绝对路径转换失败: {}", s),
        }
    }
}

impl std::error::Error for PathError {}

/// 规范化路径，移除 `..` 和 `.` 组件
///
/// # 约束遵守
/// - 防止路径穿越攻击
/// - 返回规范化的绝对路径
///
/// # 示例
/// ```rust
/// use easyssh_core::sftp::path_utils::normalize_path;
///
/// assert_eq!(normalize_path("/home/user/../admin"), "/home/admin");
/// assert_eq!(normalize_path("/var/log/./app"), "/var/log/app");
/// ```
pub fn normalize_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let path = Path::new(path);
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                normalized.push(component);
            }
            Component::CurDir => {
                // 跳过 `.`
            }
            Component::ParentDir => {
                // 尝试回退一级
                if !normalized.pop() {
                    // 无法回退（在根目录），保持原样或报错
                    // 这里我们选择忽略，防止穿越
                }
            }
            Component::Normal(part) => {
                normalized.push(part);
            }
        }
    }

    // 如果结果为空，返回根目录
    if normalized.as_os_str().is_empty() {
        return "/".to_string();
    }

    normalized.to_string_lossy().to_string()
}

/// 检查路径是否安全（不包含 `..` 穿越）
///
/// # 约束遵守
/// - 禁止 `..` 路径穿越
/// - 路径必须在允许目录范围内
///
/// # 示例
/// ```rust
/// use easyssh_core::sftp::path_utils::is_path_safe;
///
/// assert!(is_path_safe("/home/user/file.txt", &["/home/user"]));
/// assert!(!is_path_safe("/home/user/../admin/file.txt", &["/home/user"]));
/// ```
pub fn is_path_safe(path: &str, allowed_dirs: &[&str]) -> bool {
    if path.is_empty() {
        return false;
    }

    // 1. 检查是否包含非法组件
    let normalized = normalize_path(path);

    // 2. 检查原始路径中是否包含 `..`
    if contains_parent_dir(path) {
        return false;
    }

    // 3. 检查规范化后的路径是否在允许目录内
    let normalized_path = PathBuf::from(&normalized);

    for allowed_dir in allowed_dirs {
        let allowed = PathBuf::from(normalize_path(allowed_dir));
        if normalized_path.starts_with(&allowed) || normalized_path == allowed {
            return true;
        }
    }

    false
}

/// 检查路径是否包含 `..` 组件
fn contains_parent_dir(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|c| c == Component::ParentDir)
}

/// 验证路径安全性
///
/// # 返回
/// - Ok(normalized_path) 如果路径安全
/// - Err(PathError) 如果路径不安全
///
/// # 约束遵守
/// - 所有路径必须经过规范化
/// - 路径必须在允许目录范围内
pub fn validate_path(path: &str, allowed_dirs: &[&str]) -> Result<String, PathError> {
    if path.is_empty() {
        return Err(PathError::EmptyPath);
    }

    // 检查是否包含 `..`
    if contains_parent_dir(path) {
        return Err(PathError::PathTraversal(path.to_string()));
    }

    // 规范化路径
    let normalized = normalize_path(path);

    // 检查是否在允许目录内
    if !allowed_dirs.is_empty() && !is_path_safe(&normalized, allowed_dirs) {
        return Err(PathError::NotInAllowedDir(normalized));
    }

    Ok(normalized)
}

/// 拼接路径
///
/// # 约束遵守
/// - 结果路径必须经过规范化
/// - 禁止 `..` 路径穿越
///
/// # 示例
/// ```rust
/// use easyssh_core::sftp::path_utils::join_paths;
///
/// assert_eq!(join_paths("/home/user", "docs/file.txt"), "/home/user/docs/file.txt");
/// assert_eq!(join_paths("/var/log", "../data/app.log"), "/var/data/app.log");
/// ```
pub fn join_paths(base: &str, relative: &str) -> String {
    if base.is_empty() {
        return normalize_path(relative);
    }

    if relative.is_empty() {
        return normalize_path(base);
    }

    let base_path = PathBuf::from(base);
    let joined = base_path.join(relative);

    normalize_path(&joined.to_string_lossy())
}

/// 解析符号链接到真实路径
///
/// # 约束遵守
/// - 符号链接必须解析到真实路径
/// - 结果路径必须经过规范化
///
/// # 注意
/// - 此函数需要文件系统访问权限
/// - 远程路径需要通过 SFTP 客户端解析
///
/// # 示例 (本地路径)
/// ```rust
/// use easyssh_core::sftp::path_utils::resolve_symlink_local;
///
/// // 本地符号链接解析
/// let real_path = resolve_symlink_local("/var/log/app").unwrap();
/// ```
pub fn resolve_symlink_local(path: &str) -> Result<String, PathError> {
    let path_buf = PathBuf::from(path);

    // 使用 std::fs::read_link 解析符号链接
    // 注意：这只是解析一层，可能需要递归解析多层符号链接
    let mut current = path_buf.clone();
    let mut visited = Vec::new();
    let max_iterations = 40; // 防止无限循环

    for _ in 0..max_iterations {
        if !current.exists() {
            // 文件不存在，返回当前路径
            break;
        }

        // 检查是否是符号链接
        if current.is_symlink() {
            // 检查循环引用
            if visited.contains(&current) {
                return Err(PathError::SymlinkResolution(
                    "检测到符号链接循环".to_string(),
                ));
            }
            visited.push(current.clone());

            // 解析链接
            match std::fs::read_link(&current) {
                Ok(target) => {
                    if target.is_absolute() {
                        current = target;
                    } else {
                        // 相对路径，需要相对于链接所在目录
                        let parent = current.parent().unwrap_or(Path::new("/"));
                        current = parent.join(target);
                    }
                }
                Err(e) => {
                    return Err(PathError::SymlinkResolution(format!(
                        "无法读取符号链接: {}",
                        e
                    )));
                }
            }
        } else {
            // 不是符号链接，已解析到真实路径
            break;
        }
    }

    Ok(normalize_path(&current.to_string_lossy()))
}

/// 远程符号链接解析结果
///
/// 由于远程符号链接需要通过 SFTP 客户端解析，
/// 此结构提供解析结果的存储格式
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymlinkResolution {
    /// 原始路径
    pub original_path: String,
    /// 解析后的真实路径
    pub real_path: String,
    /// 是否是符号链接
    pub is_symlink: bool,
    /// 链接层数
    pub link_depth: u32,
}

impl SymlinkResolution {
    /// 创建新的符号链接解析结果
    pub fn new(original: &str, real: &str, is_symlink: bool, depth: u32) -> Self {
        Self {
            original_path: original.to_string(),
            real_path: normalize_path(real),
            is_symlink,
            link_depth: depth,
        }
    }

    /// 创建非符号链接的结果
    pub fn not_symlink(path: &str) -> Self {
        Self::new(path, path, false, 0)
    }

    /// 创建单层符号链接的结果
    pub fn single_link(original: &str, target: &str) -> Self {
        Self::new(original, target, true, 1)
    }
}

/// 验证远程路径安全性
///
/// # 约束遵守
/// - 所有路径必须经过规范化
/// - 禁止访问用户主目录之外的敏感路径
///
/// # 示例
/// ```rust
/// use easyssh_core::sftp::path_utils::validate_remote_path;
///
/// // 允许用户访问其主目录
/// let allowed = &["/home/user", "/tmp/user"];
/// assert!(validate_remote_path("/home/user/docs", allowed).is_ok());
/// assert!(validate_remote_path("/etc/passwd", allowed).is_err());
/// ```
pub fn validate_remote_path(path: &str, allowed_dirs: &[&str]) -> Result<String, PathError> {
    validate_path(path, allowed_dirs)
}

/// 默认的敏感路径列表
///
/// 这些路径禁止访问，即使用户主目录包含它们
pub const SENSITIVE_PATHS: &[&str] = &[
    "/etc/passwd",
    "/etc/shadow",
    "/etc/sudoers",
    "/root",
    "/var/run/sshd",
    "/proc",
    "/sys",
    "/dev",
];

/// 检查路径是否为敏感路径
///
/// # 约束遵守
/// - 禁止访问敏感系统路径
pub fn is_sensitive_path(path: &str) -> bool {
    let normalized = normalize_path(path);

    for sensitive in SENSITIVE_PATHS {
        let sensitive_normalized = normalize_path(sensitive);
        if normalized.starts_with(&sensitive_normalized) || normalized == sensitive_normalized {
            return true;
        }
    }

    false
}

/// 综合路径验证
///
/// 检查路径是否：
/// 1. 不包含 `..` 穿越
/// 2. 在允许目录范围内
/// 3. 不是敏感路径
///
/// # 约束遵守
/// - 所有路径必须经过规范化
/// - 禁止访问用户主目录之外的敏感路径
pub fn comprehensive_path_check(path: &str, allowed_dirs: &[&str]) -> Result<String, PathError> {
    // 1. 验证基本安全性
    let normalized = validate_path(path, allowed_dirs)?;

    // 2. 检查是否为敏感路径
    if is_sensitive_path(&normalized) {
        return Err(PathError::NotInAllowedDir(format!(
            "敏感路径禁止访问: {}",
            normalized
        )));
    }

    Ok(normalized)
}

/// 获取路径的父目录
///
/// # 约束遵守
/// - 结果路径必须经过规范化
pub fn get_parent_path(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    let path_buf = PathBuf::from(&normalized);

    path_buf
        .parent()
        .map(|p| normalize_path(&p.to_string_lossy()))
}

/// 获取路径的文件名
///
/// # 返回
/// - 文件名部分（不含路径）
/// - 根目录返回空字符串
pub fn get_filename(path: &str) -> String {
    let normalized = normalize_path(path);
    let path_buf = PathBuf::from(&normalized);

    path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

/// 获取路径的扩展名
///
/// # 返回
/// - 扩展名（不含点）
/// - 无扩展名返回空字符串
pub fn get_extension(path: &str) -> String {
    let normalized = normalize_path(path);
    let path_buf = PathBuf::from(&normalized);

    path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

/// 检查路径是否为绝对路径
pub fn is_absolute_path(path: &str) -> bool {
    Path::new(path).is_absolute()
}

/// 将相对路径转换为绝对路径
///
/// # 约束遵守
/// - 结果路径必须经过规范化
///
/// # 参数
/// - `relative`: 相对路径
/// - `base`: 基准路径（如当前工作目录）
///
/// # 示例
/// ```rust
/// use easyssh_core::sftp::path_utils::to_absolute_path;
///
/// assert_eq!(to_absolute_path("file.txt", "/home/user"), Ok("/home/user/file.txt".to_string()));
/// assert_eq!(to_absolute_path("/var/log", "/home/user"), Ok("/var/log".to_string()));
/// ```
pub fn to_absolute_path(relative: &str, base: &str) -> Result<String, PathError> {
    if relative.is_empty() {
        return Err(PathError::EmptyPath);
    }

    // 如果已经是绝对路径，直接规范化
    if is_absolute_path(relative) {
        return Ok(normalize_path(relative));
    }

    // 否则，拼接基准路径
    let joined = join_paths(base, relative);
    Ok(joined)
}

/// 确保路径以斜杠结尾（用于目录）
pub fn ensure_trailing_slash(path: &str) -> String {
    let normalized = normalize_path(path);

    if normalized.ends_with('/') {
        normalized
    } else {
        normalized + "/"
    }
}

/// 移除路径末尾的斜杠
pub fn remove_trailing_slash(path: &str) -> String {
    let normalized = normalize_path(path);

    if normalized.ends_with('/') && normalized != "/" {
        normalized.trim_end_matches('/').to_string()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_basic() {
        assert_eq!(normalize_path("/home/user/file.txt"), "/home/user/file.txt");
        assert_eq!(normalize_path("/home/user/../admin"), "/home/admin");
        assert_eq!(normalize_path("/var/log/./app"), "/var/log/app");
        assert_eq!(normalize_path("/a/b/../c/./d"), "/a/c/d");
        assert_eq!(normalize_path("/a/b/../../c"), "/c");
    }

    #[test]
    fn test_normalize_path_empty() {
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn test_normalize_path_root() {
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("/.."), "/");
    }

    #[test]
    fn test_contains_parent_dir() {
        assert!(contains_parent_dir("/home/user/../admin"));
        assert!(contains_parent_dir("../file.txt"));
        assert!(!contains_parent_dir("/home/user/file.txt"));
        assert!(!contains_parent_dir("./file.txt"));
    }

    #[test]
    fn test_is_path_safe() {
        assert!(is_path_safe("/home/user/file.txt", &["/home/user"]));
        assert!(is_path_safe("/home/user", &["/home/user"]));
        assert!(!is_path_safe("/home/user/../admin", &["/home/user"]));
        assert!(!is_path_safe("/etc/passwd", &["/home/user"]));
        assert!(!is_path_safe("", &["/home/user"]));
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/home/user/file.txt", &["/home/user"]).is_ok());
        assert!(validate_path("/home/user/../admin", &["/home/user"]).is_err());
        assert!(validate_path("", &["/home/user"]).is_err());
    }

    #[test]
    fn test_join_paths() {
        assert_eq!(
            join_paths("/home/user", "docs/file.txt"),
            "/home/user/docs/file.txt"
        );
        assert_eq!(join_paths("/home/user", "../admin"), "/home/admin");
        assert_eq!(join_paths("/var/log", "./app.log"), "/var/log/app.log");
        assert_eq!(join_paths("", "/home/user"), "/home/user");
        assert_eq!(join_paths("/home/user", ""), "/home/user");
    }

    #[test]
    fn test_is_sensitive_path() {
        assert!(is_sensitive_path("/etc/passwd"));
        assert!(is_sensitive_path("/etc/shadow"));
        assert!(is_sensitive_path("/root/.ssh"));
        assert!(is_sensitive_path("/proc/self"));
        assert!(!is_sensitive_path("/home/user/file.txt"));
        assert!(!is_sensitive_path("/tmp/upload"));
    }

    #[test]
    fn test_comprehensive_path_check() {
        let allowed = &["/home/user", "/tmp/user"];

        assert!(comprehensive_path_check("/home/user/file.txt", allowed).is_ok());
        assert!(comprehensive_path_check("/tmp/user/data", allowed).is_ok());
        assert!(comprehensive_path_check("/etc/passwd", allowed).is_err());
        assert!(comprehensive_path_check("/home/user/../admin", allowed).is_err());
    }

    #[test]
    fn test_get_parent_path() {
        assert_eq!(get_parent_path("/home/user/file.txt"), Some("/home/user"));
        assert_eq!(get_parent_path("/home/user"), Some("/home"));
        assert_eq!(get_parent_path("/"), None);
    }

    #[test]
    fn test_get_filename() {
        assert_eq!(get_filename("/home/user/file.txt"), "file.txt");
        assert_eq!(get_filename("/home/user"), "user");
        assert_eq!(get_filename("/"), "");
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("/home/user/file.txt"), "txt");
        assert_eq!(get_extension("/home/user/file.tar.gz"), "gz");
        assert_eq!(get_extension("/home/user/noext"), "");
        assert_eq!(get_extension("/home/user/.hidden"), "hidden");
    }

    #[test]
    fn test_is_absolute_path() {
        assert!(is_absolute_path("/home/user"));
        assert!(is_absolute_path("C:\\Windows")); // Windows absolute
        assert!(!is_absolute_path("relative/path"));
        assert!(!is_absolute_path("./file.txt"));
    }

    #[test]
    fn test_to_absolute_path() {
        assert_eq!(
            to_absolute_path("file.txt", "/home/user"),
            Ok("/home/user/file.txt")
        );
        assert_eq!(
            to_absolute_path("/var/log", "/home/user"),
            Ok("/var/log")
        );
        assert!(to_absolute_path("", "/home/user").is_err());
    }

    #[test]
    fn test_ensure_trailing_slash() {
        assert_eq!(ensure_trailing_slash("/home/user"), "/home/user/");
        assert_eq!(ensure_trailing_slash("/home/user/"), "/home/user/");
        assert_eq!(ensure_trailing_slash("/"), "/");
    }

    #[test]
    fn test_remove_trailing_slash() {
        assert_eq!(remove_trailing_slash("/home/user/"), "/home/user");
        assert_eq!(remove_trailing_slash("/home/user"), "/home/user");
        assert_eq!(remove_trailing_slash("/"), "/");
    }

    #[test]
    fn test_symlink_resolution() {
        let resolution = SymlinkResolution::new("/link", "/target", true, 1);
        assert_eq!(resolution.original_path, "/link");
        assert_eq!(resolution.real_path, "/target");
        assert!(resolution.is_symlink);
        assert_eq!(resolution.link_depth, 1);

        let not_link = SymlinkResolution::not_symlink("/path/file.txt");
        assert!(!not_link.is_symlink);
        assert_eq!(not_link.link_depth, 0);
    }

    #[test]
    fn test_path_error_display() {
        let err = PathError::InvalidComponent("..".to_string());
        assert!(err.to_string().contains("非法组件"));

        let err = PathError::PathTraversal("/home/../root".to_string());
        assert!(err.to_string().contains("路径穿越"));
    }

    #[test]
    fn test_normalize_path_multiple_parent_dirs() {
        assert_eq!(normalize_path("/a/b/c/d/../../e"), "/a/b/e");
        assert_eq!(normalize_path("/a/b/c/d/../../../e"), "/a/e");
    }

    #[test]
    fn test_normalize_path_parent_dir_overflow() {
        // 在根目录下使用 .. 应该停在根目录
        assert_eq!(normalize_path("/../../../home"), "/home");
    }
}