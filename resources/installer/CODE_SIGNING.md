# Code Signing Configuration for EasySSH

## Overview

Code signing ensures users can verify the authenticity and integrity of EasySSH installers and binaries.

## Certificate Options

### 1. Standard Code Signing Certificate
- **Provider**: DigiCert, Sectigo, Certum
- **Cost**: $200-500/year
- **Validation**: Organization verification
- **SmartScreen**: Reputation builds over time

### 2. Extended Validation (EV) Code Signing
- **Provider**: DigiCert, Sectigo
- **Cost**: $500-800/year
- **Validation**: Extended organization verification
- **SmartScreen**: Immediate reputation

### 3. Azure Key Vault (Cloud HSM)
- **Provider**: Microsoft Azure
- **Cost**: ~$1/key + $0.03/10,000 operations
- **Security**: Hardware security module
- **Convenience**: No local certificate management

## Configuration

### Environment Variables

```powershell
# Windows Command Prompt
set SIGN_CERT=C:\Users\username\certs\easyssh.pfx
set SIGN_CERT_PASSWORD=your-secure-password
set SIGN_TIMESTAMP_URL=http://timestamp.digicert.com

# PowerShell
$env:SIGN_CERT = "C:\Users\username\certs\easyssh.pfx"
$env:SIGN_CERT_PASSWORD = "your-secure-password"
$env:SIGN_TIMESTAMP_URL = "http://timestamp.digicert.com"

# Git Bash / MSYS2
export SIGN_CERT="/c/Users/username/certs/easyssh.pfx"
export SIGN_CERT_PASSWORD="your-secure-password"
export SIGN_TIMESTAMP_URL="http://timestamp.digicert.com"
```

### Certificate Storage

#### Local Machine (Development)
```powershell
# Import certificate to Personal store
Import-PfxCertificate -FilePath .\easyssh.pfx -CertStoreLocation Cert:\CurrentUser\My

# Reference by thumbprint in build scripts
set SIGN_CERT_THUMBPRINT=ABC123DEF456...
```

#### Azure Key Vault (CI/CD)
```yaml
# GitHub Actions
- name: Sign Binary
  uses: azure/azure-sign-action@v0
  with:
    azure-tenant-id: ${{ secrets.AZURE_TENANT_ID }}
    azure-client-id: ${{ secrets.AZURE_CLIENT_ID }}
    azure-client-secret: ${{ secrets.AZURE_CLIENT_SECRET }}
    key-vault-url: ${{ secrets.KEY_VAULT_URL }}
    key-vault-certificate-name: easyssh-signing-cert
    file-path: target/release/EasySSH.exe
```

## Signing Process

### Manual Signing

```powershell
# Sign executable
signtool.exe sign `
  /f C:\certs\easyssh.pfx `
  /p password `
  /tr http://timestamp.digicert.com `
  /td sha256 `
  /fd sha256 `
  /d "EasySSH - Native SSH Client" `
  target\release\EasySSH.exe

# Verify signature
signtool.exe verify /pa target\release\EasySSH.exe
```

### Batch Signing (All Binaries)

```powershell
# Sign all release binaries
$binaries = @(
    "target\release\EasySSH.exe",
    "releases\v0.3.0\windows\EasySSH-0.3.0-x64.msi",
    "releases\v0.3.0\windows\EasySSH-0.3.0-x64.exe"
)

foreach ($binary in $binaries) {
    signtool.exe sign `
        /f C:\certs\easyssh.pfx `
        /p $env:SIGN_CERT_PASSWORD `
        /tr http://timestamp.digicert.com `
        /td sha256 `
        /fd sha256 `
        /d "EasySSH v0.3.0" `
        $binary

    signtool.exe verify /pa $binary
}
```

## Verification

### PowerShell

```powershell
# Check signature
Get-AuthenticodeSignature -FilePath EasySSH.exe | Format-List *

# Check all files in directory
Get-ChildItem -Path . -Include *.exe,*.msi | ForEach-Object {
    $sig = Get-AuthenticodeSignature $_.FullName
    [PSCustomObject]@{
        File = $_.Name
        Status = $sig.Status
        Signer = $sig.SignerCertificate.Subject
        Timestamp = $sig.TimeStamperCertificate.NotAfter
    }
}
```

### Command Line

```powershell
# Using signtool
signtool.exe verify /pa EasySSH.exe
signtool.exe verify /v EasySSH.exe

# Using PowerShell cmdlet
powershell -Command "Get-AuthenticodeSignature EasySSH.exe"
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Sign Windows Release

on:
  release:
    types: [created]

jobs:
  sign:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download Certificate
        run: |
          echo "${{ secrets.CODE_SIGN_CERT }}" | base64 -d > cert.pfx

      - name: Sign Binaries
        shell: pwsh
        run: |
          $signtool = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe"

          & $signtool sign `
            /f cert.pfx `
            /p ${{ secrets.CODE_SIGN_PASSWORD }} `
            /tr http://timestamp.digicert.com `
            /td sha256 `
            /fd sha256 `
            /d "EasySSH" `
            target/release/EasySSH.exe

          & $signtool verify /pa target/release/EasySSH.exe
```

### Azure DevOps

```yaml
steps:
- task: DownloadSecureFile@1
  name: signingCert
  inputs:
    secureFile: 'easyssh-signing-cert.pfx'

- script: |
    signtool.exe sign /f $(signingCert.secureFilePath) /p $(CERT_PASSWORD) /tr http://timestamp.digicert.com /fd sha256 target/release/EasySSH.exe
  displayName: 'Sign Binary'
```

## Troubleshooting

### Certificate Not Found
```powershell
# List certificates in Personal store
Get-ChildItem Cert:\CurrentUser\My

# List certificates in Local Machine store
Get-ChildItem Cert:\LocalMachine\My
```

### Expired Certificate
```powershell
# Check certificate expiration
$cert = Get-PfxCertificate -FilePath .\easyssh.pfx
$cert.NotAfter

# Renew certificate before expiration
# Update SIGN_CERT and SIGN_CERT_PASSWORD
```

### Timestamp Server Unavailable
```powershell
# Alternative timestamp servers
# http://timestamp.globalsign.com/scripts/timestamp.dll
# http://timestamp.sectigo.com
# http://tsa.starfieldtech.com
```

### SmartScreen Warnings

Even with valid code signing, SmartScreen may show warnings for:
- New certificates (reputation building)
- Low download counts
- Rare file types

**Solutions:**
1. Submit to Microsoft for malware analysis
2. Wait for reputation to build (downloads + time)
3. Use EV certificate for immediate trust
4. Distribute through Microsoft Store

## Security Best Practices

1. **Never commit certificates to git**
   - Use environment variables
   - Use secure CI/CD secret storage

2. **Protect private keys**
   - Use HSM or Azure Key Vault
   - Limit access to signing infrastructure

3. **Monitor certificate expiration**
   - Set up alerts 60 days before expiry
   - Automate renewal if possible

4. **Audit signing operations**
   - Log all signing events
   - Review access logs regularly

## Resources

- [Microsoft Code Signing Guide](https://docs.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-cert-manage)
- [DigiCert Code Signing](https://www.digicert.com/code-signing/)
- [Azure Key Vault Documentation](https://docs.microsoft.com/en-us/azure/key-vault/)
