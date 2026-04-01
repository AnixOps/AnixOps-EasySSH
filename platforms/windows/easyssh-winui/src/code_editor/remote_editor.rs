#![allow(dead_code)]

//! Remote Editor - Stub

use std::collections::HashMap;

/// Remote file editor
pub struct RemoteFileEditor {
    open_files: HashMap<String, RemoteFile>,
}

impl RemoteFileEditor {
    pub fn new() -> Self {
        Self {
            open_files: HashMap::new(),
        }
    }

    pub fn open_file(&mut self, path: &str, content: String) {
        self.open_files.insert(
            path.to_string(),
            RemoteFile {
                path: path.to_string(),
                content,
                modified: false,
            },
        );
    }

    pub fn close_file(&mut self, path: &str) {
        self.open_files.remove(path);
    }

    pub fn get_file(&self, path: &str) -> Option<&RemoteFile> {
        self.open_files.get(path)
    }

    pub fn get_file_mut(&mut self, path: &str) -> Option<&mut RemoteFile> {
        self.open_files.get_mut(path)
    }

    pub fn save_file(&mut self, _path: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Remote file
#[derive(Clone, Debug)]
pub struct RemoteFile {
    pub path: String,
    pub content: String,
    pub modified: bool,
}

/// Auto save timer
#[derive(Clone, Debug)]
pub struct AutoSaveTimer {
    pub file_path: String,
    pub last_edit: std::time::Instant,
}
