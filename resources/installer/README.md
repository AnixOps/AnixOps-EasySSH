# EasySSH Windows Installers

This directory contains configuration for building Windows installers for EasySSH.

## Installer Types

### 1. MSI (WiX Toolset)
- **Best for**: Enterprise deployments, automated installations
- **Features**:
  - Windows Installer standard
  - Silent installation support
  - Automatic upgrade handling
  - Add/Remove Programs integration
  - Registry entries

### 2. NSIS (Nullsoft Scriptable Install System)
- **Best for**: Standard users, interactive installations
- **Features**:
  - User-friendly wizard
  - Multiple language support
  - Smaller file size
  - Component selection
  - Desktop shortcut option

### 3. Portable ZIP
- **Best for**: USB drives, restricted systems
- **Features**:
  - No installation required
  - Self-contained
  - Run from any location

## Directory Structure

```
installer/
├── wix/
│   ├── EasySSH.wxs          # WiX source file
│   ├── build/               # Build output (gitignored)
│   ├── *.bmp                # UI images (optional)
│   └── *.rtf                # License file
├── nsis/
│   ├── easyssh.nsi          # NSIS script
│   └── images/              # UI images (optional)
├── resources/               # Shared resources (gitignored)
├── installer.config.json    # Installer configuration
└── README.md               # This file
```

## Building Installers

### Using Bash Script (MSYS2/Git Bash)

```bash
# Default build (version 0.3.0)
./scripts/build-windows-installer.sh

# Custom version
./scripts/build-windows-installer.sh 0.4.0

# Custom source directory
./scripts/build-windows-installer.sh 0.3.0 ./custom/release

# With code signing
export SIGN_CERT="path/to/certificate.pfx"
export SIGN_CERT_PASSWORD="password"
./scripts/build-windows-installer.sh
```

### Using Batch Script (Command Prompt)

```cmd
REM Default build
scripts\build-windows-installer.bat

REM Custom version
scripts\build-windows-installer.bat 0.4.0

REM With code signing
set SIGN_CERT=path\to\certificate.pfx
set SIGN_CERT_PASSWORD=password
scripts\build-windows-installer.bat
```

### Manual Build

#### WiX MSI

```powershell
# Compile
candle.exe -arch x64 -dVersion=0.3.0 -dSourceDir=. -out build/ wix/EasySSH.wxs

# Link
light.exe -ext WixUIExtension -ext WixUtilExtension -cultures:en-US -out EasySSH-0.3.0-x64.msi build/EasySSH.wixobj
```

#### NSIS

```powershell
# Compile
makensis.exe /DPRODUCT_VERSION=0.3.0 /DOUTPUT_NAME=EasySSH-0.3.0-x64.exe nsis/easyssh.nsi
```

## Prerequisites

### Required
- [WiX Toolset v3.11](https://wixtoolset.org/releases/)
- [NSIS 3.0+](https://nsis.sourceforge.io/Download)
- Built `EasySSH.exe` binary

### Optional (for code signing)
- Windows SDK (for `signtool.exe`)
- Code signing certificate (.pfx)

## Silent Installation

### MSI
```powershell
# Silent install
msiexec /i EasySSH-0.3.0-x64.msi /qn INSTALLDESKTOPSHORTCUT=1

# Silent uninstall
msiexec /x EasySSH-0.3.0-x64.msi /qn
```

### NSIS
```powershell
# Silent install
EasySSH-0.3.0-x64.exe /S

# Silent uninstall
%LOCALAPPDATA%\Programs\EasySSH\uninstall.exe /S
```

## Code Signing

### Configure Environment Variables

```bash
export SIGN_CERT="/c/Users/username/certs/code-sign.pfx"
export SIGN_CERT_PASSWORD="your-password"
export SIGN_TIMESTAMP_URL="http://timestamp.digicert.com"
```

### Verify Signature

```powershell
# Check digital signature
Get-AuthenticodeSignature EasySSH-0.3.0-x64.exe

# Verify after installation
Get-AuthenticodeSignature "$env:LOCALAPPDATA\Programs\EasySSH\EasySSH.exe"
```

## Customization

### Installer Configuration

Edit `installer.config.json`:

```json
{
  "installer": {
    "msi": {
      "upgradeCode": "YOUR-UPGRADE-CODE-GUID"
    },
    "nsis": {
      "languages": ["English", "German", "French"]
    }
  },
  "signing": {
    "enabled": true,
    "certificate": "path/to/cert.pfx"
  }
}
```

### UI Images

Create bitmap files and place in appropriate directories:

**WiX:**
- `wix/banner.bmp` - 493x58 pixels
- `wix/dialog.bmp` - 493x312 pixels

**NSIS:**
- `nsis/images/header.bmp` - 150x57 pixels
- `nsis/images/welcome.bmp` - 164x314 pixels

## Troubleshooting

### WiX Build Fails
1. Verify WiX Toolset is installed: `candle.exe /?`
2. Check source directory contains `EasySSH.exe`
3. Ensure all referenced files exist

### NSIS Build Fails
1. Verify NSIS is installed: `makensis.exe /?`
2. Check for missing images (can use placeholders)
3. Verify script syntax

### Code Signing Fails
1. Check certificate path and password
2. Verify `signtool.exe` is in PATH
3. Ensure timestamp server is accessible

### SmartScreen Warning
- Unsigned binaries trigger Windows SmartScreen
- Solution: Purchase and apply code signing certificate
- Users can click "More info" → "Run anyway"

## GitHub Actions Integration

See `.github/workflows/windows-installer.yml` for automated builds:

```yaml
- name: Build Installers
  run: |
    ./scripts/build-windows-installer.sh ${{ github.ref_name }}
  env:
    SIGN_CERT: ${{ secrets.CODE_SIGN_CERT }}
    SIGN_CERT_PASSWORD: ${{ secrets.CODE_SIGN_PASSWORD }}
```

## References

- [WiX Toolset Documentation](https://wixtoolset.org/documentation/)
- [NSIS Manual](https://nsis.sourceforge.io/Docs/)
- [Microsoft Code Signing](https://docs.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-cert-manage)
