//! EasySSH Update Checker 使用示例
//!
//! 本示例演示如何使用 update_checker 模块进行版本检测

use easyssh_core::update_checker::{
    UpdateChecker, UpdateCheckerConfig, UpdateChannel, UpdateCheckResult,
};

#[tokio::main]
async fn main() {
    println!("EasySSH Update Checker 示例\n");

    // 1. 使用默认配置创建更新检测器
    let checker = UpdateChecker::default();
    println!("当前版本: {}", checker.get_current_version());

    // 2. 执行版本检查
    println!("\n正在检查更新...");
    match checker.check().await {
        UpdateCheckResult::UpdateAvailable(info) => {
            println!("发现新版本: {}", info.version);
            println!("当前版本: {}", info.current_version);
            println!("发布日期: {}", info.release_date);
            println!("发布说明: {}", info.release_notes);
            println!("下载页面: {}", info.download_url);

            if info.has_compatible_asset {
                println!("✓ 有适合当前平台的安装包");
            } else {
                println!("⚠ 未找到适合当前平台的安装包");
            }

            // 显示可用的资源文件
            println!("\n可用下载:");
            for asset in &info.assets {
                if asset.is_for_current_platform {
                    println!("  → {} ({:.2} MB)", asset.filename, asset.size as f64 / 1024.0 / 1024.0);
                } else {
                    println!("    {} ({:.2} MB)", asset.filename, asset.size as f64 / 1024.0 / 1024.0);
                }
            }
        }
        UpdateCheckResult::UpToDate => {
            println!("✓ 当前已是最新版本");
        }
        UpdateCheckResult::Skipped { reason } => {
            println!("检查被跳过: {}", reason);
        }
        UpdateCheckResult::Error(e) => {
            println!("检查出错: {}", e);
        }
    }

    // 3. 使用自定义配置
    println!("\n\n--- 使用预览版通道 ---");
    let preview_config = UpdateCheckerConfig {
        channel: UpdateChannel::Preview,
        check_interval_secs: 3600, // 每小时检查
        ..Default::default()
    };
    let preview_checker = UpdateChecker::new(preview_config);

    match preview_checker.check().await {
        UpdateCheckResult::UpdateAvailable(info) => {
            println!("预览版更新: {}", info.version);
        }
        _ => {}
    }

    // 4. 忽略特定版本
    println!("\n--- 忽略版本示例 ---");
    checker.ignore_version("1.5.0").await;
    let ignored = checker.get_ignored_versions().await;
    println!("已忽略的版本: {:?}", ignored);

    // 5. 禁用更新检查
    println!("\n--- 禁用更新检查 ---");
    checker.disable().await;
    println!("更新检查已禁用: {}", !checker.is_enabled().await);

    println!("\n示例完成!");
}
