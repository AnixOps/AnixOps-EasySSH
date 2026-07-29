use crate::domain::{Connection, ConnectionTarget};
use crate::openssh::{OpenSsh, OpenSshError};
use crate::security::validate_path;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Queued,
    Pending,
    Authorizing,
    Transferring,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub direction: TransferDirection,
    pub local_path: PathBuf,
    pub remote_path: String,
    pub recursive: bool,
    pub status: TransferStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub output: String,
}
impl Transfer {
    pub fn new(
        direction: TransferDirection,
        local_path: PathBuf,
        remote_path: String,
        recursive: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            direction,
            local_path,
            remote_path,
            recursive,
            status: TransferStatus::Pending,
            started_at: None,
            finished_at: None,
            output: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScpInvocation {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

/// A batch-only directory listing through the system OpenSSH `sftp` client.
/// This deliberately supports a conservative remote path subset so untrusted
/// metadata cannot become additional batch commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftpListingInvocation {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub batch: String,
}

impl SftpListingInvocation {
    pub fn build(
        openssh: &OpenSsh,
        connection: &Connection,
        path: &str,
    ) -> Result<Self, OpenSshError> {
        validate_sftp_path(path)?;
        let mut args = vec!["-b".into(), "-".into()];
        if let ConnectionTarget::Endpoint { port, .. } = &connection.target {
            args.extend(["-P".into(), port.to_string()]);
        }
        args.push(sftp_target(connection));
        Ok(Self {
            executable: openssh.sftp_path()?,
            args,
            batch: format!("ls -l {path}\n"),
        })
    }

    pub fn output(&self) -> Result<String, OpenSshError> {
        let output = Command::new(&self.executable)
            .args(&self.args)
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
                    .write_all(self.batch.as_bytes())?;
                child.wait_with_output()
            })?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(OpenSshError::Failed(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }
}
impl ScpInvocation {
    pub fn build(
        openssh: &OpenSsh,
        connection: &Connection,
        transfer: &Transfer,
    ) -> Result<Self, OpenSshError> {
        validate_path(&transfer.local_path.to_string_lossy(), "local path")
            .map_err(OpenSshError::Validation)?;
        validate_path(&transfer.remote_path, "remote path").map_err(OpenSshError::Validation)?;
        let remote = remote_spec(connection, &transfer.remote_path)?;
        let mut args = Vec::new();
        if transfer.recursive {
            args.push("-r".into());
        }
        if let ConnectionTarget::Endpoint { port, .. } = &connection.target {
            args.extend(["-P".into(), port.to_string()]);
        }
        args.push("--".into());
        match transfer.direction {
            TransferDirection::Upload => {
                args.push(transfer.local_path.to_string_lossy().into_owned());
                args.push(remote);
            }
            TransferDirection::Download => {
                args.push(remote);
                args.push(transfer.local_path.to_string_lossy().into_owned());
            }
        }
        Ok(Self {
            executable: openssh.scp_path()?,
            args,
        })
    }
    pub fn spawn(&self) -> Result<Child, OpenSshError> {
        Ok(Command::new(&self.executable)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?)
    }
}

fn remote_spec(connection: &Connection, path: &str) -> Result<String, OpenSshError> {
    let host = match &connection.target {
        ConnectionTarget::Alias { alias } => alias.clone(),
        ConnectionTarget::Endpoint {
            hostname, username, ..
        } => match username {
            Some(username) => format!("{username}@{hostname}"),
            None => hostname.clone(),
        },
    };
    let host = if host.contains('@') {
        let (user, hostname) = host.split_once('@').expect("validated user/host");
        if hostname.contains(':') && !hostname.starts_with('[') {
            format!("{user}@[{hostname}]")
        } else {
            host
        }
    } else if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host
    };
    Ok(format!("{host}:{path}"))
}

fn sftp_target(connection: &Connection) -> String {
    match &connection.target {
        ConnectionTarget::Alias { alias } => alias.clone(),
        ConnectionTarget::Endpoint {
            hostname, username, ..
        } => username
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|username| format!("{username}@{hostname}"))
            .unwrap_or_else(|| hostname.clone()),
    }
}

fn validate_sftp_path(path: &str) -> Result<(), OpenSshError> {
    validate_path(path, "remote path").map_err(OpenSshError::Validation)?;
    if !path
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return Err(OpenSshError::Failed(
            "remote directory paths may contain only letters, numbers, '/', '.', '_' and '-'"
                .into(),
        ));
    }
    Ok(())
}

pub fn cancel(child: &mut Child) -> std::io::Result<()> {
    child.kill()
}

pub fn local_path_is_directory(path: &Path) -> bool {
    path.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Connection, ConnectionTarget};
    #[test]
    fn scp_uses_uppercase_port_and_argument_separator() {
        let connection = Connection {
            target: ConnectionTarget::Endpoint {
                hostname: "2001:db8::1".into(),
                username: Some("ops".into()),
                port: 2222,
            },
            ..Connection::alias("x", "unused")
        };
        let transfer = Transfer::new(
            TransferDirection::Upload,
            PathBuf::from("C:/My File.txt"),
            "/srv/app".into(),
            false,
        );
        let open = OpenSsh;
        if let Ok(command) = ScpInvocation::build(&open, &connection, &transfer) {
            assert_eq!(command.args[0..3], ["-P", "2222", "--"]);
            assert!(command
                .args
                .last()
                .unwrap()
                .starts_with("ops@[2001:db8::1]:"));
        }
    }
}
