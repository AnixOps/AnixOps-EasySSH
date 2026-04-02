#!/usr/bin/env powershell
# EasySSH 前台自动化监控脚本
# 实时显示构建/测试/安全/CI/CD状态

$REPO_PATH = "C:\Users\z7299\Documents\GitHub\AnixOps-EasySSH"
$host.ui.RawUI.WindowTitle = "🔧 EasySSH 前台自动化监控中心"

# 颜色定义
function Write-ColorLine($text, $color) {
    switch($color) {
        "green" { Write-Host $text -ForegroundColor Green }
        "red" { Write-Host $text -ForegroundColor Red }
        "yellow" { Write-Host $text -ForegroundColor Yellow }
        "cyan" { Write-Host $text -ForegroundColor Cyan }
        default { Write-Host $text }
    }
}

# 清除屏幕
function Clear-Screen {
    Clear-Host
}

# 显示头部
function Show-Header {
    Clear-Screen
    Write-ColorLine "╔══════════════════════════════════════════════════════════════════════════════╗" "cyan"
    Write-ColorLine "║                    🔧 EasySSH 前台自动化监控中心 🔧                           ║" "cyan"
    Write-ColorLine "╠══════════════════════════════════════════════════════════════════════════════╣" "cyan"
    Write-ColorLine "║ 时间: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')                                                    ║" "cyan"
    Write-ColorLine "╠══════════════════════════════════════════════════════════════════════════════╣" "cyan"
}

# 显示底部
function Show-Footer {
    Write-ColorLine "╠══════════════════════════════════════════════════════════════════════════════╣" "cyan"
    Write-ColorLine "║ 命令: [R]刷新 [A]修复全部 [Q]退出 [C]清理 [S]提交                              ║" "cyan"
    Write-ColorLine "╚══════════════════════════════════════════════════════════════════════════════╝" "cyan"
}

# 检查代码质量
function Check-CodeQuality {
    param([ref]$status, [ref]$details, [ref]$color)

    Set-Location $REPO_PATH

    # 检查格式化
    $fmt = cargo fmt -- --check 2>&1
    $fmt_ok = $LASTEXITCODE -eq 0

    # 检查 clippy
    $clippy = cargo clippy --all-targets 2>&1 | Select-Object -First 20
    $clippy_ok = ($clippy -join "").Contains("Finished") -or -not ($clippy -join "").Contains("error:")

    if ($fmt_ok -and $clippy_ok) {
        $status.Value = "✅"
        $details.Value = "代码格式和clippy检查通过"
        $color.Value = "green"
    } elseif (-not $fmt_ok) {
        $status.Value = "✗"
        $details.Value = "需要格式化: cargo fmt"
        $color.Value = "red"
    } else {
        $status.Value = "✗"
        $details.Value = "Clippy警告需要修复"
        $color.Value = "red"
    }
}

# 检查安全审计
function Check-Security {
    param([ref]$status, [ref]$details, [ref]$color)

    Set-Location $REPO_PATH

    # 检查是否存在 cargo audit
    $audit_cmd = Get-Command cargo-audit -ErrorAction SilentlyContinue
    if (-not $audit_cmd) {
        $status.Value = "⏳"
        $details.Value = "cargo-audit 未安装"
        $color.Value = "yellow"
        return
    }

    $audit = cargo audit 2>&1 | Select-Object -First 30
    $audit_str = $audit -join ""

    if ($audit_str -match "RUSTSEC") {
        $count = ([regex]::Matches($audit_str, "RUSTSEC")).Count
        $status.Value = "✗"
        $details.Value = "发现 $count 个安全警告"
        $color.Value = "red"
    } elseif ($audit_str -match "Success") {
        $status.Value = "✅"
        $details.Value = "未发现安全漏洞"
        $color.Value = "green"
    } else {
        $status.Value = "⏳"
        $details.Value = "安全检查未完成"
        $color.Value = "yellow"
    }
}

# 检查构建状态
function Check-Build {
    param([ref]$status, [ref]$details, [ref]$color)

    Set-Location $REPO_PATH

    # 检查 core
    $core = cargo check -p easyssh-core 2>&1 | Select-Object -First 20
    $core_str = $core -join ""
    $core_ok = $core_str.Contains("Finished") -and -not $core_str.Contains("error:")

    # 检查 winui
    $winui = cargo check -p easyssh-winui 2>&1 | Select-Object -First 20
    $winui_str = $winui -join ""
    $winui_ok = $winui_str.Contains("Finished") -and -not $winui_str.Contains("error:")

    if ($core_ok -and $winui_ok) {
        $status.Value = "✅"
        $details.Value = "Core + WinUI 检查通过"
        $color.Value = "green"
    } elseif (-not $core_ok) {
        $error_count = ([regex]::Matches($core_str, "error:")).Count
        $status.Value = "✗"
        $details.Value = "Core 有 $error_count 个错误"
        $color.Value = "red"
    } else {
        $error_count = ([regex]::Matches($winui_str, "error:")).Count
        $status.Value = "✗"
        $details.Value = "WinUI 有 $error_count 个错误"
        $color.Value = "red"
    }
}

# 检查测试
function Check-Tests {
    param([ref]$status, [ref]$details, [ref]$color)

    Set-Location $REPO_PATH

    $test = cargo test -p easyssh-core --no-fail-fast -- --test-threads=1 2>&1 | Select-Object -Last 30
    $test_str = $test -join ""

    if ($test_str -match "test result: ok") {
        $passed = [regex]::Match($test_str, '(\d+) passed').Groups[1].Value
        $status.Value = "✅"
        $details.Value = "$passed 个测试通过"
        $color.Value = "green"
    } elseif ($test_str -match "FAILED") {
        $failed = ([regex]::Matches($test_str, "FAILED")).Count
        $status.Value = "✗"
        $details.Value = "$failed 个测试失败"
        $color.Value = "red"
    } else {
        $status.Value = "⏳"
        $details.Value = "测试未执行或编译错误"
        $color.Value = "yellow"
    }
}

# 检查CI/CD
function Check-CICD {
    param([ref]$status, [ref]$details, [ref]$color)

    $workflow_dir = "$REPO_PATH\.github\workflows"

    if (Test-Path $workflow_dir) {
        $workflows = Get-ChildItem $workflow_dir -Filter "*.yml"
        $count = $workflows.Count

        # 尝试获取最近的 GitHub Actions 运行状态
        $gh_cmd = Get-Command gh -ErrorAction SilentlyContinue
        if ($gh_cmd) {
            Set-Location $REPO_PATH
            $runs = gh run list -R anixn/EasySSH -L 5 --json status,conclusion 2>&1 | ConvertFrom-Json -ErrorAction SilentlyContinue
            if ($runs) {
                $failures = $runs | Where-Object { $_.conclusion -eq "failure" }
                if ($failures.Count -gt 0) {
                    $status.Value = "✗"
                    $details.Value = "最近的 CI 运行失败"
                    $color.Value = "red"
                    return
                }
                $success = $runs | Where-Object { $_.conclusion -eq "success" }
                if ($success.Count -gt 0) {
                    $status.Value = "✅"
                    $details.Value = "$count 个工作流, CI 正常"
                    $color.Value = "green"
                    return
                }
            }
        }

        $status.Value = "✅"
        $details.Value = "$count 个工作流配置就绪"
        $color.Value = "green"
    } else {
        $status.Value = "✗"
        $details.Value = "工作流目录不存在"
        $color.Value = "red"
    }
}

# 自动修复
function Auto-Fix {
    Write-ColorLine "" ""
    Write-ColorLine "🔧 开始自动修复..." "cyan"

    Set-Location $REPO_PATH

    # 格式化代码
    Write-ColorLine "  → 执行 cargo fmt..." "yellow"
    cargo fmt 2>&1 | Out-Null

    # 清理缓存
    Write-ColorLine "  → 清理构建缓存..." "yellow"
    cargo clean 2>&1 | Out-Null

    # 重新检查
    Write-ColorLine "  → 重新构建..." "yellow"
    cargo check -p easyssh-core 2>&1 | Select-Object -First 10 | Out-Null

    Write-ColorLine "✅ 自动修复完成" "green"
    Start-Sleep -Seconds 2
}

# 显示状态行
function Show-StatusLine($name, $status, $time, $details, $color) {
    $truncated = if ($details.Length -gt 38) { $details.Substring(0, 35) + "..." } else { $details }
    $line = "║ {0,-12} │ {1,-2} │ {2,-19} │ {3,-40} ║" -f $name, $status, $time, $truncated
    Write-ColorLine $line $color
}

# 主循环
function Main-Loop {
    # 初始状态
    $checks = @(
        @{ Name = "代码质量"; Status = "⏳"; LastCheck = "等待中"; Details = "准备检查..."; Color = "yellow" }
        @{ Name = "安全审计"; Status = "⏳"; LastCheck = "等待中"; Details = "准备检查..."; Color = "yellow" }
        @{ Name = "构建状态"; Status = "⏳"; LastCheck = "等待中"; Details = "准备检查..."; Color = "yellow" }
        @{ Name = "测试执行"; Status = "⏳"; LastCheck = "等待中"; Details = "准备检查..."; Color = "yellow" }
        @{ Name = "CI/CD流程"; Status = "⏳"; LastCheck = "等待中"; Details = "准备检查..."; Color = "yellow" }
    )

    # 首次完整检查
    Show-Header
    Write-ColorLine "║ 首次检查中，请稍候...                                                           ║" "yellow"

    Check-CodeQuality -status ([ref]$checks[0].Status) -details ([ref]$checks[0].Details) -color ([ref]$checks[0].Color)
    $checks[0].LastCheck = Get-Date -Format "HH:mm:ss"

    Check-Security -status ([ref]$checks[1].Status) -details ([ref]$checks[1].Details) -color ([ref]$checks[1].Color)
    $checks[1].LastCheck = Get-Date -Format "HH:mm:ss"

    Check-Build -status ([ref]$checks[2].Status) -details ([ref]$checks[2].Details) -color ([ref]$checks[2].Color)
    $checks[2].LastCheck = Get-Date -Format "HH:mm:ss"

    Check-Tests -status ([ref]$checks[3].Status) -details ([ref]$checks[3].Details) -color ([ref]$checks[3].Color)
    $checks[3].LastCheck = Get-Date -Format "HH:mm:ss"

    Check-CICD -status ([ref]$checks[4].Status) -details ([ref]$checks[4].Details) -color ([ref]$checks[4].Color)
    $checks[4].LastCheck = Get-Date -Format "HH:mm:ss"

    $lastFullCheck = Get-Date

    while ($true) {
        Show-Header

        foreach ($check in $checks) {
            Show-StatusLine $check.Name $check.Status $check.LastCheck $check.Details $check.Color
        }

        Show-Footer
        Write-ColorLine "按 R 刷新, A 修复, Q 退出, C 清理, S 提交" "cyan"

        # 检查是否到了自动刷新时间（30秒）
        $timeSinceLastCheck = (Get-Date) - $lastFullCheck
        if ($timeSinceLastCheck.TotalSeconds -ge 30) {
            Write-ColorLine "`n🔄 自动刷新中..." "cyan"

            Check-CodeQuality -status ([ref]$checks[0].Status) -details ([ref]$checks[0].Details) -color ([ref]$checks[0].Color)
            $checks[0].LastCheck = Get-Date -Format "HH:mm:ss"

            Check-Security -status ([ref]$checks[1].Status) -details ([ref]$checks[1].Details) -color ([ref]$checks[1].Color)
            $checks[1].LastCheck = Get-Date -Format "HH:mm:ss"

            Check-Build -status ([ref]$checks[2].Status) -details ([ref]$checks[2].Details) -color ([ref]$checks[2].Color)
            $checks[2].LastCheck = Get-Date -Format "HH:mm:ss"

            Check-Tests -status ([ref]$checks[3].Status) -details ([ref]$checks[3].Details) -color ([ref]$checks[3].Color)
            $checks[3].LastCheck = Get-Date -Format "HH:mm:ss"

            Check-CICD -status ([ref]$checks[4].Status) -details ([ref]$checks[4].Details) -color ([ref]$checks[4].Color)
            $checks[4].LastCheck = Get-Date -Format "HH:mm:ss"

            $lastFullCheck = Get-Date
            continue
        }

        # 等待按键（超时1秒）
        if ([Console]::KeyAvailable) {
            $key = [Console]::ReadKey($true).Key

            switch ($key) {
                "R" {  # 刷新
                    Write-ColorLine "`n🔄 手动刷新..." "cyan"

                    Check-CodeQuality -status ([ref]$checks[0].Status) -details ([ref]$checks[0].Details) -color ([ref]$checks[0].Color)
                    $checks[0].LastCheck = Get-Date -Format "HH:mm:ss"

                    Check-Security -status ([ref]$checks[1].Status) -details ([ref]$checks[1].Details) -color ([ref]$checks[1].Color)
                    $checks[1].LastCheck = Get-Date -Format "HH:mm:ss"

                    Check-Build -status ([ref]$checks[2].Status) -details ([ref]$checks[2].Details) -color ([ref]$checks[2].Color)
                    $checks[2].LastCheck = Get-Date -Format "HH:mm:ss"

                    Check-Tests -status ([ref]$checks[3].Status) -details ([ref]$checks[3].Details) -color ([ref]$checks[3].Color)
                    $checks[3].LastCheck = Get-Date -Format "HH:mm:ss"

                    Check-CICD -status ([ref]$checks[4].Status) -details ([ref]$checks[4].Details) -color ([ref]$checks[4].Color)
                    $checks[4].LastCheck = Get-Date -Format "HH:mm:ss"

                    $lastFullCheck = Get-Date
                }
                "A" {  # 自动修复
                    Auto-Fix
                }
                "C" {  # 清理
                    Write-ColorLine "`n🧹 清理中..." "cyan"
                    Set-Location $REPO_PATH
                    cargo clean 2>&1 | Out-Null
                    Write-ColorLine "✅ 清理完成" "green"
                    Start-Sleep -Seconds 1
                }
                "S" {  # 提交
                    Write-ColorLine "`n📦 提交更改..." "cyan"
                    Set-Location $REPO_PATH
                    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
                    git add -A 2>&1 | Out-Null
                    git commit -m "auto: CI监控自动修复 $timestamp" 2>&1 | Out-Null
                    git push 2>&1 | Select-Object -First 5
                    Write-ColorLine "✅ 提交完成" "green"
                    Start-Sleep -Seconds 2
                }
                "Q" {  # 退出
                    Write-ColorLine "`n👋 退出监控..." "green"
                    exit 0
                }
            }
        }

        Start-Sleep -Milliseconds 100
    }
}

# 入口点
Write-ColorLine "启动 EasySSH 前台自动化监控..." "green"
Write-ColorLine "仓库路径: $REPO_PATH" "cyan"

# 检查仓库路径
if (-not (Test-Path $REPO_PATH)) {
    Write-ColorLine "错误: 仓库路径不存在!" "red"
    exit 1
}

# 进入主循环
Main-Loop
