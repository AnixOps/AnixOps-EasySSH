//! Linux GTK4版本标识集成示例
//!
//! 本模块展示如何在Linux GTK4应用中集成版本显示

use easyssh_core::edition::{BuildType, Edition};
use easyssh_core::version::FullBuildInfo;
use gtk4::prelude::*;
use gtk4::{AboutDialog, Box as GtkBox, Label, Orientation, Picture, ScrolledWindow, Window};

/// GTK4版本信息对话框
pub struct VersionInfoDialog;

impl VersionInfoDialog {
    /// 创建并显示关于对话框
    pub fn show(parent: &impl IsA<Window>) {
        let info = FullBuildInfo::current();

        let dialog = AboutDialog::builder()
            .program_name("EasySSH")
            .version(&info.version_info.version)
            .comments(info.version_info.edition.tagline())
            .website("https://easyssh.dev")
            .website_label("访问官网")
            .copyright("© 2024 EasySSH Team")
            .license_type(gtk4::License::MitX11)
            .transient_for(parent)
            .modal(true)
            .build();

        // 设置版本特定的样式类
        let style_class = match info.version_info.edition {
            Edition::Lite => "edition-lite",
            Edition::Standard => "edition-standard",
            Edition::Pro => "edition-pro",
        };
        dialog.add_css_class(style_class);

        // 添加构建信息
        let build_info = Self::build_info_string(&info);
        dialog.set_system_information(&build_info);

        dialog.present();
    }

    /// 构建详细信息字符串
    fn build_info_string(info: &FullBuildInfo) -> String {
        let mut lines = vec![
            format!("版本类型: {}", info.version_info.edition.name()),
            format!("版本号: {}", info.version_info.version),
            format!("构建日期: {}", info.build_date),
            format!("平台: {}", info.platform.display()),
        ];

        if let Some(ref hash) = info.version_info.git_hash {
            let branch_info = info
                .git_branch
                .as_ref()
                .map(|b| format!(" ({})", b))
                .unwrap_or_default();
            lines.push(format!(
                "Git: {}{}",
                &hash[..8.min(hash.len())],
                branch_info
            ));
        }

        if let Some(ref rustc) = info.rustc_version {
            lines.push(format!("编译器: {}", rustc));
        }

        if info.version_info.build_type == BuildType::Dev {
            lines.push(String::from("构建类型: 开发版本"));
        }

        lines.push(format!("启用的功能: {}", info.version_info.features.join(", ")));

        lines.join("\n")
    }

    /// 创建自定义版本详情对话框（更详细的版本信息）
    pub fn show_detailed(parent: &impl IsA<Window>) {
        let info = FullBuildInfo::current();

        let window = Window::builder()
            .title("版本信息")
            .transient_for(parent)
            .modal(true)
            .default_width(500)
            .default_height(400)
            .build();

        let vbox = GtkBox::new(Orientation::Vertical, 12);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);

        // 标题
        let title = Label::new(Some("EasySSH"));
        title.add_css_class("title-1");
        vbox.append(&title);

        // 版本徽章
        let edition_label = Self::create_edition_badge(&info);
        vbox.append(&edition_label);

        // 版本号
        let version_label = Label::new(Some(&format!("版本 {}", info.version_info.version)));
        version_label.add_css_class("monospace");
        vbox.append(&version_label);

        // 开发标记
        if info.version_info.build_type == BuildType::Dev {
            let dev_label = Label::new(Some("⚠ 开发版本"));
            dev_label.add_css_class("warning");
            vbox.append(&dev_label);
        }

        // 详细信息区域
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .has_frame(true)
            .height_request(200)
            .build();

        let details_box = GtkBox::new(Orientation::Vertical, 6);
        details_box.set_margin_top(6);
        details_box.set_margin_bottom(6);
        details_box.set_margin_start(6);
        details_box.set_margin_end(6);

        // 详细信息网格
        Self::add_detail_row(&details_box, "版本类型:", info.version_info.edition.name());
        Self::add_detail_row(&details_box, "构建日期:", &info.build_date);

        if let Some(ref hash) = info.version_info.git_hash {
            let branch_info = info
                .git_branch
                .as_ref()
                .map(|b| format!(" ({})", b))
                .unwrap_or_default();
            Self::add_detail_row(
                &details_box,
                "Git Commit:",
                &format!("{}{}", &hash[..8.min(hash.len())], branch_info),
            );
        }

        Self::add_detail_row(&details_box, "操作系统:", &info.platform.os);
        Self::add_detail_row(&details_box, "架构:", &info.platform.arch);

        if let Some(ref rustc) = info.rustc_version {
            Self::add_detail_row(&details_box, "编译器:", rustc);
        }

        if info.version_info.build_type == BuildType::Dev {
            Self::add_detail_row(&details_box, "构建类型:", "开发版本");
        }

        scrolled.set_child(Some(&details_box));
        vbox.append(&scrolled);

        // 功能列表
        let features_label = Label::new(Some(&format!("启用的功能: {}", info.version_info.features.join(", "))));
        features_label.set_wrap(true);
        features_label.set_max_width_chars(60);
        vbox.append(&features_label);

        window.set_child(Some(&vbox));
        window.present();
    }

    /// 创建版本徽章标签
    fn create_edition_badge(info: &FullBuildInfo) -> Label {
        let (bg_color, text_color) = match info.version_info.edition {
            Edition::Lite => ("#E0F2F1", "#00695C"),
            Edition::Standard => ("#E3F2FD", "#0D47A1"),
            Edition::Pro => ("#F3E5F5", "#6A1B9A"),
        };

        let label = Label::new(Some(&format!("{} Edition", info.version_info.edition.name())));
        label.set_markup(&format!(
            "<span background=\"{}\" foreground=\"{}\" font_weight=\"bold\"> {} Edition </span>",
            bg_color,
            text_color,
            info.version_info.edition.name()
        ));

        label
    }

    /// 添加详细信息行
    fn add_detail_row(container: &impl IsA<gtk4::Widget>, label: &str, value: &str) {
        let row = GtkBox::new(Orientation::Horizontal, 8);

        let label_widget = Label::new(Some(label));
        label_widget.set_xalign(1.0);
        label_widget.set_hexpand(false);
        label_widget.add_css_class("dim-label");
        row.append(&label_widget);

        let value_widget = Label::new(Some(value));
        value_widget.set_xalign(0.0);
        value_widget.set_hexpand(true);
        value_widget.add_css_class("monospace");
        row.append(&value_widget);

        if let Some(box_) = container.downcast_ref::<GtkBox>() {
            box_.append(&row);
        }
    }
}

/// 标题栏版本徽章
pub struct HeaderBarVersionBadge;

impl HeaderBarVersionBadge {
    /// 创建标题栏版本标签
    pub fn create() -> Label {
        let info = FullBuildInfo::current();

        let text = if info.version_info.build_type == BuildType::Dev {
            format!("{} [Dev]", info.version_info.edition.short_identifier())
        } else {
            info.version_info.edition.short_identifier().to_string()
        };

        let label = Label::new(Some(&text));
        label.add_css_class("version-badge");

        // 添加CSS类用于样式
        let edition_class = match info.version_info.edition {
            Edition::Lite => "edition-lite",
            Edition::Standard => "edition-standard",
            Edition::Pro => "edition-pro",
        };
        label.add_css_class(edition_class);

        label
    }
}

/// CSS样式定义（用于GTK4）
pub const VERSION_CSS: &str = r#"
.version-badge {
    font-size: 10pt;
    font-weight: bold;
    padding: 2px 8px;
    border-radius: 4px;
    margin-left: 8px;
}

.edition-lite {
    background-color: #E0F2F1;
    color: #00695C;
}

.edition-standard {
    background-color: #E3F2FD;
    color: #0D47A1;
}

.edition-pro {
    background-color: #F3E5F5;
    color: #6A1B9A;
}

.warning {
    color: #F57C00;
    font-weight: bold;
}

.monospace {
    font-family: monospace;
}
"#;

/// 版本检查器 - 用于在功能使用前检查版本
pub struct EditionChecker;

impl EditionChecker {
    /// 检查是否满足最低版本要求
    pub fn check_requirement(required: Edition) -> Result<(), String> {
        let current = Edition::current();

        // Use tier comparison instead of meets_requirement
        if current.tier() >= required.tier() {
            Ok(())
        } else {
            Err(format!(
                "此功能需要 {} 版本或更高版本，您当前使用的是 {} 版本。",
                required.name(),
                current.name()
            ))
        }
    }

    /// 显示版本不足提示
    pub fn show_upgrade_dialog(parent: &impl IsA<Window>, required: Edition) {
        let current = Edition::current();

        // Use tier comparison instead of meets_requirement
        if current.tier() >= required.tier() {
            return;
        }

        let dialog = gtk4::MessageDialog::new(
            Some(parent),
            gtk4::DialogFlags::MODAL,
            gtk4::MessageType::Info,
            gtk4::ButtonsType::Ok,
            &format!("功能需要 {} 版本", required.name()),
        );

        dialog.set_secondary_text(Some(&format!(
            "您当前使用的是 {} 版本。请升级到 {} 版本以使用此功能。",
            current.name(),
            required.name()
        )));

        dialog.connect_response(|dialog, _| {
            dialog.close();
        });

        dialog.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info_string() {
        let info = FullBuildInfo::current();
        let build_info = VersionInfoDialog::build_info_string(&info);

        assert!(build_info.contains(&info.version_info.version));
        assert!(build_info.contains(info.version_info.edition.name()));
    }

    #[test]
    fn test_edition_checker() {
        // Lite总是满足Lite要求
        assert!(EditionChecker::check_requirement(Edition::Lite).is_ok());

        // 当前版本应该满足自身要求
        assert!(EditionChecker::check_requirement(Edition::current()).is_ok());
    }
}

fn main() {
    // This is an example library showing how to integrate version display
    // in a GTK4 application. It's not meant to be run as a standalone
    // executable but rather used as a reference for integration.
    println!("EasySSH GTK4 Version Integration Example");
    println!("==========================================");
    println!();
    println!("This example demonstrates how to integrate version display");
    println!("in a GTK4 application.");
    println!();
    println!("To use this code, integrate the VersionInfoDialog struct into");
    println!("your GTK4 application's UI code.");
    println!();

    let info = FullBuildInfo::current();
    println!("Current version: {} {}", info.version_info.edition.name(), info.version_info.version);
    println!("Build date: {}", info.build_date);
    println!("Platform: {}", info.platform.display());
}
