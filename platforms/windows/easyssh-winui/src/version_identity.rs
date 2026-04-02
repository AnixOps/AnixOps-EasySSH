//! EasySSH Windows UI 版本标识系统
//!
//! 为Windows平台UI提供版本特定的主题和功能标识

use eframe::egui::{Color32, RichText, Style, Ui, Visuals};

/// 版本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    Lite,
    Standard,
    Pro,
}

impl Edition {
    /// 从特征标志确定当前版本
    pub const fn current() -> Self {
        #[cfg(feature = "pro")]
        return Edition::Pro;
        #[cfg(all(feature = "standard", not(feature = "pro")))]
        return Edition::Standard;
        #[cfg(not(any(feature = "standard", feature = "pro")))]
        return Edition::Lite;
    }

    pub fn name(&self) -> &'static str {
        match self {
            Edition::Lite => "Lite",
            Edition::Standard => "Standard",
            Edition::Pro => "Pro",
        }
    }

    pub fn full_name(&self) -> &'static str {
        match self {
            Edition::Lite => "EasySSH Lite",
            Edition::Standard => "EasySSH Standard",
            Edition::Pro => "EasySSH Pro",
        }
    }

    pub fn tagline(&self) -> &'static str {
        match self {
            Edition::Lite => "Secure SSH Config Manager",
            Edition::Standard => "Full-Featured SSH Client",
            Edition::Pro => "Enterprise Collaboration Platform",
        }
    }

    pub fn short_id(&self) -> &'static str {
        match self {
            Edition::Lite => "L",
            Edition::Standard => "S",
            Edition::Pro => "P",
        }
    }

    pub fn tier(&self) -> u8 {
        match self {
            Edition::Lite => 1,
            Edition::Standard => 2,
            Edition::Pro => 3,
        }
    }

    /// 主色调 - 返回 Color32
    pub fn primary_color(&self) -> Color32 {
        match self {
            Edition::Lite => Color32::from_hex("#10B981"),     // Emerald 500
            Edition::Standard => Color32::from_hex("#3B82F6"), // Blue 500
            Edition::Pro => Color32::from_hex("#8B5CF6"),     // Violet 500
        }
    }

    /// 次色调
    pub fn secondary_color(&self) -> Color32 {
        match self {
            Edition::Lite => Color32::from_hex("#34D399"),     // Emerald 400
            Edition::Standard => Color32::from_hex("#60A5FA"), // Blue 400
            Edition::Pro => Color32::from_hex("#A78BFA"),     // Violet 400
        }
    }

    /// 强调色
    pub fn accent_color(&self) -> Color32 {
        match self {
            Edition::Lite => Color32::from_hex("#059669"),     // Emerald 600
            Edition::Standard => Color32::from_hex("#2563EB"), // Blue 600
            Edition::Pro => Color32::from_hex("#7C3AED"),       // Violet 600
        }
    }

    pub fn supported_features(&self) -> Vec<&'static str> {
        let mut features = vec!["ssh", "keychain", "native-terminal"];

        if self.has_embedded_terminal() {
            features.push("embedded-terminal");
        }
        if self.has_split_screen() {
            features.push("split-screen");
        }
        if self.has_sftp() {
            features.push("sftp");
        }
        if self.has_monitoring() {
            features.push("monitoring");
        }
        if self.has_team() {
            features.push("team");
        }
        if self.has_audit() {
            features.push("audit");
        }
        if self.has_sso() {
            features.push("sso");
        }

        features
    }

    pub fn has_embedded_terminal(&self) -> bool {
        #[cfg(feature = "embedded-terminal")]
        return true;
        #[cfg(not(feature = "embedded-terminal"))]
        return false;
    }

    pub fn has_split_screen(&self) -> bool {
        #[cfg(feature = "split-screen")]
        return true;
        #[cfg(not(feature = "split-screen"))]
        return false;
    }

    pub fn has_sftp(&self) -> bool {
        #[cfg(feature = "sftp")]
        return true;
        #[cfg(not(feature = "sftp"))]
        return false;
    }

    pub fn has_monitoring(&self) -> bool {
        #[cfg(feature = "monitoring")]
        return true;
        #[cfg(not(feature = "monitoring"))]
        return false;
    }

    pub fn has_team(&self) -> bool {
        #[cfg(feature = "team")]
        return true;
        #[cfg(not(feature = "team"))]
        return false;
    }

    pub fn has_audit(&self) -> bool {
        #[cfg(feature = "audit")]
        return true;
        #[cfg(not(feature = "audit"))]
        return false;
    }

    pub fn has_sso(&self) -> bool {
        #[cfg(feature = "sso")]
        return true;
        #[cfg(not(feature = "sso"))]
        return false;
    }

    /// 渲染版本徽章
    pub fn render_badge(&self, ui: &mut Ui) {
        let (bg, fg) = match self {
            Edition::Lite => (self.primary_color(), Color32::WHITE),
            Edition::Standard => (self.primary_color(), Color32::WHITE),
            Edition::Pro => (self.primary_color(), Color32::WHITE),
        };

        ui.colored_label(
            bg,
            RichText::new(self.short_id())
                .strong()
                .color(fg)
                .size(12.0),
        );
    }
}

/// 构建类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildType {
    Release,
    Dev,
}

impl BuildType {
    pub const fn current() -> Self {
        #[cfg(debug_assertions)]
        return BuildType::Dev;
        #[cfg(not(debug_assertions))]
        return BuildType::Release;
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuildType::Release => "Release",
            BuildType::Dev => "Dev",
        }
    }

    pub fn show_dev_tools(&self) -> bool {
        matches!(self, BuildType::Dev)
    }
}

/// 版本标识信息
#[derive(Debug, Clone)]
pub struct VersionIdentity {
    edition: Edition,
    build_type: BuildType,
}

impl VersionIdentity {
    pub fn new() -> Self {
        Self {
            edition: Edition::current(),
            build_type: BuildType::current(),
        }
    }

    pub fn edition(&self) -> Edition {
        self.edition
    }

    pub fn edition_name(&self) -> &'static str {
        self.edition.name()
    }

    pub fn edition_tier(&self) -> u8 {
        self.edition.tier()
    }

    pub fn build_type(&self) -> BuildType {
        self.build_type
    }

    pub fn build_type_name(&self) -> &'static str {
        self.build_type.name()
    }

    pub fn full_name(&self) -> &'static str {
        self.edition.full_name()
    }

    pub fn tagline(&self) -> &'static str {
        self.edition.tagline()
    }

    pub fn window_title(&self) -> String {
        match self.build_type {
            BuildType::Dev => format!("{} {} [{}]", self.full_name(), self.version(), self.build_type_name()),
            BuildType::Release => format!("{} {}", self.full_name(), self.version()),
        }
    }

    pub fn short_version(&self) -> String {
        format!("{} {} {}", self.edition.short_id(), self.version(), self.build_type_name())
    }

    pub fn full_version_string(&self) -> String {
        let build_info = format!("{}", self.build_type_name());
        format!(
            "{} {} ({})",
            self.full_name(),
            self.version(),
            build_info
        )
    }

    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    pub fn primary_color(&self) -> Color32 {
        self.edition.primary_color()
    }

    pub fn secondary_color(&self) -> Color32 {
        self.edition.secondary_color()
    }

    pub fn accent_color(&self) -> Color32 {
        self.edition.accent_color()
    }

    pub fn features(&self) -> Vec<&'static str> {
        self.edition.supported_features()
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.edition.supported_features().contains(&feature)
    }

    /// 渲染关于对话框内容
    pub fn render_about(&self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(self.full_name());
            ui.label(format!("Version {}", self.version()));
            ui.label(self.tagline());

            if self.build_type == BuildType::Dev {
                ui.add_space(8.0);
                ui.colored_label(
                    Color32::YELLOW,
                    format!("Build Type: {} Mode", self.build_type_name()),
                );
            }

            ui.add_space(16.0);

            // 版本徽章
            ui.horizontal(|ui| {
                ui.label("Edition:");
                self.edition.render_badge(ui);
            });

            ui.add_space(8.0);

            // 功能列表
            ui.label("Features:");
            ui.horizontal_wrapped(|ui| {
                for feature in self.features() {
                    ui.colored_label(
                        self.secondary_color(),
                        format!("{}", feature),
                    );
                    ui.label(" ");
                }
            });
        });
    }
}

impl Default for VersionIdentity {
    fn default() -> Self {
        Self::new()
    }
}

/// 版本感知主题
pub struct VersionAwareTheme {
    primary: Color32,
    secondary: Color32,
    accent: Color32,
    edition: Edition,
}

impl VersionAwareTheme {
    pub fn from_identity(identity: &VersionIdentity) -> Self {
        Self {
            primary: identity.primary_color(),
            secondary: identity.secondary_color(),
            accent: identity.accent_color(),
            edition: identity.edition,
        }
    }

    /// 应用版本颜色到 DesignTheme
    pub fn apply_to_design_theme(&self, theme: &mut crate::design::DesignTheme) {
        // 应用版本特定的强调色到 DesignTheme
        // 这会覆盖 DesignTheme 的默认强调色
        theme.accent_color = self.primary;
    }

    /// 应用版本主题到 egui 样式
    pub fn apply_to_egui_style(&self, style: &mut Style) {
        // 修改视觉样式以匹配版本主题
        style.visuals.selection.bg_fill = self.primary;
        style.visuals.hyperlink_color = self.secondary;
        style.visuals.widgets.active.bg_fill = self.accent;
    }

    /// 创建版本特定的 Visuals
    pub fn create_visuals(&self) -> Visuals {
        let mut visuals = Visuals::dark();
        visuals.selection.bg_fill = self.primary;
        visuals.hyperlink_color = self.secondary;
        visuals.widgets.active.bg_fill = self.accent;
        visuals
    }

    /// 渲染版本指示器（用于标题栏或状态栏）
    pub fn render_indicator(&self, ui: &mut Ui) {
        let text = format!("{}", self.edition.short_id());

        ui.allocate_ui_with_layout(
            egui::vec2(24.0, 24.0),
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.painter().rect_filled(
                    ui.available_rect_before_wrap(),
                    4.0,
                    self.primary,
                );
                ui.colored_label(Color32::WHITE, text);
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edition_current() {
        let edition = Edition::current();
        // 根据当前编译特性，可能是任何版本
        assert!(
            matches!(edition, Edition::Lite | Edition::Standard | Edition::Pro)
        );
    }

    #[test]
    fn test_edition_colors() {
        let lite = Edition::Lite;
        let standard = Edition::Standard;
        let pro = Edition::Pro;

        // 确保每个版本都有有效的颜色
        assert_ne!(lite.primary_color(), Color32::BLACK);
        assert_ne!(standard.primary_color(), Color32::BLACK);
        assert_ne!(pro.primary_color(), Color32::BLACK);
    }

    #[test]
    fn test_version_identity() {
        let identity = VersionIdentity::new();
        assert!(!identity.full_name().is_empty());
        assert!(!identity.version().is_empty());
        assert!(!identity.features().is_empty());
    }

    #[test]
    fn test_version_comparison() {
        assert!(Edition::Standard.tier() > Edition::Lite.tier());
        assert!(Edition::Pro.tier() > Edition::Standard.tier());
    }
}
