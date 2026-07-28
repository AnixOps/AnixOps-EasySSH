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
    Pending,
    Authorizing,
    Transferring,
    Completed,
    Failed,
    Cancelled,
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
    Ok(format!("{host}:{path}"))
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
            assert!(command.args.last().unwrap().starts_with("ops@2001:db8::1:"));
        }
    }
}
