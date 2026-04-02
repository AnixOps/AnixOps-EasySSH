# Troubleshooting Guide

## Error Code Reference

| Code | Description | Common Causes | Solution |
|------|-------------|---------------|----------|
| E001 | Connection timeout | Network issues, firewall, server down | Check network, firewall, SSH service |
| E002 | Authentication failed | Wrong password, invalid key, user not found | Check credentials, key permissions, username |
| E003 | Host key changed | Server reinstalled, MITM attack | Update known_hosts or verify security |
| E004 | Network unreachable | DNS failure, routing issues | Check DNS, network configuration |
| E005 | Connection refused | SSH service not running, wrong port | Check SSH service and port |
| E006 | Invalid master password | Forgot master password | Use password manager or reset data |
| E007 | Database corruption | Disk failure, unexpected shutdown | Restore from backup |
| E008 | Encryption failed | Key file corruption | Reconfigure encryption |
| E009 | Permission denied | File permission errors | Fix permission settings |
| E010 | Out of memory | System resources exhausted | Close other apps or upgrade hardware |

## Connection Issues

### E001 - Connection Timeout

**Symptoms:**
```
Error E001: Connection timed out after 30 seconds
```

**Diagnosis:**

1. **Test basic connectivity**
   ```bash
   ping <hostname>
   # If fails: Check DNS or network

   telnet <hostname> 22
   # If fails: Check SSH port and firewall
   ```

2. **Check SSH service**
   ```bash
   # On target server
   sudo systemctl status sshd  # systemd
   sudo service ssh status     # init.d

   # Check listening port
   sudo netstat -tlnp | grep sshd
   # or
   sudo ss -tlnp | grep sshd
   ```

3. **Check firewall**
   ```bash
   # Local firewall
   sudo iptables -L | grep 22

   # Cloud provider security groups (AWS, Azure, GCP)
   # Check security group settings in console
   ```

**Solutions:**

```bash
# Increase timeout
easyssh config set ssh.timeout 60
easyssh config set ssh.connection-timeout 30
```

### E002 - Authentication Failed

**Symptoms:**
```
Error E002: Authentication failed
Permission denied (publickey,password)
```

**Fix key permissions:**
```bash
chmod 600 ~/.ssh/id_rsa
chmod 700 ~/.ssh
```

**Convert key format:**
```bash
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa
```

## Authentication Issues

### Key Format Incompatibility

**Symptoms:**
```
Error: Key type not supported
Load key "id_rsa": invalid format
```

**Solution:**
```bash
# Convert key format
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

# Or generate new format key (recommended)
ssh-keygen -t ed25519 -a 100 -f ~/.ssh/id_ed25519
```

### SSH Agent Issues

**Symptoms:**
```
Error: Could not open a connection to your authentication agent
```

**Solution:**
```bash
# Start Agent
eval $(ssh-agent -s)  # Linux
eval $(ssh-agent)     # macOS

# Add key
ssh-add ~/.ssh/id_rsa
```

## Performance Issues

### Terminal Lag

**Solutions:**
```bash
# Disable WebGL (if GPU issues)
easyssh config set terminal.webgl false

# Reduce scrollback buffer
easyssh config set terminal.scrollback 10000

# Limit frame rate
easyssh config set terminal.render-fps 30
```

### High CPU Usage

```bash
# Reduce polling frequency
easyssh config set ui.refresh-rate 5000  # 5 seconds

# Disable unnecessary features
easyssh config set terminal.cursor-blink false
```

## Data Issues

### Database Corruption (E007)

**Solutions:**
```bash
# Restore from backup
cp ~/.easyssh/backups/data-2026-01-15.enc ~/.easyssh/data.enc

# Or export and rebuild
easyssh export --format json --output backup.json
rm ~/.easyssh/data.enc
easyssh --init
easyssh import --source json --file backup.json
```

### Forgot Master Password

::: danger Data Cannot Be Recovered
Lite/Standard use local encryption. Forgetting the master password means permanent data loss.
:::

**Prevention:**
```bash
# Regular backups
easyssh export --format json --output backup-$(date +%Y%m%d).json
```

## Reset and Recovery

### Soft Reset (Keep Data)

```bash
# Reset configuration to defaults
easyssh config reset

# Reset layout (Standard)
easyssh layout reset
```

### Hard Reset (Delete All Data)

::: danger Warning
This will permanently delete all data!
:::

```bash
# Export backup first (if needed)
easyssh export --format json --output final-backup.json

# Close the application

# Delete data directory
# macOS:
rm -rf ~/Library/Application\ Support/EasySSH/

# Windows:
rmdir /s %APPDATA%\EasySSH

# Linux:
rm -rf ~/.config/easyssh/

# Restart the application
```

## Getting Help

If the issue persists:

1. Enable verbose logging:
   ```bash
   easyssh --verbose --log-level trace
   ```

2. Collect logs from:
   - macOS: `~/Library/Application Support/EasySSH/logs/`
   - Windows: `%APPDATA%\EasySSH\logs\`
   - Linux: `~/.config/easyssh/logs/`

3. Submit a report:
   - GitHub Issues: https://github.com/anixops/easyssh/issues
   - Email: support@easyssh.dev
   - Discord: https://discord.gg/easyssh
