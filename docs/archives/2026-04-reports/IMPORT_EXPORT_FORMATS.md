# Import/Export Format Examples

## 1. JSON Export Format

```json
{
  "version": "1.0",
  "exported_at": "1711920000",
  "app_version": "0.3.0",
  "servers": [
    {
      "id": "srv-1",
      "name": "Production Server",
      "host": "192.168.1.100",
      "port": 22,
      "username": "admin",
      "auth_type": "key",
      "identity_file": "~/.ssh/id_rsa",
      "group_id": "grp-1",
      "group_name": "Production",
      "status": "active",
      "tags": ["prod", "critical"]
    }
  ],
  "groups": [
    {
      "id": "grp-1",
      "name": "Production",
      "parent_id": null
    }
  ],
  "hosts": [],
  "identities": [
    {
      "id": "id-1",
      "name": "My SSH Key",
      "private_key_path": "~/.ssh/id_rsa",
      "auth_type": "key"
    }
  ],
  "snippets": [
    {
      "id": "snp-1",
      "name": "Update System",
      "command": "sudo apt update && sudo apt upgrade -y",
      "description": "Update Ubuntu system",
      "folder_id": null,
      "scope": "personal",
      "tags": ["maintenance"]
    }
  ],
  "tags": [
    {
      "id": "tag-1",
      "name": "production",
      "color": "#ff0000",
      "description": "Production servers"
    }
  ],
  "settings": null
}
```

## 2. Encrypted JSON Format

```json
{
  "version": "1.0",
  "salt": "a1b2c3d4e5f6...",
  "data": "base64-encoded-encrypted-content..."
}
```

The `data` field contains AES-256-GCM encrypted JSON content.
Decryption requires the master password used during export.

## 3. CSV Export Format

```csv
name,host,port,username,auth_type,identity_file,group,tags
Production Server,192.168.1.100,22,admin,key,~/.ssh/id_rsa,Production,"prod,critical"
Staging Server,192.168.1.101,22,admin,password,,Staging,"staging"
Dev Server,192.168.1.102,22,developer,agent,,Development,"dev"
```

## 4. SSH Config Export Format

```ssh-config
# EasySSH Configuration Export
# Generated at: 1711920000
#

# Group: Production
Host Production Server
    HostName 192.168.1.100
    Port 22
    User admin
    IdentityFile ~/.ssh/id_rsa
    StrictHostKeyChecking accept-new

Host Staging Server
    HostName 192.168.1.101
    Port 22
    User admin
    StrictHostKeyChecking accept-new

# Ungrouped Servers
Host Dev Server
    HostName 192.168.1.102
    Port 22
    User developer
    ForwardAgent yes
    StrictHostKeyChecking accept-new
```

## Supported Import Formats

### Auto-Detect
The system automatically detects format based on:
- `.json` extension → JSON format
- `.csv` extension → CSV format
- Contains `Host` directive → SSH config format
- Contains `salt` and `data` fields → Encrypted JSON

### SSH Config Import
Parse existing `~/.ssh/config` files:

```ssh-config
# Personal servers
Host myserver
    HostName 192.168.1.50
    Port 2222
    User admin
    IdentityFile ~/.ssh/id_rsa

Host *.example.com
    User deploy
    ForwardAgent yes
```

Imported as:
- Server name: `myserver`
- Host: `192.168.1.50`
- Port: `2222`
- Username: `admin`
- Auth: SSH key with identity file

## Conflict Resolution Examples

### Skip Strategy (Default)
```
Existing: 192.168.1.100 admin
Import:   192.168.1.100 admin (different name)
Result:   Keep existing, skip import
```

### Overwrite Strategy
```
Existing: 192.168.1.100 admin
Import:   192.168.1.100 admin (different name)
Result:   Replace existing with import
```

### Merge Strategy
```
Existing: 192.168.1.100 admin (tags: [prod])
Import:   192.168.1.100 admin (tags: [production])
Result:   Keep existing, add new tags [prod, production]
```

## Security Notes

### Encryption Details
- **Algorithm**: AES-256-GCM
- **Key Derivation**: Argon2id (memory-hard)
- **Salt**: Random 32 bytes per export
- **Nonce**: Random 12 bytes per encryption

### Password Requirements
- Minimum 8 characters recommended
- No complexity requirements (user responsibility)
- Password used only for key derivation
- Original password not stored anywhere

## Cloud Sync JSON (Pro Feature)

```json
{
  "device_id": "device-uuid",
  "last_sync": "1711920000",
  "checksum": "sha256-hash",
  "data": "<encrypted-config-data>"
}
```
