//! Windows updater implementation using WinSparkle-inspired approach
#![cfg(target_os = "windows")]

use super::PlatformUpdater;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// Windows update flags
const MOVEFILE_REPLACE_EXISTING: u32 = 0x00000001;
const MOVEFILE_DELAY_UNTIL_REBOOT: u32 = 0x00000004;

#[derive(Debug)]
pub struct WindowsUpdater {
    app_data_dir: PathBuf,
    temp_dir: PathBuf,
    use_msi: bool,
}

impl WindowsUpdater {
    pub async fn new() -> anyhow::Result<Self> {
        let app_data_dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find AppData directory"))?
            .join("EasySSH");

        let temp_dir = std::env::temp_dir().join("easyssh-updates");
        tokio::fs::create_dir_all(&temp_dir).await?;
        tokio::fs::create_dir_all(&app_data_dir).await?;

        // Detect if we're installed via MSI
        let use_msi = Self::detect_msi_installation().await;

        Ok(Self {
            app_data_dir,
            temp_dir,
            use_msi,
        })
    }

    async fn detect_msi_installation() -> bool {
        // Check registry for MSI installation
        let output = Command::new("reg")
            .args(&[
                "query",
                "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
                "/s",
                "/f",
                "EasySSH",
            ])
            .output();

        match output {
            Ok(result) => String::from_utf8_lossy(&result.stdout).contains("EasySSH"),
            Err(_) => false,
        }
    }

    async fn install_msi(&self, msi_path: &Path) -> anyhow::Result<()> {
        // Install MSI silently
        let output = Command::new("msiexec.exe")
            .args(&[
                "/i",
                msi_path.to_str().unwrap(),
                "/qn",        // No UI
                "/norestart", // Don't restart automatically
                "/log",       // Log to temp
                self.temp_dir.join("install.log").to_str().unwrap(),
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("MSI installation failed: {}", stderr));
        }

        Ok(())
    }

    async fn install_portable(&self, exe_path: &Path) -> anyhow::Result<()> {
        let current_exe = std::env::current_exe()?;

        // Create batch script for delayed replacement
        let batch_script = format!(
            r#"@echo off
:retry
timeout /t 1 /nobreak >nul
tasklist | find /i "{}" >nul
if errorlevel 1 goto proceed
goto retry
:proceed
copy /Y "{}" "{}"
start "" "{}"
del "%~f0"
"#,
            current_exe.file_stem().unwrap().to_str().unwrap(),
            exe_path.display(),
            current_exe.display(),
            current_exe.display(),
        );

        let batch_path = self.temp_dir.join("update.bat");
        tokio::fs::write(&batch_path, batch_script).await?;

        // Execute batch script in background
        Command::new("cmd.exe")
            .args(&["/C", "start", "/B", batch_path.to_str().unwrap()])
            .spawn()?;

        Ok(())
    }

    async fn install_nsis(&self, exe_path: &Path) -> anyhow::Result<()> {
        // NSIS installer - run with silent flag
        let output = Command::new(exe_path)
            .args(&["/S", "/D=C:\\Program Files\\EasySSH"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("NSIS installation failed"));
        }

        Ok(())
    }

    /// Use MoveFileEx for delayed replacement on reboot
    pub async fn schedule_replace_on_reboot(source: &Path, dest: &Path) -> anyhow::Result<()> {
        use std::os::windows::ffi::OsStrExt;

        // Convert paths to wide strings (UTF-16)
        let source_wide: Vec<u16> = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let dest_wide: Vec<u16> = dest
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let result = MoveFileExW(
                source_wide.as_ptr(),
                dest_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_DELAY_UNTIL_REBOOT,
            );

            if result == 0 {
                return Err(anyhow::anyhow!(
                    "MoveFileEx failed: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }

        Ok(())
    }

    /// Check if Windows is in S Mode (restricted app installation)
    pub async fn is_windows_s_mode() -> bool {
        // Check registry for S Mode
        let output = Command::new("reg")
            .args(&[
                "query",
                "HKLM\\SYSTEM\\CurrentControlSet\\Control\\CI\\Policy",
                "/v",
                "SkuPolicyRequired",
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                stdout.contains("0x1")
            }
            Err(_) => false,
        }
    }

    /// Get Windows version info
    pub async fn get_windows_version() -> anyhow::Result<(u32, u32, u32)> {
        use windows_sys::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};

        unsafe {
            let mut osvi: OSVERSIONINFOW = std::mem::zeroed();
            osvi.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;

            if GetVersionExW(&mut osvi) == 0 {
                return Err(anyhow::anyhow!("GetVersionEx failed"));
            }

            Ok((osvi.dwMajorVersion, osvi.dwMinorVersion, osvi.dwBuildNumber))
        }
    }
}

#[async_trait]
impl PlatformUpdater for WindowsUpdater {
    fn get_package_extension(&self) -> &'static str {
        if self.use_msi {
            "msi"
        } else {
            "exe"
        }
    }

    async fn install_update(&self, package_path: &Path) -> anyhow::Result<()> {
        let extension = package_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("exe");

        match extension.to_lowercase().as_str() {
            "msi" => self.install_msi(package_path).await,
            "exe" => {
                // Check if NSIS installer
                let file_stem = package_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                if file_stem.contains("setup") || file_stem.contains("install") {
                    self.install_nsis(package_path).await
                } else {
                    self.install_portable(package_path).await
                }
            }
            _ => Err(anyhow::anyhow!("Unsupported package format: {}", extension)),
        }
    }

    async fn restart_application(&self) -> anyhow::Result<()> {
        let current_exe = std::env::current_exe()?;

        Command::new("cmd.exe")
            .args(&[
                "/C",
                "timeout",
                "/t",
                "2",
                "&&",
                "start",
                "",
                current_exe.to_str().unwrap(),
            ])
            .spawn()?;

        std::process::exit(0);
    }

    async fn needs_elevation(&self, _package_path: &Path) -> anyhow::Result<bool> {
        // Check if running as admin by trying to open a privileged token
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_SUCCESS, HANDLE};
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();

            // Open current process token
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return Err(anyhow::anyhow!("Failed to open process token"));
            }

            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut size: u32 = 0;

            // Get elevation info
            let result = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );

            CloseHandle(token);

            if result == 0 {
                return Err(anyhow::anyhow!("Failed to get token information"));
            }

            // TokenIsElevated is 1 if user is admin
            Ok(elevation.TokenIsElevated == 0)
        }
    }

    fn get_current_executable(&self) -> anyhow::Result<std::path::PathBuf> {
        Ok(std::env::current_exe()?)
    }

    async fn verify_package(&self, package_path: &Path) -> anyhow::Result<bool> {
        // On Windows, verify Authenticode signature if available
        let output = Command::new("powershell.exe")
            .args(&[
                "-Command",
                &format!(
                    "Get-AuthenticodeSignature '{}' | Select-Object -ExpandProperty Status",
                    package_path.display()
                ),
            ])
            .output()?;

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(status == "Valid" || status.is_empty()) // Empty means no signature check available
    }
}

/// Replace executable on Windows using delayed move
pub async fn replace_executable_windows(current: &Path, new: &Path) -> anyhow::Result<()> {
    WindowsUpdater::schedule_replace_on_reboot(new, current).await
}

/// Windows update helper - create update task
pub async fn create_update_task(package_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    let task_name = "EasySSH_Update";

    // Create scheduled task to run after current process exits
    let xml_content = format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>EasySSH Auto Update</Description>
  </RegistrationInfo>
  <Settings>
    <AllowHardTerminate>true</AllowHardTerminate>
    <DeleteExpiredTaskAfter>PT0S</DeleteExpiredTaskAfter>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
    </Exec>
  </Actions>
</Task>"#,
        package_path.display()
    );

    let xml_path = std::env::temp_dir().join("easyssh_update_task.xml");
    tokio::fs::write(&xml_path, xml_content).await?;

    // Register task
    Command::new("schtasks.exe")
        .args(&[
            "/Create",
            "/TN",
            task_name,
            "/XML",
            xml_path.to_str().unwrap(),
            "/F", // Force overwrite
        ])
        .output()?;

    // Run task immediately
    Command::new("schtasks.exe")
        .args(&["/Run", "/TN", task_name])
        .output()?;

    Ok(())
}
