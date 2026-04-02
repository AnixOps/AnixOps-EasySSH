use std::process::Command;

fn main() {
    // 告诉Cargo在环境变量变化时重新运行
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    // 获取Git commit hash
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=EASYSSH_GIT_HASH={}", git_hash);

    // 获取Git分支
    let git_branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=EASYSSH_GIT_BRANCH={}", git_branch);

    // 获取构建日期
    let build_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=EASYSSH_BUILD_DATE={}", build_date);

    // 获取构建时间
    let build_time = chrono::Utc::now().format("%H:%M:%S UTC").to_string();
    println!("cargo:rustc-env=EASYSSH_BUILD_TIME={}", build_time);

    // 获取Rust编译器版本
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=EASYSSH_RUSTC_VERSION={}", rustc_version);

    // 获取Cargo特性列表
    let features = std::env::var("CARGO_FEATURE_FLAGS")
        .unwrap_or_else(|_| String::new());
    if !features.is_empty() {
        println!("cargo:warning=Building with features: {}", features);
    }

    // 版本信息摘要
    println!("cargo:warning=EasySSH Build Info:");
    println!("cargo:warning=  Git: {} ({})", git_hash, git_branch);
    println!("cargo:warning=  Date: {} {}", build_date, build_time);
    println!("cargo:warning=  Rustc: {}", rustc_version);
}
