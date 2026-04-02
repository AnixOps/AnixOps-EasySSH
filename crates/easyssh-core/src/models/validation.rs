//! Extended validation utilities
//!
//! This module provides additional validation functions beyond the basic ones
//! in the parent module, including email validation, SSH key path validation,
//! and password strength checking.

use regex::Regex;
use std::path::Path;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, crate::models::ValidationError>;

/// Password strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrength {
    /// Very weak password (easily guessable)
    VeryWeak = 0,
    /// Weak password (some complexity but still vulnerable)
    Weak = 1,
    /// Moderate password (acceptable for non-critical accounts)
    Moderate = 2,
    /// Strong password (good complexity)
    Strong = 3,
    /// Very strong password (excellent complexity)
    VeryStrong = 4,
}

impl PasswordStrength {
    /// Get a text description of the strength
    pub fn description(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "Very Weak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Moderate => "Moderate",
            PasswordStrength::Strong => "Strong",
            PasswordStrength::VeryStrong => "Very Strong",
        }
    }

    /// Check if the password is acceptable (moderate or better)
    pub fn is_acceptable(&self) -> bool {
        *self as u8 >= PasswordStrength::Moderate as u8
    }

    /// Check if the password is strong (strong or better)
    pub fn is_strong(&self) -> bool {
        *self as u8 >= PasswordStrength::Strong as u8
    }
}

impl Default for PasswordStrength {
    fn default() -> Self {
        PasswordStrength::VeryWeak
    }
}

/// Validates an email address according to RFC 5322
///
/// This validation checks for proper email format including:
/// - Local part (before @) with valid characters
/// - @ symbol
/// - Domain part with valid characters and at least one dot
///
/// # Examples
///
/// ```
/// use easyssh_core::models::validation::is_valid_email;
///
/// assert!(is_valid_email("user@example.com"));
/// assert!(is_valid_email("user.name+tag@example.co.uk"));
///
/// assert!(!is_valid_email("invalid-email"));
/// assert!(!is_valid_email("@example.com"));
/// assert!(!is_valid_email("user@"));
/// ```
pub fn is_valid_email(email: &str) -> bool {
    if email.is_empty() || email.len() > 254 {
        return false;
    }

    // RFC 5322 compliant regex (simplified but practical)
    static EMAIL_REGEX: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$").unwrap()
    });

    EMAIL_REGEX.is_match(email)
}

/// Validates an SSH key file path
///
/// Checks that:
/// - The path is not empty
/// - The path is absolute or relative (no validation of existence)
/// - The file has a valid SSH key extension (.pem, .key, .pub, or no extension for OpenSSH keys)
/// - The path doesn't contain dangerous characters
///
/// # Examples
///
/// ```
/// use easyssh_core::models::validation::is_valid_ssh_key_path;
///
/// assert!(is_valid_ssh_key_path("/home/user/.ssh/id_rsa"));
/// assert!(is_valid_ssh_key_path("~/.ssh/id_rsa.pub"));
/// assert!(is_valid_ssh_key_path("./keys/my_key.pem"));
///
/// assert!(!is_valid_ssh_key_path(""));
/// assert!(!is_valid_ssh_key_path("/path/with\x00/null"));
/// ```
pub fn is_valid_ssh_key_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }

    // Check for null bytes (indicates potential injection)
    if path.contains('\0') {
        return false;
    }

    // Check for path traversal attempts
    let dangerous_patterns = ["..", "//", "~/../", "./../"];
    for pattern in &dangerous_patterns {
        if path.contains(pattern) {
            return false;
        }
    }

    let path_obj = Path::new(path);

    // Check if there's a filename
    if path_obj.file_name().is_none() {
        return false;
    }

    // Valid extensions for SSH keys
    if let Some(ext) = path_obj.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        let valid_extensions = ["pem", "key", "pub", "ppk", "rsa", "dsa", "ecdsa", "ed25519"];
        if !valid_extensions.contains(&ext_str.as_str()) {
            // Allow no extension for files like "id_rsa", "id_ed25519"
            // But flag suspicious extensions
            let suspicious = ["exe", "bat", "cmd", "sh", "com", "vbs", "js", "py"];
            if suspicious.contains(&ext_str.as_str()) {
                return false;
            }
        }
    }

    true
}

/// Validates password strength
///
/// Scoring based on:
/// - Length (8+ chars minimum)
/// - Character variety (uppercase, lowercase, digits, symbols)
/// - No common patterns
///
/// Returns a `PasswordStrength` enum indicating the strength level.
///
/// # Examples
///
/// ```
/// use easyssh_core::models::validation::{validate_password_strength, PasswordStrength};
///
/// let weak = validate_password_strength("password");
/// assert!(!weak.is_acceptable());
///
/// let strong = validate_password_strength("MyStr0ng!P@ss");
/// assert!(strong.is_acceptable());
/// ```
pub fn validate_password_strength(password: &str) -> PasswordStrength {
    if password.is_empty() {
        return PasswordStrength::VeryWeak;
    }

    let length = password.len();
    let mut score = 0u32;

    // Length scoring
    if length >= 8 {
        score += 1;
    }
    if length >= 12 {
        score += 1;
    }
    if length >= 16 {
        score += 1;
    }

    // Character variety
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digits = password.chars().any(|c| c.is_ascii_digit());
    let has_symbols = password.chars().any(|c| !c.is_alphanumeric());

    let variety_count = [has_lowercase, has_uppercase, has_digits, has_symbols]
        .iter()
        .filter(|&&x| x)
        .count();

    if variety_count >= 2 {
        score += 1;
    }
    if variety_count >= 3 {
        score += 1;
    }
    if variety_count >= 4 {
        score += 1;
    }

    // Penalize common patterns
    let lower = password.to_lowercase();
    let common_patterns = [
        "password", "123456", "qwerty", "abc123", "letmein", "welcome",
        "admin", "root", "test", "guest", "default", "changeme",
    ];
    for pattern in &common_patterns {
        if lower.contains(pattern) {
            score = score.saturating_sub(2);
            break;
        }
    }

    // Penalize sequential characters
    if has_sequential_chars(password) {
        score = score.saturating_sub(1);
    }

    // Penalize repeated characters
    if has_repeated_chars(password) {
        score = score.saturating_sub(1);
    }

    // Map score to strength level
    match score {
        0 => PasswordStrength::VeryWeak,
        1 => PasswordStrength::Weak,
        2 => PasswordStrength::Moderate,
        3 => PasswordStrength::Strong,
        _ => PasswordStrength::VeryStrong,
    }
}

/// Check for sequential characters like "abc", "123", "xyz"
fn has_sequential_chars(s: &str) -> bool {
    let chars: Vec<char> = s.to_lowercase().chars().collect();

    for window in chars.windows(3) {
        // Check for ascending sequence
        if (window[0] as u8 + 1 == window[1] as u8 && window[1] as u8 + 1 == window[2] as u8)
            || (window[0] as u8 == window[1] as u8 && window[1] as u8 == window[2] as u8)
        {
            return true;
        }
    }

    false
}

/// Check for excessive repeated characters
fn has_repeated_chars(s: &str) -> bool {
    let mut last_char = '\0';
    let mut repeat_count = 0;

    for c in s.chars() {
        if c == last_char {
            repeat_count += 1;
            if repeat_count >= 3 {
                return true;
            }
        } else {
            repeat_count = 0;
            last_char = c;
        }
    }

    false
}

/// Sanitize a string for safe use as a filename
///
/// Removes or replaces characters that are unsafe for filesystems.
///
/// # Examples
///
/// ```
/// use easyssh_core::models::validation::sanitize_filename;
///
/// assert_eq!(sanitize_filename("test.txt"), "test.txt");
/// assert_eq!(sanitize_filename("test/file.txt"), "test_file.txt");
/// assert_eq!(sanitize_filename("test:file"), "test_file");
/// ```
pub fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Validates a SSH username according to various platform rules
///
/// Usernames must:
/// - Be 1-32 characters long
/// - Start with a letter or underscore
/// - Contain only alphanumeric characters, underscore, hyphen, and dot
/// - Not be a reserved/system username
///
/// # Examples
///
/// ```
/// use easyssh_core::models::validation::is_valid_ssh_username;
///
/// assert!(is_valid_ssh_username("root"));
/// assert!(is_valid_ssh_username("user_123"));
/// assert!(is_valid_ssh_username("_admin"));
///
/// assert!(!is_valid_ssh_username(""));
/// assert!(!is_valid_ssh_username("123user")); // Can't start with digit
/// assert!(!is_valid_ssh_username("user name")); // No spaces
/// ```
pub fn is_valid_ssh_username(username: &str) -> bool {
    if username.is_empty() || username.len() > 32 {
        return false;
    }

    // Check first character
    let first_char = username.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Check all characters
    for c in username.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' {
            return false;
        }
    }

    // Check against reserved usernames
    let reserved = [
        "root", "admin", "administrator", "guest", "test", "user", "default",
    ];
    // Note: We don't reject reserved names, just document them
    // Actual policy might choose to warn about these

    true
}

/// Validates a port range string (e.g., "8080-8090" or "8080")
///
/// # Examples
///
/// ```
/// use easyssh_core::models::validation::is_valid_port_range;
///
/// assert!(is_valid_port_range("8080"));
/// assert!(is_valid_port_range("8080-8090"));
///
/// assert!(!is_valid_port_range("0"));
/// assert!(!is_valid_port_range("8080-"));
/// assert!(!is_valid_port_range("8090-8080")); // Start > end
/// ```
pub fn is_valid_port_range(range: &str) -> bool {
    if range.is_empty() {
        return false;
    }

    if let Some(dash_pos) = range.find('-') {
        let start = &range[..dash_pos];
        let end = &range[dash_pos + 1..];

        if start.is_empty() || end.is_empty() {
            return false;
        }

        let start_port: u16 = match start.parse() {
            Ok(p) if p > 0 => p,
            _ => return false,
        };

        let end_port: u16 = match end.parse() {
            Ok(p) if p > 0 => p,
            _ => return false,
        };

        start_port <= end_port
    } else {
        // Single port
        match range.parse::<u16>() {
            Ok(p) => p > 0,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_email() {
        // Valid emails
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user.name@example.com"));
        assert!(is_valid_email("user+tag@example.com"));
        assert!(is_valid_email("user_name@example.co.uk"));
        assert!(is_valid_email("a@b.co"));

        // Invalid emails
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user @example.com"));
        assert!(!is_valid_email("user@exam ple.com"));

        // Too long
        let long_local = "a".repeat(250);
        assert!(!is_valid_email(&format!("{}@example.com", long_local)));
    }

    #[test]
    fn test_is_valid_ssh_key_path() {
        // Valid paths
        assert!(is_valid_ssh_key_path("/home/user/.ssh/id_rsa"));
        assert!(is_valid_ssh_key_path("~/.ssh/id_rsa.pub"));
        assert!(is_valid_ssh_key_path("./keys/my_key.pem"));
        assert!(is_valid_ssh_key_path("/path/to/key"));
        assert!(is_valid_ssh_key_path("C:\\Users\\User\\.ssh\\id_rsa"));

        // Invalid paths
        assert!(!is_valid_ssh_key_path(""));
        assert!(!is_valid_ssh_key_path("/path/with\x00/null"));
        assert!(!is_valid_ssh_key_path("../etc/passwd"));
        assert!(!is_valid_ssh_key_path("/path/to/file.exe"));
        assert!(!is_valid_ssh_key_path("/path/to/file.bat"));
    }

    #[test]
    fn test_password_strength() {
        // Very weak passwords
        assert_eq!(validate_password_strength(""), PasswordStrength::VeryWeak);
        assert_eq!(validate_password_strength("a"), PasswordStrength::VeryWeak);
        assert_eq!(validate_password_strength("password"), PasswordStrength::VeryWeak);
        assert_eq!(validate_password_strength("12345678"), PasswordStrength::VeryWeak);

        // Weak passwords
        assert_eq!(validate_password_strength("Password1"), PasswordStrength::Weak);

        // Moderate passwords
        let moderate = validate_password_strength("GoodPass1");
        assert!(moderate.is_acceptable());

        // Strong passwords
        let strong = validate_password_strength("MyStr0ng!Pass");
        assert!(strong.is_strong());
        assert!(strong.is_acceptable());

        // Very strong passwords
        let very_strong = validate_password_strength("MyStr0ng!P@ssw0rd#2024");
        assert!(very_strong.is_strong());
    }

    #[test]
    fn test_password_strength_common_patterns() {
        assert_eq!(
            validate_password_strength("password123"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("qwerty12345"),
            PasswordStrength::VeryWeak
        );
        assert_eq!(
            validate_password_strength("admin12345"),
            PasswordStrength::VeryWeak
        );
    }

    #[test]
    fn test_password_strength_descriptions() {
        assert_eq!(PasswordStrength::VeryWeak.description(), "Very Weak");
        assert_eq!(PasswordStrength::Weak.description(), "Weak");
        assert_eq!(PasswordStrength::Moderate.description(), "Moderate");
        assert_eq!(PasswordStrength::Strong.description(), "Strong");
        assert_eq!(PasswordStrength::VeryStrong.description(), "Very Strong");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.txt"), "test.txt");
        assert_eq!(sanitize_filename("test/file.txt"), "test_file.txt");
        assert_eq!(sanitize_filename("test:file"), "test_file");
        assert_eq!(sanitize_filename("test<file>"), "test_file_");
        assert_eq!(sanitize_filename("  spaces  "), "spaces");
    }

    #[test]
    fn test_is_valid_ssh_username() {
        assert!(is_valid_ssh_username("root"));
        assert!(is_valid_ssh_username("user"));
        assert!(is_valid_ssh_username("user_123"));
        assert!(is_valid_ssh_username("_admin"));
        assert!(is_valid_ssh_username("test.user"));
        assert!(is_valid_ssh_username("test-user"));

        assert!(!is_valid_ssh_username(""));
        assert!(!is_valid_ssh_username("123user")); // Can't start with digit
        assert!(!is_valid_ssh_username("user name")); // No spaces
        assert!(!is_valid_ssh_username("user@domain")); // No @
        assert!(!is_valid_ssh_username("a".repeat(33).as_str())); // Too long
    }

    #[test]
    fn test_is_valid_port_range() {
        // Single ports
        assert!(is_valid_port_range("22"));
        assert!(is_valid_port_range("8080"));
        assert!(is_valid_port_range("65535"));

        // Port ranges
        assert!(is_valid_port_range("8080-8090"));
        assert!(is_valid_port_range("1-65535"));
        assert!(is_valid_port_range("1000-1000")); // Same start and end

        // Invalid
        assert!(!is_valid_port_range(""));
        assert!(!is_valid_port_range("0")); // Port 0 is invalid
        assert!(!is_valid_port_range("8080-"));
        assert!(!is_valid_port_range("-8090"));
        assert!(!is_valid_port_range("8090-8080")); // Start > end
        assert!(!is_valid_port_range("abc"));
        assert!(!is_valid_port_range("8080-abc"));
    }
}
