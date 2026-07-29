use crate::domain::Connection;
use crate::openssh::{OpenSsh, OpenSshError, SshInvocation};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemoteFileError {
    #[error(transparent)]
    OpenSsh(#[from] OpenSshError),
    #[error("system ssh process failed: {0}")]
    Process(#[from] std::io::Error),
    #[error("remote path is empty or contains a NUL byte")]
    UnsafePath,
    #[error("remote platform is unsupported: {0}")]
    UnsupportedPlatform(String),
    #[error("remote command returned invalid structured output")]
    InvalidOutput,
    #[error("remote command failed: {0}")]
    Failed(String),
    #[error("remote filename is not valid UTF-8")]
    NonUtf8Filename,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemotePlatformKind {
    Posix,
    Windows,
    Unsupported,
}

pub trait RemotePlatformAdapter: Send + Sync {
    fn kind(&self) -> RemotePlatformKind;
    fn supports_file_operations(&self) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PosixRemoteAdapter;
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsRemoteAdapter;
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedRemoteAdapter;

impl RemotePlatformAdapter for PosixRemoteAdapter {
    fn kind(&self) -> RemotePlatformKind {
        RemotePlatformKind::Posix
    }
    fn supports_file_operations(&self) -> bool {
        true
    }
}
impl RemotePlatformAdapter for WindowsRemoteAdapter {
    fn kind(&self) -> RemotePlatformKind {
        RemotePlatformKind::Windows
    }
    fn supports_file_operations(&self) -> bool {
        false
    }
}
impl RemotePlatformAdapter for UnsupportedRemoteAdapter {
    fn kind(&self) -> RemotePlatformKind {
        RemotePlatformKind::Unsupported
    }
    fn supports_file_operations(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCapabilities {
    pub platform: RemotePlatformKind,
    pub uname: String,
    pub shell: String,
    pub has_find: bool,
    pub has_stat: bool,
    pub has_sha256: bool,
    pub has_mv: bool,
    pub has_chmod: bool,
    pub has_readlink: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteEntryType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEntry {
    pub path: String,
    pub name: String,
    pub entry_type: RemoteEntryType,
    pub size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFileState {
    pub size: u64,
    pub modified_epoch: i64,
    pub sha256: Option<String>,
}

/// The only core entry point for file operations performed through system ssh.
/// User-controlled paths are encoded before being included in a fixed shell script.
#[derive(Debug, Clone)]
pub struct RemoteFileService {
    openssh: OpenSsh,
}

impl RemoteFileService {
    pub fn new(openssh: OpenSsh) -> Self {
        Self { openssh }
    }

    pub fn detect_capabilities(
        &self,
        connection: &Connection,
    ) -> Result<RemoteCapabilities, RemoteFileError> {
        let output = self.run_script(connection, capability_script())?;
        let fields = split_nul(&output)?;
        if fields.len() != 9 {
            return Err(RemoteFileError::InvalidOutput);
        }
        let uname = fields[0].clone();
        let platform = if uname.eq_ignore_ascii_case("windows_nt") {
            RemotePlatformKind::Windows
        } else if fields[1] == "1" && fields[2] == "1" {
            RemotePlatformKind::Posix
        } else {
            RemotePlatformKind::Unsupported
        };
        Ok(RemoteCapabilities {
            platform,
            uname,
            shell: fields[8].clone(),
            has_find: fields[1] == "1",
            has_stat: fields[2] == "1",
            has_sha256: fields[3] == "1",
            has_mv: fields[4] == "1",
            has_chmod: fields[5] == "1",
            has_readlink: fields[6] == "1",
        })
    }

    pub fn list_dir(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
        include_hidden: bool,
    ) -> Result<Vec<RemoteEntry>, RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        let path = encoded(path);
        let hidden = if include_hidden { "1" } else { "0" };
        let script = format!(
            "{}\npath=$(decode '{path}')\nhidden={hidden}\n{}",
            posix_prelude(),
            POSIX_LIST_BODY
        );
        parse_entries(&self.run_script(connection, script)?)
    }

    pub fn stat(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
    ) -> Result<RemoteFileState, RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        let script = format!(
            "{}\npath=$(decode '{}')\n{}",
            posix_prelude(),
            encoded(path),
            POSIX_STATE_BODY
        );
        let fields = split_nul(&self.run_script(connection, script)?)?;
        if fields.len() != 2 {
            return Err(RemoteFileError::InvalidOutput);
        }
        Ok(RemoteFileState {
            size: fields[0]
                .parse()
                .map_err(|_| RemoteFileError::InvalidOutput)?,
            modified_epoch: fields[1]
                .parse()
                .map_err(|_| RemoteFileError::InvalidOutput)?,
            sha256: None,
        })
    }

    pub fn read_file(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
        max_bytes: usize,
    ) -> Result<Vec<u8>, RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        let script = format!(
            "{}\npath=$(decode '{}')\nif [ \"$(wc -c < \"$path\")\" -gt {max_bytes} ]; then exit 75; fi\ncat -- \"$path\"",
            posix_prelude(), encoded(path)
        );
        self.run_script(connection, script)
    }

    /// Replaces a regular remote file with a same-directory temporary upload.
    /// The destination is never overwritten when it is a symbolic link.
    pub fn atomic_replace(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        temporary_path: &str,
        destination_path: &str,
    ) -> Result<(), RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(temporary_path)?;
        validate_remote_path(destination_path)?;
        let script = format!(
            "{}\ntemporary=$(decode '{}')\ndestination=$(decode '{}')\ncleanup() {{ rm -f -- \"$temporary\"; }}\n[ ! -L \"$destination\" ] || {{ cleanup; exit 64; }}\nif [ -e \"$destination\" ]; then if stat -c '%a' -- \"$destination\" >/dev/null 2>&1; then mode=$(stat -c '%a' -- \"$destination\"); else mode=$(stat -f '%Lp' -- \"$destination\"); fi; chmod -- \"$mode\" \"$temporary\" || {{ cleanup; exit 1; }}; fi\nmv -- \"$temporary\" \"$destination\" || {{ cleanup; exit 1; }}",
            posix_prelude(),
            encoded(temporary_path),
            encoded(destination_path)
        );
        self.run_script(connection, script).map(|_| ())
    }

    pub fn create_dir(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
    ) -> Result<(), RemoteFileError> {
        self.run_mutation(connection, capabilities, path, "mkdir -- \"$path\"")
    }

    pub fn remove_file(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
    ) -> Result<(), RemoteFileError> {
        self.run_mutation(connection, capabilities, path, "rm -- \"$path\"")
    }

    pub fn remove_dir(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
    ) -> Result<(), RemoteFileError> {
        self.run_mutation(connection, capabilities, path, "rm -r -- \"$path\"")
    }

    pub fn set_permissions(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
        mode: &str,
    ) -> Result<(), RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        if mode.is_empty() || !mode.chars().all(|character| character.is_ascii_digit()) {
            return Err(RemoteFileError::UnsafePath);
        }
        let script = format!(
            "{}\npath=$(decode '{}')\nchmod -- {mode} \"$path\"",
            posix_prelude(),
            encoded(path)
        );
        self.run_script(connection, script).map(|_| ())
    }

    pub fn rename(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        source: &str,
        destination: &str,
    ) -> Result<(), RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(source)?;
        validate_remote_path(destination)?;
        let script = format!("{}\nsource=$(decode '{}')\ndestination=$(decode '{}')\nmv -- \"$source\" \"$destination\"", posix_prelude(), encoded(source), encoded(destination));
        self.run_script(connection, script).map(|_| ())
    }

    pub fn move_path(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        source: &str,
        destination: &str,
    ) -> Result<(), RemoteFileError> {
        self.rename(connection, capabilities, source, destination)
    }

    pub fn move_file(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        source: &str,
        destination: &str,
    ) -> Result<(), RemoteFileError> {
        self.move_path(connection, capabilities, source, destination)
    }

    pub fn read_link(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
    ) -> Result<String, RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        let script = format!(
            "{}\npath=$(decode '{}')\nreadlink -- \"$path\"",
            posix_prelude(),
            encoded(path)
        );
        let output = self.run_script(connection, script)?;
        String::from_utf8(output)
            .map(|text| text.trim_end_matches(['\r', '\n']).to_owned())
            .map_err(|_| RemoteFileError::NonUtf8Filename)
    }

    pub fn copy(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        source: &str,
        destination: &str,
    ) -> Result<(), RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(source)?;
        validate_remote_path(destination)?;
        let script = format!("{}\nsource=$(decode '{}')\ndestination=$(decode '{}')\ncp -p -- \"$source\" \"$destination\"", posix_prelude(), encoded(source), encoded(destination));
        self.run_script(connection, script).map(|_| ())
    }

    pub fn calculate_hash(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
    ) -> Result<String, RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        let script = format!("{}\npath=$(decode '{}')\nif command -v sha256sum >/dev/null 2>&1; then sha256sum -- \"$path\" | awk '{{print $1}}'; else shasum -a 256 -- \"$path\" | awk '{{print $1}}'; fi", posix_prelude(), encoded(path));
        let output = self.run_script(connection, script)?;
        let hash = String::from_utf8(output).map_err(|_| RemoteFileError::InvalidOutput)?;
        let hash = hash.trim();
        if hash.len() == 64 && hash.chars().all(|character| character.is_ascii_hexdigit()) {
            Ok(hash.to_owned())
        } else {
            Err(RemoteFileError::InvalidOutput)
        }
    }

    fn run_mutation(
        &self,
        connection: &Connection,
        capabilities: &RemoteCapabilities,
        path: &str,
        body: &str,
    ) -> Result<(), RemoteFileError> {
        self.require_posix(capabilities)?;
        validate_remote_path(path)?;
        let script = format!(
            "{}\npath=$(decode '{}')\n{body}",
            posix_prelude(),
            encoded(path)
        );
        self.run_script(connection, script).map(|_| ())
    }

    fn require_posix(&self, capabilities: &RemoteCapabilities) -> Result<(), RemoteFileError> {
        if capabilities.platform == RemotePlatformKind::Posix {
            Ok(())
        } else {
            Err(RemoteFileError::UnsupportedPlatform(
                capabilities.uname.clone(),
            ))
        }
    }

    fn run_script(
        &self,
        connection: &Connection,
        script: String,
    ) -> Result<Vec<u8>, RemoteFileError> {
        let invocation =
            SshInvocation::for_fixed_remote_command(&self.openssh, connection, "sh -s")?;
        let output = Command::new(invocation.executable)
            .args(invocation.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .expect("piped stdin")
                    .write_all(script.as_bytes())?;
                child.wait_with_output()
            })?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            Err(RemoteFileError::Failed(if message.is_empty() {
                "system ssh exited unsuccessfully".into()
            } else {
                message
            }))
        }
    }
}

pub fn validate_remote_path(path: &str) -> Result<(), RemoteFileError> {
    if path.is_empty() || path.contains('\0') {
        Err(RemoteFileError::UnsafePath)
    } else {
        Ok(())
    }
}

fn encoded(value: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value.as_bytes();
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(TABLE[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(third & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    output
}

fn split_nul(output: &[u8]) -> Result<Vec<String>, RemoteFileError> {
    let mut fields = output
        .split(|byte| *byte == 0)
        .map(|field| {
            String::from_utf8(field.to_vec()).map_err(|_| RemoteFileError::NonUtf8Filename)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.last().is_some_and(String::is_empty) {
        fields.pop();
    }
    Ok(fields)
}

fn parse_entries(output: &[u8]) -> Result<Vec<RemoteEntry>, RemoteFileError> {
    let fields = split_nul(output)?;
    if fields.len() % 9 != 0 {
        return Err(RemoteFileError::InvalidOutput);
    }
    fields
        .chunks_exact(9)
        .map(|field| {
            let kind = match field[2].as_str() {
                "directory" => RemoteEntryType::Directory,
                "symlink" => RemoteEntryType::Symlink,
                "file" => RemoteEntryType::File,
                _ => RemoteEntryType::Other,
            };
            let name = field[1].clone();
            Ok(RemoteEntry {
                path: field[0].clone(),
                name,
                entry_type: kind,
                size: field[3]
                    .parse()
                    .map_err(|_| RemoteFileError::InvalidOutput)?,
                modified_at: field[4]
                    .parse::<i64>()
                    .ok()
                    .and_then(|epoch| DateTime::from_timestamp(epoch, 0)),
                permissions: field[5].clone(),
                owner: field[6].clone(),
                group: field[7].clone(),
                link_target: (!field[8].is_empty()).then(|| field[8].clone()),
            })
        })
        .collect()
}

fn posix_prelude() -> &'static str {
    "set -eu\ndecode() { printf '%s' \"$1\" | base64 -D 2>/dev/null || printf '%s' \"$1\" | base64 -d; }"
}

fn capability_script() -> String {
    "printf '%s\\0' \"$(uname -s 2>/dev/null || printf unknown)\"\nfor tool in find stat sha256sum mv chmod readlink; do if command -v \"$tool\" >/dev/null 2>&1; then printf '1\\0'; else printf '0\\0'; fi; done\nprintf '%s\\0' \"${SHELL:-sh}\"".into()
}

const POSIX_LIST_BODY: &str = r#"
case "$path" in -*) path=./$path ;; esac
export hidden
find "$path" -mindepth 1 -maxdepth 1 -exec sh -c '
  for item do
    name=${item##*/}
    if [ "$hidden" != 1 ]; then case "$name" in .*) continue ;; esac; fi
    case "$(stat -c %F -- "$item" 2>/dev/null || stat -f %HT -- "$item")" in
      directory|Directory) kind=directory ;;
      *link*|*Link*) kind=symlink ;;
      *file*|*File*) kind=file ;;
      *) kind=other ;;
    esac
    if stat -c %s -- "$item" >/dev/null 2>&1; then
      size=$(stat -c %s -- "$item")
      modified=$(stat -c %Y -- "$item")
      permissions=$(stat -c %A -- "$item")
      owner=$(stat -c %U -- "$item")
      group=$(stat -c %G -- "$item")
    else
      size=$(stat -f %z -- "$item")
      modified=$(stat -f %m -- "$item")
      permissions=$(stat -f %Sp -- "$item")
      owner=$(stat -f %Su -- "$item")
      group=$(stat -f %Sg -- "$item")
    fi
    link_target=
    if [ "$kind" = symlink ]; then link_target=$(readlink -- "$item" 2>/dev/null || readlink "$item"); fi
    printf "%s\0%s\0%s\0%s\0%s\0%s\0%s\0%s\0%s\0" "$item" "$name" "$kind" "$size" "$modified" "$permissions" "$owner" "$group" "$link_target"
  done
' sh {} +
"#;

const POSIX_STATE_BODY: &str = r#"
if stat -c '%s\0%Y\0' -- "$path" >/dev/null 2>&1; then stat -c '%s\0%Y\0' -- "$path"; else stat -f '%z\0%m\0' -- "$path"; fi
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_paths_never_escape_single_quoted_payload() {
        for path in [
            "space name",
            "quote'\"$;*",
            "-leading",
            "line\nbreak",
            "中文😀",
        ] {
            let value = encoded(path);
            assert!(value
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || matches!(character, '+' | '/' | '=')));
        }
    }

    #[test]
    fn remote_paths_allow_shell_sensitive_and_leading_dash_names() {
        assert!(validate_remote_path("/tmp/-name;$(x)\n中文").is_ok());
        assert!(validate_remote_path("bad\0path").is_err());
    }

    #[test]
    fn parses_nul_structured_entry_without_ls_output() {
        let output =
            b"/srv/a\x00a\x00file\x0012\x001700000000\x00-rw-r--r--\x00root\x00root\x00\x00";
        let entries = parse_entries(output).unwrap();
        assert_eq!(entries[0].name, "a");
        assert_eq!(entries[0].size, 12);
    }

    #[test]
    fn parses_newline_names_and_link_targets_from_nul_records() {
        let output = b"/srv/a\nname\x00a\nname\x00symlink\x001\x001700000000\x00lrwxrwxrwx\x00root\x00root\x00target file\x00";
        let entries = parse_entries(output).unwrap();
        assert_eq!(entries[0].name, "a\nname");
        assert_eq!(entries[0].link_target.as_deref(), Some("target file"));
    }

    #[test]
    fn directory_listing_uses_find_argument_boundaries() {
        assert!(POSIX_LIST_BODY.contains("find \"$path\" -mindepth 1 -maxdepth 1 -exec"));
        assert!(POSIX_LIST_BODY.contains("for item do"));
        assert!(!POSIX_LIST_BODY.contains("for item in $pattern"));
    }
}
