#!/usr/bin/env pwsh
# EasySSH Lite Version Build Script for Windows
# Supports Windows (x64, ARM64)
# Usage: .\build-lite.ps1 [version] [target]
#   version: semantic version (default: extracted from Cargo.toml)
#   target: specific target (default: native, options: x64, arm64, all)

param(
    [string]$Version = "",
    [string]$Target = "native"
)

# Requires PowerShell 7.0+
#Requires -Version 7.0

$ErrorActionPreference = "Stop"

# ============================================================================
# Configuration
# ============================================================================
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir "..\..")
$WorkspaceRoot = $ProjectRoot

# Colors for output
function Write-Info { param([string]$Message) Write-Host "[INFO] $Message" -ForegroundColor Green }
function Write-Warn { param([string]$Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Error { param([string]$Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }
function Write-Step { param([string]$Message) Write-Host "[STEP] $Message" -ForegroundColor Cyan }

# ============================================================================
# Version Extraction
# ============================================================================
function Get-Version {
    $cargoToml = Join-Path $WorkspaceRoot "Cargo.toml"
    if (Test-Path $cargoToml) {
        $content = Get-Content $cargoToml -Raw
        if ($content -match 'version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"') {
            return $matches[1]
        }
    }
    return "0.3.0"
}

if ([string]::IsNullOrEmpty($Version)) {
    $Version = Get-Version
}

$ReleaseDir = Join-Path $WorkspaceRoot "releases\lite-v$Version"
$BuildProfile = "release-lite"

# ============================================================================
# Dependency Checks
# ============================================================================
function Test-Dependencies {
    Write-Step "Checking dependencies..."

    # Check Rust
    try {
        $rustVersion = rustc --version 2>&1
        Write-Info "Rust version: $rustVersion"
    }
    catch {
        Write-Error "Rust is not installed. Please install Rust: https://rustup.rs/"
        exit 1
    }

    # Check Cargo
    try {
        $cargoVersion = cargo --version 2>&1
        Write-Info "Cargo version: $cargoVersion"
    }
    catch {
        Write-Error "Cargo is not installed"
        exit 1
    }

    # Check for required Windows SDK components
    $windowsSdkFound = $false
    $programFiles = $env:ProgramFiles
    $windowsSdkPaths = @(
        "${programFiles(x86)}\Windows Kits\10\bin",
        "${programFiles}\Windows Kits\10\bin"
    )

    foreach ($path in $windowsSdkPaths) {
        if (Test-Path $path) {
            $windowsSdkFound = $true
            break
        }
    }

    if (-not $windowsSdkFound) {
        Write-Warn "Windows SDK not found. Code signing may not work."
    }

    # Check for WiX (Windows Installer XML) for MSI creation
    $wixPath = "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin"
    $script:HasWiX = Test-Path $wixPath
    if ($script:HasWiX) {
        Write-Info "WiX Toolset found - MSI creation available"
    }
    else {
        Write-Warn "WiX Toolset not found - MSI creation will be skipped"
    }

    Write-Info "All dependencies satisfied"
}

# ============================================================================
# Build Functions
# ============================================================================
function Build-Windows {
    param([string]$Arch = "x64")

    Write-Step "Building EasySSH Lite for Windows $Arch..."

    $target = ""
    $crossCompile = $false

    switch ($Arch) {
        "x64" {
            $target = "x86_64-pc-windows-msvc"
        }
        "arm64" {
            $target = "aarch64-pc-windows-msvc"
            $crossCompile = $true
        }
        default {
            Write-Error "Unsupported architecture: $Arch"
            exit 1
        }
    }

    # Install target if not present
    $installedTargets = rustup target list --installed
    if ($installedTargets -notcontains $target) {
        Write-Info "Installing target $target..."
        rustup target add $target
    }

    $crateDir = Join-Path $WorkspaceRoot "crates\easyssh-platforms\windows\easyssh-winui"

    # Set build flags
    $env:RUSTFLAGS = "-C target-feature=+crt-static"

    # Build with Lite features
    if ($crossCompile) {
        Write-Info "Cross-compiling for ARM64..."
        # Note: Cross-compilation to ARM64 on Windows requires specific setup
        # This assumes LLVM/Clang is installed for ARM64 target
    }

    Push-Location $crateDir
    try {
        $buildArgs = @(
            "build",
            "--profile", $BuildProfile,
            "--target", $target,
            "--features", "lite",
            "--no-default-features"
        )

        # Add version injection via build script
        $env:EASYSSH_VERSION = $Version
        $env:CARGO_PKG_VERSION = $Version

        & cargo @buildArgs

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Build failed with exit code $LASTEXITCODE"
            exit 1
        }
    }
    finally {
        Pop-Location
    }

    # Determine binary path
    $binaryPath = Join-Path $WorkspaceRoot "target\$target\$BuildProfile\EasySSH.exe"

    # Package the build
    Package-Windows -Arch $Arch -BinaryPath $binaryPath
}

# ============================================================================
# Packaging Functions
# ============================================================================
function Package-Windows {
    param(
        [string]$Arch,
        [string]$BinaryPath
    )

    Write-Step "Packaging for Windows $Arch..."

    $pkgName = "easyssh-lite-v$Version-windows-$Arch"
    $pkgDir = Join-Path $ReleaseDir $pkgName

    # Create package structure
    New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null

    # Copy binary
    $destBinary = Join-Path $pkgDir "EasySSH Lite.exe"
    Copy-Item $BinaryPath $destBinary -Force

    # Copy assets if exist
    $assetsDir = Join-Path $WorkspaceRoot "crates\easyssh-platforms\windows\easyssh-winui\assets"
    if (Test-Path $assetsDir) {
        $destAssets = Join-Path $pkgDir "assets"
        Copy-Item -Recurse $assetsDir $destAssets -Force
    }

    # Create README
    $readmePath = Join-Path $pkgDir "README.txt"
    @"
EasySSH Lite v$Version for Windows
=====================================

Quick Start:
1. Run "EasySSH Lite.exe"
2. Add your SSH servers via the UI
3. Connect using password or key authentication

System Requirements:
- Windows 10/11 64-bit
- No additional dependencies required

Features:
- Native Windows UI with egui
- SSH connection management
- Password and key-based authentication
- Server grouping
- Secure credential storage via Windows Credential Manager

For support: https://github.com/anixops/easyssh

Version: $Version
Built: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
"@ | Set-Content $readmePath -Encoding UTF8

    # Create ZIP archive
    $zipPath = Join-Path $ReleaseDir "$pkgName.zip"
    Compress-Archive -Path "$pkgDir\*" -DestinationPath $zipPath -Force

    Write-Info "Package created: $zipPath"

    # Create installer if WiX is available
    if ($script:HasWiX) {
        Create-WindowsInstaller -Arch $Arch -PkgDir $pkgDir -PkgName $pkgName
    }

    # Prepare for code signing
    Prepare-CodeSigning -Arch $Arch -BinaryPath $destBinary -PkgName $pkgName
}

function Create-WindowsInstaller {
    param(
        [string]$Arch,
        [string]$PkgDir,
        [string]$PkgName
    )

    Write-Step "Creating Windows installer (MSI)..."

    $wixDir = Join-Path $ReleaseDir "wix-$Arch"
    New-Item -ItemType Directory -Force -Path $wixDir | Out-Null

    # Create WiX source files
    $wxsPath = Join-Path $wixDir "easyssh-lite.wxs"

    @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="*"
             Name="EasySSH Lite"
             Language="1033"
             Version="$Version"
             Manufacturer="AnixOps"
             UpgradeCode="E3F5A2B7-8C9D-4E1F-2A3B-5C6D7E8F9A0B">

        <Package InstallerVersion="200" Compressed="yes" InstallScope="perUser" />

        <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
        <MediaTemplate EmbedCab="yes" />

        <Feature Id="ProductFeature" Title="EasySSH Lite" Level="1">
            <ComponentGroupRef Id="ProductComponents" />
        </Feature>

        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="LocalAppDataFolder">
                <Directory Id="APPLICATIONFOLDER" Name="EasySSH Lite">
                    <Component Id="MainExecutable" Guid="*">
                        <File Id="EasySSHLiteExe" Source="$($PkgDir -replace '\\', '\\')\\EasySSH Lite.exe" KeyPath="yes">
                            <Shortcut Id="StartMenuShortcut" Directory="ProgramMenuFolder" Name="EasySSH Lite"
                                     WorkingDirectory="APPLICATIONFOLDER" Icon="EasySSHLite.exe" Advertise="yes" />
                        </File>
                    </Component>
                </Directory>
            </Directory>
            <Directory Id="ProgramMenuFolder" Name="Programs" />
        </Directory>

        <ComponentGroup Id="ProductComponents" Directory="APPLICATIONFOLDER">
            <ComponentRef Id="MainExecutable" />
        </ComponentGroup>

        <Icon Id="EasySSHLite.exe" SourceFile="$($PkgDir -replace '\\', '\\')\\EasySSH Lite.exe" />
        <Property Id="ARPPRODUCTICON" Value="EasySSHLite.exe" />
    </Product>
</Wix>
"@ | Set-Content $wxsPath -Encoding UTF8

    # Compile WiX
    $wixBin = "${env:ProgramFiles(x86)}\WiX Toolset v3.11\bin"
    $candle = Join-Path $wixBin "candle.exe"
    $light = Join-Path $wixBin "light.exe"

    if ((Test-Path $candle) -and (Test-Path $light)) {
        $objPath = Join-Path $wixDir "easyssh-lite.wixobj"
        $msiPath = Join-Path $ReleaseDir "$pkgName.msi"

        & $candle -out "$objPath" "$wxsPath" -arch $Arch
        & $light -out "$msiPath" "$objPath" -ext WixUIExtension

        if (Test-Path $msiPath) {
            Write-Info "MSI installer created: $msiPath"
        }
    }
}

function Prepare-CodeSigning {
    param(
        [string]$Arch,
        [string]$BinaryPath,
        [string]$PkgName
    )

    Write-Step "Preparing for code signing..."

    $signScript = Join-Path $ReleaseDir "sign-windows-$Arch.ps1"

    @"
# Windows Code Signing Script
# Usage: .\sign-windows-$Arch.ps1 -CertificateThumbprint "THUMBPRINT"

param(
    [Parameter(Mandatory=`$true)]
    [string]`$CertificateThumbprint,

    [string]`$TimestampUrl = "http://timestamp.digicert.com",

    [switch]`$SignMSI = `$false
)

`$ErrorActionPreference = "Stop"

`$pkgName = "$PkgName"
`$releaseDir = "$($ReleaseDir -replace '\\', '\\')"

# Sign the executable
`$exePath = Join-Path `$releaseDir "`$pkgName\EasySSH Lite.exe"
if (Test-Path `$exePath) {
    Write-Host "Signing executable: `$exePath" -ForegroundColor Cyan
    signtool.exe sign /sha1 `$CertificateThumbprint /tr `$TimestampUrl /td sha256 /fd sha256 "`$exePath"
}

# Sign the MSI if exists and requested
if (`$SignMSI) {
    `$msiPath = Join-Path `$releaseDir "`$pkgName.msi"
    if (Test-Path `$msiPath) {
        Write-Host "Signing MSI: `$msiPath" -ForegroundColor Cyan
        signtool.exe sign /sha1 `$CertificateThumbprint /tr `$TimestampUrl /td sha256 /fd sha256 "`$msiPath"
    }
}

Write-Host "Signing complete!" -ForegroundColor Green

# Verify signature
signtool.exe verify /pa "`$exePath"
"@ | Set-Content $signScript -Encoding UTF8

    Write-Info "Signing script created: $signScript"

    # Create batch file for easier execution
    $signBatch = Join-Path $ReleaseDir "sign-windows-$Arch.bat"
    @"
@echo off
echo Windows Code Signing for EasySSH Lite
echo =====================================
echo.
echo Prerequisites:
echo 1. Code signing certificate installed in Windows Certificate Store
echo 2. signtool.exe available (included in Windows SDK)
echo.
echo Usage:
echo   sign-windows-$Arch.bat ^<CertificateThumbprint^>
echo.
echo Example:
echo   sign-windows-$Arch.bat A1B2C3D4E5F6...
echo.

if "%~1"=="" (
    echo Error: Certificate thumbprint required
    exit /b 1
)

powershell -ExecutionPolicy Bypass -File "%~dp0sign-windows-$Arch.ps1" -CertificateThumbprint %1
pause
"@ | Set-Content $signBatch -Encoding ASCII

    Write-Info "Signing batch file created: $signBatch"
}

# ============================================================================
# Checksum Generation
# ============================================================================
function Generate-Checksums {
    Write-Step "Generating checksums..."

    $checksumFile = Join-Path $ReleaseDir "SHA256SUMS.txt"

    @"
EasySSH Lite v$Version Release Checksums
=========================================

Generated: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss UTC")

"@ | Set-Content $checksumFile -Encoding UTF8

    # Generate checksums for all packages
    Get-ChildItem $ReleaseDir -Filter "*.zip" -File | ForEach-Object {
        $hash = Get-FileHash $_.FullName -Algorithm SHA256
        $line = "$($hash.Hash)  $($_.Name)`n"
        Add-Content $checksumFile $line
    }

    Get-ChildItem $ReleaseDir -Filter "*.msi" -File | ForEach-Object {
        $hash = Get-FileHash $_.FullName -Algorithm SHA256
        $line = "`n$($hash.Hash)  $($_.Name)`n"
        Add-Content $checksumFile $line
    }

    Write-Info "Checksums written to: $checksumFile"
}

# ============================================================================
# Version Injection
# ============================================================================
function Inject-Version {
    Write-Step "Injecting version information..."

    $versionInfo = @{
        name = "EasySSH Lite"
        version = $Version
        build_date = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
        git_commit = (git rev-parse --short HEAD 2>`$null)
        git_branch = (git rev-parse --abbrev-ref HEAD 2>`$null)
        rustc_version = ((rustc --version) -split ' ')[1]
        features = @("lite")
    }

    $versionFile = Join-Path $ReleaseDir "version.json"
    $versionInfo | ConvertTo-Json | Set-Content $versionFile -Encoding UTF8

    Write-Info "Version info: $versionFile"
}

# ============================================================================
# Main Build Process
# ============================================================================
function Main {
    Write-Host "=========================================="
    Write-Host "  EasySSH Lite Build Script v$Version"
    Write-Host "  Platform: Windows"
    Write-Host "=========================================="
    Write-Host ""

    # Create release directory
    New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null

    # Run dependency checks
    Test-Dependencies

    # Determine what to build
    $archs = @()
    switch ($Target) {
        "native" { $archs = @("x64") }
        "all" { $archs = @("x64", "arm64") }
        "x64" { $archs = @("x64") }
        "arm64" { $archs = @("arm64") }
        default { $archs = @($Target) }
    }

    # Build for each architecture
    foreach ($arch in $archs) {
        Build-Windows -Arch $arch
    }

    # Post-build steps
    Inject-Version
    Generate-Checksums

    # Summary
    Write-Host ""
    Write-Host "=========================================="
    Write-Info "Build Complete!"
    Write-Host "=========================================="
    Write-Host ""
    Write-Host "Output directory: $ReleaseDir"
    Write-Host ""
    Write-Host "Generated artifacts:"
    Get-ChildItem $ReleaseDir -File | Where-Object { $_.Extension -in @(".zip", ".msi", ".json", ".txt") } |
        ForEach-Object { Write-Host "  - $($_.Name)" }
    Write-Host ""

    Write-Host "Next steps:"
    Write-Host "  1. Sign the executable: $ReleaseDir\sign-windows-x64.bat <thumbprint>"
    if ($archs -contains "arm64") {
        Write-Host "  2. Sign the ARM64 executable: $ReleaseDir\sign-windows-arm64.bat <thumbprint>"
    }
    Write-Host "  3. Test the packages in a clean VM"
    Write-Host "  4. Upload to GitHub releases"
    Write-Host ""
}

# Run main
Main
