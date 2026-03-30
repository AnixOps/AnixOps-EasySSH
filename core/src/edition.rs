//! 版本管理模块
//! 提供编译时版本信息和功能可用性检查

/// 版本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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
