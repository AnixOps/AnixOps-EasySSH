# Windows Installer Configuration Summary

## Created Files

### WiX MSI Configuration
- **Path**: `installer/wix/EasySSH.wxs`
- **Type**: Windows Installer Package (MSI)
- **Features**:
  - Per-user and per-machine installation
  - Automatic upgrade handling
  - Desktop shortcut option
  - Start Menu integration
  - Add/Remove Programs entry
  - Windows 10+ version check

### NSIS Installer Script
- **Path**: `installer/nsis/easyssh.nsi`
- **Type**: Nullsoft Scriptable Installer (EXE)
- **Features**:
  - Multi-language support (English, Chinese)
  - Interactive installation wizard
  - Component selection
  - Silent installation support
  - Portable mode option
  - Automatic previous version removal

### Build Scripts
- **Path**: `scripts/build-windows-installer.sh` (Bash/Git Bash)
- **Path**: `scripts/build-windows-installer.bat` (Windows CMD)
- **Features**:
  - Automated MSI and NSIS builds
  - Code signing integration
  - Portable ZIP creation
  - Checksum generation
  - Batch and Bash support

### Release Script
- **Path**: `scripts/release-windows.sh`
- **Features**:
  - Full release automation
  - Build, test, sign, package
  - GitHub release draft creation

### Configuration Files
- **Path**: `installer/installer.config.json`
- **Path**: `installer/CODE_SIGNING.md`
- **Path**: `installer/README.md`

## Installer Types Comparison

| Feature | MSI (WiX) | NSIS (EXE) | Portable ZIP |
|---------|-----------|------------|--------------|
| **Size** | Larger | Smaller | Smallest |
| **Installation** | Standard Windows | Custom wizard | None |
| **Silent Install** | Yes (`/qn`) | Yes (`/S`) | N/A |
| **Upgrade** | Automatic | Manual | N/A |
| **Uninstall** | Add/Remove Programs | Uninstall.exe | Delete folder |
| **Best For** | Enterprise | End users | USB/Restricted |

## Build Instructions

### Prerequisites
1. **WiX Toolset v3.11+**: https://wixtoolset.org/releases/
2. **NSIS 3.0+**: https://nsis.sourceforge.io/Download
3. **Windows SDK** (optional, for code signing)

### Quick Build

```bash
# Using Bash (MSYS2/Git Bash)
cd scripts
./build-windows-installer.sh 0.3.0

# Using Windows CMD
scripts\build-windows-installer.bat 0.3.0
```

### Manual Build

**WiX MSI:**
```powershell
cd installer/wix
candle.exe -arch x64 -dVersion=0.3.0 -dSourceDir=. -out build/ EasySSH.wxs
light.exe -ext WixUIExtension -out EasySSH-0.3.0-x64.msi build/EasySSH.wixobj
```

**NSIS:**
```powershell
cd installer/nsis
makensis.exe /DPRODUCT_VERSION=0.3.0 easyssh.nsi
```

## Code Signing Configuration

### Environment Variables
```bash
export SIGN_CERT="/c/Users/username/certs/code-sign.pfx"
export SIGN_CERT_PASSWORD="your-password"
export SIGN_TIMESTAMP_URL="http://timestamp.digicert.com"
```

### Certificate Options
- **Standard Code Signing**: $200-500/year
- **EV Code Signing**: $500-800/year (immediate SmartScreen trust)
- **Azure Key Vault**: Cloud-based signing

## Silent Installation

### MSI
```powershell
# Silent install with desktop shortcut
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

## Output Structure

```
releases/v0.3.0/windows/
├── EasySSH-0.3.0-x64.msi          # WiX MSI installer
├── EasySSH-0.3.0-x64.exe          # NSIS installer
├── EasySSH-0.3.0-portable.zip     # Portable ZIP
├── SHA256SUMS.txt                 # Checksums
├── INSTALL.md                     # Installation guide
└── RELEASE_NOTES.md               # Release notes
```

## Testing Checklist

- [ ] Test MSI on Windows 10/11
- [ ] Test NSIS on Windows 10/11
- [ ] Test silent installation
- [ ] Test uninstallation
- [ ] Verify shortcuts are created
- [ ] Verify registry entries
- [ ] Test portable version
- [ ] Verify code signing (if configured)
- [ ] Check SmartScreen warning (unsigned)

## CI/CD Integration

### GitHub Actions
```yaml
- name: Build Windows Installers
  run: |
    ./scripts/build-windows-installer.sh ${{ github.ref_name }}
  env:
    SIGN_CERT: ${{ secrets.CODE_SIGN_CERT }}
    SIGN_CERT_PASSWORD: ${{ secrets.CODE_SIGN_PASSWORD }}
```

## Next Steps

1. **Install Prerequisites**: WiX Toolset and NSIS
2. **Create UI Images**: Add custom bitmaps for branded installer
3. **Purchase Certificate**: Obtain code signing certificate
4. **Test Installers**: Run on clean Windows VMs
5. **Update CI/CD**: Add installer builds to GitHub Actions
6. **Document Process**: Add to CONTRIBUTING.md

## Support

- **WiX Documentation**: https://wixtoolset.org/documentation/
- **NSIS Manual**: https://nsis.sourceforge.io/Docs/
- **Code Signing**: See `installer/CODE_SIGNING.md`
