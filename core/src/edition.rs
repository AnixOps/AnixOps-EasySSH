//! 版本管理模块
//! 提供编译时版本信息和功能可用性检查

/// 版本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edition {
    Lite,
    Standard,
    Pro,
}

impl Edition {
    /// 获取当前编译版本
    pub const fn current() -> Self {
        #[cfg(feature = "pro")]
        return Edition::Pro;
        #[cfg(all(feature = "standard", not(feature = "pro")))]
        return Edition::Standard;
        #[cfg(not(any(feature = "standard", feature = "pro")))]
        return Edition::Lite;
    }

    /// 版本名称
    pub const fn name(&self) -> &'static str {
        match self {
            Edition::Lite => "Lite",
            Edition::Standard => "Standard",
            Edition::Pro => "Pro",
        }
    }

    /// 是否支持嵌入式终端
    pub const fn has_embedded_terminal(&self) -> bool {
        #[cfg(feature = "embedded-terminal")]
        return true;
        #[cfg(not(feature = "embedded-terminal"))]
        return false;
    }

    /// 是否支持分屏
    pub const fn has_split_screen(&self) -> bool {
        #[cfg(feature = "split-screen")]
        return true;
        #[cfg(not(feature = "split-screen"))]
        return false;
    }

    /// 是否支持SFTP
    pub const fn has_sftp(&self) -> bool {
        #[cfg(feature = "sftp")]
        return true;
        #[cfg(not(feature = "sftp"))]
        return false;
    }

    /// 是否支持监控
    pub const fn has_monitoring(&self) -> bool {
        #[cfg(feature = "monitoring")]
        return true;
        #[cfg(not(feature = "monitoring"))]
        return false;
    }

    /// 是否支持团队功能
    pub const fn has_team(&self) -> bool {
        #[cfg(feature = "team")]
        return true;
        #[cfg(not(feature = "team"))]
        return false;
    }

    /// 是否支持审计
    pub const fn has_audit(&self) -> bool {
        #[cfg(feature = "audit")]
        return true;
        #[cfg(not(feature = "audit"))]
        return false;
    }

    /// 是否支持SSO
    pub const fn has_sso(&self) -> bool {
        #[cfg(feature = "sso")]
        return true;
        #[cfg(not(feature = "sso"))]
        return false;
    }
}

/// 版本信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct VersionInfo {
    pub edition: Edition,
    pub edition_name: &'static str,
    pub version: &'static str,
    pub features: Vec<&'static str>,
}

impl VersionInfo {
    pub fn current() -> Self {
        let edition = Edition::current();
        #[allow(unused_mut)]
        let mut features = vec!["ssh", "keychain", "native-terminal"];

        #[cfg(feature = "embedded-terminal")]
        features.push("embedded-terminal");
        #[cfg(feature = "split-screen")]
        features.push("split-screen");
        #[cfg(feature = "sftp")]
        features.push("sftp");
        #[cfg(feature = "monitoring")]
        features.push("monitoring");
        #[cfg(feature = "team")]
        features.push("team");
        #[cfg(feature = "audit")]
        features.push("audit");
        #[cfg(feature = "sso")]
        features.push("sso");

        VersionInfo {
            edition,
            edition_name: edition.name(),
            version: env!("CARGO_PKG_VERSION"),
            features,
        }
    }
}

/// 检查功能是否可用 - 用于编译时优化
#[macro_export]
macro_rules! check_feature {
    (embedded_terminal) => {
        cfg!(feature = "embedded-terminal")
    };
    (split_screen) => {
        cfg!(feature = "split-screen")
    };
    (sftp) => {
        cfg!(feature = "sftp")
    };
    (monitoring) => {
        cfg!(feature = "monitoring")
    };
    (team) => {
        cfg!(feature = "team")
    };
    (audit) => {
        cfg!(feature = "audit")
    };
    (sso) => {
        cfg!(feature = "sso")
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edition_variants() {
        let lite = Edition::Lite;
        let standard = Edition::Standard;
        let pro = Edition::Pro;

        assert_eq!(lite.name(), "Lite");
        assert_eq!(standard.name(), "Standard");
        assert_eq!(pro.name(), "Pro");
    }

    #[test]
    fn test_edition_equality() {
        assert_eq!(Edition::Lite, Edition::Lite);
        assert_eq!(Edition::Pro, Edition::Pro);
        assert_ne!(Edition::Lite, Edition::Standard);
        assert_ne!(Edition::Standard, Edition::Pro);
    }

    #[test]
    fn test_edition_clone() {
        let edition = Edition::Pro;
        let cloned = edition.clone();
        assert_eq!(edition, cloned);
    }

    #[test]
    fn test_edition_copy() {
        let edition = Edition::Standard;
        let copied = edition;
        assert_eq!(edition, copied); // Edition is Copy, so both valid
    }

    #[test]
    fn test_edition_current() {
        let current = Edition::current();
        // Current edition depends on compile features
        // Just verify it returns a valid variant
        assert!(
            matches!(current, Edition::Lite | Edition::Standard | Edition::Pro)
        );
    }

    #[test]
    fn test_edition_serialize() {
        let lite = Edition::Lite;
        let json = serde_json::to_string(&lite).expect("Failed to serialize");
        assert_eq!(json, "\"lite\"");

        let standard = Edition::Standard;
        let json = serde_json::to_string(&standard).expect("Failed to serialize");
        assert_eq!(json, "\"standard\"");

        let pro = Edition::Pro;
        let json = serde_json::to_string(&pro).expect("Failed to serialize");
        assert_eq!(json, "\"pro\"");
    }

    #[test]
    fn test_edition_deserialize() {
        let lite: Edition = serde_json::from_str("\"lite\"").expect("Failed to deserialize");
        assert_eq!(lite, Edition::Lite);

        let standard: Edition =
            serde_json::from_str("\"standard\"").expect("Failed to deserialize");
        assert_eq!(standard, Edition::Standard);

        let pro: Edition = serde_json::from_str("\"pro\"").expect("Failed to deserialize");
        assert_eq!(pro, Edition::Pro);
    }

    #[test]
    fn test_version_info_current() {
        let info = VersionInfo::current();

        // Verify all fields are populated
        assert!(!info.version.is_empty());
        assert!(!info.edition_name.is_empty());
        assert!(!info.features.is_empty());

        // Verify features always include base features
        assert!(info.features.contains(&"ssh"));
        assert!(info.features.contains(&"keychain"));
        assert!(info.features.contains(&"native-terminal"));

        // Verify edition matches edition_name
        assert_eq!(info.edition.name(), info.edition_name);
    }

    #[test]
    fn test_version_info_serialize() {
        let info = VersionInfo::current();
        let json = serde_json::to_string(&info).expect("Failed to serialize");

        assert!(json.contains("edition"));
        assert!(json.contains("version"));
        assert!(json.contains("features"));
    }

    #[test]
    fn test_version_info_clone() {
        let info = VersionInfo::current();
        let cloned = info.clone();

        assert_eq!(info.version, cloned.version);
        assert_eq!(info.edition, cloned.edition);
        assert_eq!(info.features.len(), cloned.features.len());
    }

    #[test]
    fn test_feature_checks_consistency() {
        let edition = Edition::current();

        // The feature checks should be consistent with the edition
        // These are compile-time checks, so we're verifying they don't panic
        let _ = edition.has_embedded_terminal();
        let _ = edition.has_split_screen();
        let _ = edition.has_sftp();
        let _ = edition.has_monitoring();
        let _ = edition.has_team();
        let _ = edition.has_audit();
        let _ = edition.has_sso();
    }

    #[test]
    fn test_check_feature_macro() {
        // Verify the macro expands correctly
        let embedded = check_feature!(embedded_terminal);
        let sftp = check_feature!(sftp);
        let split = check_feature!(split_screen);

        // Just verify they compile and return booleans
        assert!(embedded || !embedded);
        assert!(sftp || !sftp);
        assert!(split || !split);
    }

    #[test]
    fn test_edition_debug() {
        let edition = Edition::Pro;
        let debug = format!("{:?}", edition);
        assert!(debug.contains("Pro"));
    }

    #[test]
    fn test_version_info_debug() {
        let info = VersionInfo::current();
        let debug = format!("{:?}", info);
        assert!(!debug.is_empty());
        assert!(debug.contains("VersionInfo"));
    }

    #[test]
    fn test_base_features_always_present() {
        // Base features should always be present regardless of edition
        let info = VersionInfo::current();

        assert!(
            info.features.contains(&"ssh"),
            "ssh should always be present"
        );
        assert!(
            info.features.contains(&"keychain"),
            "keychain should always be present"
        );
        assert!(
            info.features.contains(&"native-terminal"),
            "native-terminal should always be present"
        );
    }
}
