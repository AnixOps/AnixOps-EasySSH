//! TUI版本标识集成示例
//!
//! 本模块展示如何在TUI应用中集成版本显示

use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, ExecutableCommand};
use easyssh_core::edition::{BuildType, Edition};
use easyssh_core::version::FullBuildInfo;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    println!("EasySSH TUI Version Integration Demo");
    println!("=====================================\n");

    // Display splash banner
    render_splash_banner()?;

    println!("\nPress Enter to continue to about dialog...");
    let _ = std::io::stdin().read_line(&mut String::new());

    // Display about dialog
    render_about_dialog()?;

    println!("\nDemo completed!");
    Ok(())
}

/// 渲染启动横幅
pub fn render_splash_banner() -> io::Result<()> {
    let info = FullBuildInfo::current();
    let mut stdout = io::stdout();

    // 清屏
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;

    // ASCII艺术Logo
    let logo = r#"
    ███████╗ █████╗ ███████╗██╗   ██╗███████╗███████╗██╗  ██╗
    ██╔════╝██╔══██╗██╔════╝╚██╗ ██╔╝██╔════╝██╔════╝██║  ██║
    █████╗  ███████║███████╗ ╚████╔╝ ███████╗███████╗███████║
    ██╔══╝  ██╔══██║╚════██║  ╚██╔╝  ╚════██║╚════██║██╔══██║
    ███████╗██║  ██║███████║   ██║   ███████║███████║██║  ██║
    ╚══════╝╚═╝  ╚═╝╚══════╝   ╚═╝   ╚══════╝╚══════╝╚═╝  ╚═╝
    "#;

    // 选择颜色
    let color = match info.version_info.edition {
        Edition::Lite => Color::Cyan,
        Edition::Standard => Color::Blue,
        Edition::Pro => Color::Magenta,
    };

    // 打印Logo
    stdout.execute(SetForegroundColor(Color::White))?;
    writeln!(stdout, "{}", logo)?;

    // 版本信息横幅
    let banner = format!("  {} Edition v{}  ", info.version_info.edition.name(), info.version_info.version);

    stdout.execute(SetForegroundColor(color))?;
    writeln!(stdout, "{}", "═".repeat(banner.len()))?;
    stdout.execute(SetForegroundColor(Color::White))?;
    write!(stdout, "{}", banner)?;
    stdout.execute(SetForegroundColor(color))?;
    writeln!(stdout, "")?;
    writeln!(stdout, "{}", "═".repeat(banner.len()))?;

    // 开发版本标记
    if info.version_info.build_type == BuildType::Dev {
        stdout.execute(SetForegroundColor(Color::Yellow))?;
        writeln!(stdout, "\n  ⚠  开发版本 - 仅供测试使用")?;
    }

    // 构建信息
    stdout.execute(SetForegroundColor(Color::DarkGrey))?;
    writeln!(stdout, "\n  构建日期: {}", info.build_date)?;

    if let Some(ref hash) = info.version_info.git_hash {
        let branch_info = info
            .git_branch
            .as_ref()
            .map(|b| format!(" [{}]", b))
            .unwrap_or_default();
        writeln!(
            stdout,
            "  Git: {}{}",
            &hash[..8.min(hash.len())],
            branch_info
        )?;
    }

    writeln!(stdout, "  平台: {}", info.platform.display())?;

    stdout.execute(ResetColor)?;

    // 功能提示
    writeln!(stdout, "\n  可用功能:")?;
    let features = &info.version_info.features;
    let chunks: Vec<_> = features.chunks(4).collect();
    for chunk in chunks {
        let line = chunk
            .iter()
            .map(|f| format!("  ✓ {}", f))
            .collect::<Vec<_>>()
            .join("  ");
        writeln!(stdout, "{}", line)?;
    }

    writeln!(stdout)?;
    stdout.flush()?;

    Ok(())
}

/// 渲染精简版本行（用于提示符等场景）
pub fn render_version_line() -> io::Result<()> {
    let info = FullBuildInfo::current();
    let mut stdout = io::stdout();

    let color = match info.version_info.edition {
        Edition::Lite => Color::Cyan,
        Edition::Standard => Color::Blue,
        Edition::Pro => Color::Magenta,
    };

    let dev_marker = if info.version_info.build_type == BuildType::Dev {
        " [Dev]"
    } else {
        ""
    };

    stdout.execute(SetForegroundColor(color))?;
    write!(
        stdout,
        "EasySSH {} {}{}",
        info.version_info.edition.short_identifier(),
        info.version_info.version,
        dev_marker
    )?;
    stdout.execute(ResetColor)?;
    stdout.flush()?;

    Ok(())
}

/// 渲染状态栏版本信息
pub fn render_status_bar_version() -> io::Result<()> {
    let info = FullBuildInfo::current();
    let mut stdout = io::stdout();

    // 移动光标到最后一行
    let (_, rows) = crossterm::terminal::size()?;
    stdout.execute(cursor::MoveTo(0, rows - 1))?;

    // 清除行
    stdout.execute(Clear(ClearType::CurrentLine))?;

    // 版本信息
    let dev_marker = if info.version_info.build_type == BuildType::Dev {
        " [Dev]"
    } else {
        ""
    };

    let version_text = format!(
        " {} {} {} | {} | {} ",
        info.version_info.edition.name(),
        info.version_info.version,
        dev_marker,
        info.platform.display(),
        &info.build_date
    );

    // 填充背景
    let (_, cols) = crossterm::terminal::size()?;
    let padding = cols as usize - version_text.len();

    stdout.execute(SetForegroundColor(Color::White))?;
    stdout.execute(SetForegroundColor(match info.version_info.edition {
        Edition::Lite => Color::DarkCyan,
        Edition::Standard => Color::DarkBlue,
        Edition::Pro => Color::DarkMagenta,
    }))?;

    write!(stdout, "{}", " ".repeat(padding))?;
    write!(stdout, "{}", version_text)?;

    stdout.execute(ResetColor)?;
    stdout.flush()?;

    Ok(())
}

/// 渲染关于对话框
pub fn render_about_dialog() -> io::Result<()> {
    let info = FullBuildInfo::current();
    let mut stdout = io::stdout();

    // 清屏
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;

    // 标题
    stdout.execute(SetForegroundColor(Color::White))?;
    writeln!(stdout, "\n  ╔════════════════════════════════════════╗")?;
    writeln!(stdout, "  ║                                        ║")?;
    writeln!(stdout, "  ║          E a s y S S H                 ║")?;
    writeln!(stdout, "  ║                                        ║")?;

    let edition_text = format!("{} Edition", info.version_info.edition.name());
    writeln!(stdout, "  ║{}║", format!("{:^40}", edition_text))?;

    writeln!(stdout, "  ║                                        ║")?;
    writeln!(stdout, "  ╚════════════════════════════════════════╝")?;

    // 版本信息
    writeln!(stdout)?;
    stdout.execute(SetForegroundColor(Color::Cyan))?;
    writeln!(stdout, "  版本信息")?;
    stdout.execute(ResetColor)?;
    writeln!(stdout, "  ────────────────────────────────────────")?;
    writeln!(stdout, "  版本:        {}", info.version_info.version)?;
    writeln!(stdout, "  版本类型:    {}", info.version_info.edition.name())?;
    writeln!(stdout, "  构建日期:    {}", info.build_date)?;

    if let Some(ref hash) = info.version_info.git_hash {
        let branch_info = info
            .git_branch
            .as_ref()
            .map(|b| format!(" [{}]", b))
            .unwrap_or_default();
        writeln!(
            stdout,
            "  Git:         {}{}",
            &hash[..8.min(hash.len())],
            branch_info
        )?;
    }

    writeln!(stdout, "  平台:        {}", info.platform.display())?;

    if info.version_info.build_type == BuildType::Dev {
        stdout.execute(SetForegroundColor(Color::Yellow))?;
        writeln!(stdout, "  构建类型:    开发版本")?;
        stdout.execute(ResetColor)?;
    }

    // 功能列表
    writeln!(stdout)?;
    stdout.execute(SetForegroundColor(Color::Cyan))?;
    writeln!(stdout, "  已启用功能")?;
    stdout.execute(ResetColor)?;
    writeln!(stdout, "  ────────────────────────────────────────")?;

    for feature in &info.version_info.features {
        writeln!(stdout, "    ✓ {}", feature)?;
    }

    // 版权信息
    writeln!(stdout)?;
    stdout.execute(SetForegroundColor(Color::DarkGrey))?;
    writeln!(stdout, "  © 2024 EasySSH Team. All rights reserved.")?;
    writeln!(stdout, "  https://easyssh.dev")?;
    stdout.execute(ResetColor)?;

    writeln!(stdout)?;
    writeln!(stdout, "  按任意键继续...")?;
    stdout.flush()?;

    Ok(())
}

/// 渲染版本对比表格
pub fn render_edition_comparison() -> io::Result<()> {
    let mut stdout = io::stdout();

    // 清屏
    stdout.execute(Clear(ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;

    // 标题
    stdout.execute(SetForegroundColor(Color::White))?;
    writeln!(stdout, "\n  版本功能对比")?;
    stdout.execute(ResetColor)?;
    writeln!(
        stdout,
        "  ════════════════════════════════════════════════════════════════"
    )?;

    // 表头
    writeln!(stdout)?;
    writeln!(
        stdout,
        "  {:<20} {:<12} {:<12} {:<12}",
        "功能", "Lite", "Standard", "Pro"
    )?;
    writeln!(
        stdout,
        "  ────────────────────────────────────────────────────────────────"
    )?;

    // 功能行
    let features = vec![
        ("SSH连接", "✓", "✓", "✓"),
        ("密钥管理", "✓", "✓", "✓"),
        ("原生终端", "✓", "✓", "✓"),
        ("嵌入式终端", "✗", "✓", "✓"),
        ("分屏功能", "✗", "✓", "✓"),
        ("SFTP传输", "✗", "✓", "✓"),
        ("服务器监控", "✗", "✓", "✓"),
        ("日志监控", "✗", "✓", "✓"),
        ("Docker管理", "✗", "✓", "✓"),
        ("团队协作", "✗", "✗", "✓"),
        ("审计日志", "✗", "✗", "✓"),
        ("SSO集成", "✗", "✗", "✓"),
        ("高级安全", "✗", "✗", "✓"),
    ];

    for (name, lite, standard, pro) in features {
        let current = Edition::current();
        let lite_marker = if current == Edition::Lite {
            "●"
        } else {
            lite
        };
        let std_marker = if current == Edition::Standard {
            "●"
        } else {
            standard
        };
        let pro_marker = if current == Edition::Pro { "●" } else { pro };

        writeln!(
            stdout,
            "  {:<20} {:<12} {:<12} {:<12}",
            name, lite_marker, std_marker, pro_marker
        )?;
    }

    writeln!(stdout)?;
    writeln!(stdout, "  ● = 当前版本")?;
    writeln!(stdout)?;
    writeln!(stdout, "  按任意键继续...")?;
    stdout.flush()?;

    Ok(())
}

/// 版本升级提示
pub struct UpgradePrompt {
    required: Edition,
}

impl UpgradePrompt {
    pub fn new(required: Edition) -> Self {
        Self { required }
    }

    pub fn render(&self) -> io::Result<()> {
        let current = Edition::current();

        // Use tier comparison to check if current edition meets requirement
        if current.tier() >= self.required.tier() {
            return Ok(());
        }

        let mut stdout = io::stdout();

        stdout.execute(SetForegroundColor(Color::Yellow))?;
        writeln!(stdout)?;
        writeln!(stdout, "  ⚠  版本限制")?;
        writeln!(stdout, "  ────────────────────────────────────────")?;
        stdout.execute(ResetColor)?;

        writeln!(
            stdout,
            "  此功能需要 {} 版本，您当前使用的是 {} 版本。",
            self.required.name(),
            current.name()
        )?;

        writeln!(stdout)?;
        writeln!(stdout, "  升级 {} 版本可获得:", self.required.name())?;

        let benefits = match self.required {
            Edition::Standard => vec![
                "• 嵌入式终端 - 内置Web终端，无需外部工具",
                "• 分屏功能 - 同时查看多个会话",
                "• SFTP文件传输 - 图形化文件管理",
                "• 服务器监控 - 实时性能指标",
            ],
            Edition::Pro => vec![
                "• 团队协作 - 共享服务器配置",
                "• 审计日志 - 完整操作记录",
                "• SSO集成 - 企业单点登录",
                "• 高级安全 - 2FA、密钥轮换",
            ],
            _ => vec![],
        };

        for benefit in benefits {
            writeln!(stdout, "    {}", benefit)?;
        }

        writeln!(stdout)?;
        writeln!(stdout, "  访问 https://easyssh.dev/upgrade 了解更多")?;
        writeln!(stdout)?;

        stdout.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = FullBuildInfo::current();
        assert!(!info.version_info.version.is_empty());
        assert!(!info.build_date.is_empty());
    }
}
