//! Windows UI版本标识集成示例
//!
//! 本模块展示如何在Windows egui应用中集成版本显示

use eframe::egui;
use easyssh_core::version::{Edition, FullVersionInfo, BuildType};

/// Windows应用版本显示组件
pub struct VersionDisplay {
    full_info: &'static FullVersionInfo,
    show_detailed: bool,
}

impl VersionDisplay {
    pub fn new() -> Self {
        Self {
            full_info: FullVersionInfo::current(),
            show_detailed: false,
        }
    }

    /// 在标题栏显示版本徽章
    pub fn render_title_bar_version(&self, ui: &mut egui::Ui) {
        let (text, color) = match self.full_info.edition {
            Edition::Lite => ("Lite", egui::Color32::from_rgb(0, 150, 136)), // Teal
            Edition::Standard => ("Standard", egui::Color32::from_rgb(33, 150, 243)), // Blue
            Edition::Pro => ("Pro", egui::Color32::from_rgb(156, 39, 176)), // Purple
        };

        let dev_marker = if self.full_info.build_type == BuildType::Dev {
            " [Dev]"
        } else {
            ""
        };

        ui.horizontal(|ui| {
            // 版本徽章
            ui.colored_label(
                color,
                egui::RichText::new(format!("{}{}", text, dev_marker))
                    .strong()
                    .monospace(),
            );
        });
    }

    /// 渲染关于对话框
    pub fn render_about_dialog(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new("关于 EasySSH")
            .collapsible(false)
            .resizable(false)
            .min_width(400.0)
            .open(open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // 应用图标/Logo区域
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("EasySSH")
                            .size(32.0)
                            .strong(),
                    );

                    // 版本信息
                    ui.add_space(10.0);
                    self.render_version_badge(ui);

                    ui.add_space(20.0);
                    ui.separator();

                    // 详细信息开关
                    ui.checkbox(&mut self.show_detailed, "显示详细信息");

                    if self.show_detailed {
                        ui.add_space(10.0);
                        self.render_detailed_info(ui);
                    }

                    ui.add_space(20.0);
                    ui.separator();

                    // 版权信息
                    ui.label("© 2024 EasySSH Team. All rights reserved.");
                    ui.hyperlink_to("访问官网", "https://easyssh.dev");
                });
            });
    }

    /// 渲染版本徽章
    fn render_version_badge(&self, ui: &mut egui::Ui) {
        let (bg_color, text_color) = match self.full_info.edition {
            Edition::Lite => (
                egui::Color32::from_rgb(224, 242, 241), // Light teal
                egui::Color32::from_rgb(0, 105, 92),    // Dark teal
            ),
            Edition::Standard => (
                egui::Color32::from_rgb(227, 242, 253), // Light blue
                egui::Color32::from_rgb(13, 71, 161),  // Dark blue
            ),
            Edition::Pro => (
                egui::Color32::from_rgb(243, 229, 245), // Light purple
                egui::Color32::from_rgb(106, 27, 154),  // Dark purple
            ),
        };

        let edition_text = match self.full_info.edition {
            Edition::Lite => "Lite Edition",
            Edition::Standard => "Standard Edition",
            Edition::Pro => "Pro Edition",
        };

        // 绘制徽章背景
        let padding = egui::vec2(16.0, 8.0);
        let text = egui::RichText::new(edition_text)
            .color(text_color)
            .strong()
            .size(14.0);

        let (rect, _) = ui.allocate_space(ui.spacing().interact_size + padding * 2.0);

        ui.painter().rect_filled(
            rect,
            6.0, // 圆角
            bg_color,
        );

        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            edition_text,
            egui::FontId::proportional(14.0),
            text_color,
        );

        ui.add_space(10.0);

        // 版本号
        ui.label(
            egui::RichText::new(format!("版本 {}", self.full_info.version))
                .size(16.0)
                .monospace(),
        );

        // 构建类型标记
        if self.full_info.build_type == BuildType::Dev {
            ui.colored_label(
                egui::Color32::YELLOW,
                egui::RichText::new("开发版本")
                    .italics()
                    .size(12.0),
            );
        }
    }

    /// 渲染详细信息
    fn render_detailed_info(&self, ui: &mut egui::Ui) {
        egui::Grid::new("version_details")
            .num_columns(2)
            .spacing([10.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                // 版本
                ui.label("版本:");
                ui.label(&self.full_info.version);
                ui.end_row();

                // 版本类型
                ui.label("版本类型:");
                ui.label(self.full_info.edition.name());
                ui.end_row();

                // 构建日期
                ui.label("构建日期:");
                ui.label(&self.full_info.build_date);
                ui.end_row();

                // Git信息
                if let Some(ref hash) = self.full_info.git_hash {
                    ui.label("Git Commit:");
                    let short_hash = &hash[..8.min(hash.len())];
                    if let Some(ref branch) = self.full_info.git_branch {
                        ui.label(format!("{} ({})", short_hash, branch));
                    } else {
                        ui.label(short_hash);
                    }
                    ui.end_row();
                }

                // 平台
                ui.label("平台:");
                ui.label(self.full_info.platform.display());
                ui.end_row();

                // Rust版本
                if let Some(ref rustc) = self.full_info.rustc_version {
                    ui.label("Rustc:");
                    ui.label(rustc);
                    ui.end_row();
                }

                // 功能特性
                ui.label("启用的功能:");
                let features_text = self.full_info.features.join(", ");
                ui.label(features_text);
                ui.end_row();
            });
    }

    /// 在状态栏显示精简版本信息
    pub fn render_status_bar_version(&self, ui: &mut egui::Ui) {
        let version_text = if self.full_info.build_type == BuildType::Dev {
            format!("{} {} [Dev]", self.full_info.edition.code(), self.full_info.version)
        } else {
            format!("{} {}", self.full_info.edition.code(), self.full_info.version)
        };

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(version_text);
        });
    }
}

/// 在启动画面显示版本
pub fn render_splash_version(ui: &mut egui::Ui) {
    let info = FullVersionInfo::current();

    ui.vertical_centered(|ui| {
        ui.add_space(100.0);

        // Logo
        ui.label(
            egui::RichText::new("EasySSH")
                .size(48.0)
                .strong(),
        );

        ui.add_space(20.0);

        // 版本横幅
        let banner_color = match info.edition {
            Edition::Lite => egui::Color32::from_rgb(0, 150, 136),
            Edition::Standard => egui::Color32::from_rgb(33, 150, 243),
            Edition::Pro => egui::Color32::from_rgb(156, 39, 176),
        };

        let edition_name = info.edition.name();
        ui.colored_label(
            banner_color,
            egui::RichText::new(format!("{} Edition", edition_name))
                .size(20.0)
                .strong(),
        );

        ui.add_space(10.0);

        // 版本号
        ui.monospace(format!("版本 {}", info.version));

        // 开发标记
        if info.build_type == BuildType::Dev {
            ui.add_space(5.0);
            ui.colored_label(
                egui::Color32::YELLOW,
                egui::RichText::new("⚠ 开发版本").size(14.0),
            );
        }

        ui.add_space(30.0);

        // 加载提示
        ui.label("正在启动...");
    });
}

/// 版本升级提示组件
pub struct EditionUpgradePrompt {
    target_edition: Edition,
}

impl EditionUpgradePrompt {
    pub fn new(target: Edition) -> Self {
        Self {
            target_edition: target,
        }
    }

    pub fn render(&self, ctx: &egui::Context, open: &mut bool) {
        let current = Edition::current();

        if current.tier() >= self.target_edition.tier() {
            // 已经满足要求
            return;
        }

        egui::Window::new("功能需要升级")
            .collapsible(false)
            .resizable(false)
            .open(open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "此功能需要 {} 版本，您当前使用的是 {} 版本。",
                    self.target_edition.name(),
                    current.name()
                ));

                ui.add_space(10.0);

                match self.target_edition {
                    Edition::Standard => {
                        ui.label("升级到 Standard 版本，您将获得:");
                        ui.bullet("嵌入式终端");
                        ui.bullet("分屏功能");
                        ui.bullet("SFTP文件传输");
                        ui.bullet("服务器监控");
                    }
                    Edition::Pro => {
                        ui.label("升级到 Pro 版本，您将获得:");
                        ui.bullet("团队协作");
                        ui.bullet("审计日志");
                        ui.bullet("SSO集成");
                        ui.bullet("高级安全功能");
                    }
                    _ => {}
                }

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button("了解更多").clicked() {
                        // 打开升级页面
                    }
                    if ui.button("暂不升级").clicked() {
                        *open = false;
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_display_creation() {
        let display = VersionDisplay::new();
        assert!(!display.full_info.version.is_empty());
    }
}
