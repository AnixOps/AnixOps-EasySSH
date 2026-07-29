use crate::remote::RemoteFileState;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFileInfo {
    pub line_ending: LineEnding,
    pub has_utf8_bom: bool,
    pub is_binary: bool,
}

#[derive(Debug, Clone)]
pub struct EditSession {
    pub id: String,
    pub remote_path: String,
    pub local_path: PathBuf,
    pub base_path: PathBuf,
    pub initial_remote: RemoteFileState,
    pub file_info: LocalFileInfo,
}

#[derive(Debug, Clone)]
pub struct WorkspaceTempManager {
    root: PathBuf,
}

impl WorkspaceTempManager {
    pub fn create() -> io::Result<Self> {
        let root = std::env::temp_dir().join(format!("easyssh-work-{}", Uuid::new_v4()));
        fs::create_dir_all(&root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn new_copy(
        &self,
        remote_path: &str,
        content: &[u8],
        initial_remote: RemoteFileState,
    ) -> io::Result<EditSession> {
        let mut session = self.create_session(remote_path, initial_remote)?;
        fs::write(&session.local_path, content)?;
        session.file_info = inspect_text(content);
        Ok(session)
    }

    pub fn create_session(
        &self,
        remote_path: &str,
        initial_remote: RemoteFileState,
    ) -> io::Result<EditSession> {
        let id = Uuid::new_v4().to_string();
        let local_path = self.root.join(&id).with_extension("work");
        let base_path = self.root.join(&id).with_extension("base");
        fs::File::create(&local_path)?;
        fs::File::create(&base_path)?;
        Ok(EditSession {
            id,
            remote_path: remote_path.to_owned(),
            local_path,
            base_path,
            initial_remote,
            file_info: inspect_text(&[]),
        })
    }

    pub fn cleanup(&self) -> io::Result<()> {
        if self.root.exists() {
            fs::remove_dir_all(&self.root)?;
        }
        Ok(())
    }
}

impl Drop for WorkspaceTempManager {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

pub fn inspect_text(content: &[u8]) -> LocalFileInfo {
    let has_utf8_bom = content.starts_with(&[0xef, 0xbb, 0xbf]);
    let is_binary = content.contains(&0) || std::str::from_utf8(content).is_err();
    let line_ending = if content.windows(2).any(|pair| pair == b"\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    };
    LocalFileInfo {
        line_ending,
        has_utf8_bom,
        is_binary,
    }
}

pub fn remote_state_changed(initial: &RemoteFileState, current: &RemoteFileState) -> bool {
    initial.size != current.size
        || initial.modified_epoch != current.modified_epoch
        || initial.sha256.is_some() && initial.sha256 != current.sha256
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bom_line_endings_and_binary_content() {
        let info = inspect_text(b"\xef\xbb\xbffirst\r\nsecond\r\n");
        assert!(info.has_utf8_bom);
        assert_eq!(info.line_ending, LineEnding::Crlf);
        assert!(!info.is_binary);
        assert!(inspect_text(b"text\0binary").is_binary);
    }

    #[test]
    fn compares_remote_state_without_content_logging() {
        let initial = RemoteFileState {
            size: 1,
            modified_epoch: 2,
            sha256: None,
        };
        let current = RemoteFileState {
            size: 2,
            modified_epoch: 2,
            sha256: None,
        };
        assert!(remote_state_changed(&initial, &current));
    }
}
