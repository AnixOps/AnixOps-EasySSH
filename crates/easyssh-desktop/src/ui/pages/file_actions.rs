use super::*;

impl EasySshApp {
    pub(super) fn files_connection_record(&self) -> Option<Connection> {
        self.files_connection
            .as_ref()
            .and_then(|id| self.config.connections.iter().find(|item| &item.id == id))
            .cloned()
    }

    pub(super) fn refresh_files(&mut self, record_history: bool) {
        let Some(connection) = self.files_connection_record() else {
            self.files_status = "Select a host to browse files.".into();
            return;
        };
        let capabilities = match self.files_capabilities.clone() {
            Some(value) => value,
            None => match self.remote_files.detect_capabilities(&connection) {
                Ok(value) => {
                    self.files_capabilities = Some(value.clone());
                    value
                }
                Err(error) => {
                    self.files_status = format!("Capability detection failed: {error}");
                    return;
                }
            },
        };
        match self.remote_files.list_dir(
            &connection,
            &capabilities,
            &self.files_path,
            self.files_hidden,
        ) {
            Ok(entries) => {
                self.files_entries = entries;
                self.files_status = format!("{} entries", self.files_entries.len());
                if record_history {
                    self.files_history.truncate(self.files_history_index + 1);
                    if self.files_history.last() != Some(&self.files_path) {
                        self.files_history.push(self.files_path.clone());
                    }
                    self.files_history_index = self.files_history.len().saturating_sub(1);
                }
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    pub(super) fn open_files_path(&mut self, path: String) {
        self.files_path = path;
        self.files_path_input = self.files_path.clone();
        self.files_selected = None;
        self.refresh_files(true);
    }

    pub(super) fn files_go_up(&mut self) {
        let path = self.files_path.trim_end_matches('/');
        let parent = path
            .rsplit_once('/')
            .map(|(prefix, _)| prefix)
            .unwrap_or("");
        self.open_files_path(if parent.is_empty() {
            "/".into()
        } else {
            parent.into()
        });
    }

    pub(super) fn files_select_host(&mut self, id: String) {
        self.files_connection = Some(id);
        self.files_capabilities = None;
        self.files_path = "/".into();
        self.files_path_input = "/".into();
        self.files_history = vec!["/".into()];
        self.files_history_index = 0;
        self.refresh_files(false);
    }

    pub(super) fn refresh_local_files(&mut self) {
        let entries = fs::read_dir(&self.local_path)
            .map(|items| {
                items
                    .filter_map(Result::ok)
                    .filter_map(|item| {
                        let metadata = item.metadata().ok()?;
                        Some(LocalEntry {
                            name: item.file_name().to_string_lossy().into_owned(),
                            path: item.path(),
                            is_directory: metadata.is_dir(),
                            size: metadata.len(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.local_entries = entries;
        self.local_entries
            .sort_by_cached_key(|entry| (!entry.is_directory, entry.name.to_lowercase()));
    }

    pub(super) fn queue_local_upload(&mut self, path: &Path) {
        if self.files_connection.is_none() {
            self.files_status = "Select a remote host before uploading.".into();
            return;
        }
        self.transfer_connection = self.files_connection.clone();
        self.transfer_local_path = path.to_string_lossy().into_owned();
        self.transfer_remote_path = self.files_path.clone();
        self.transfer_direction = TransferDirection::Upload;
        self.transfer_recursive = path.is_dir();
        self.config.workspace = Workspace::Transfers;
    }

    pub(super) fn queue_remote_download(&mut self, remote_path: String) {
        self.transfer_connection = self.files_connection.clone();
        self.transfer_local_path = self.local_path.to_string_lossy().into_owned();
        self.transfer_remote_path = remote_path;
        self.transfer_direction = TransferDirection::Download;
        self.transfer_recursive = false;
        self.config.workspace = Workspace::Transfers;
    }

    pub(super) fn remote_operation_context(&self) -> Option<(Connection, RemoteCapabilities)> {
        Some((
            self.files_connection_record()?,
            self.files_capabilities.clone()?,
        ))
    }

    pub(super) fn create_remote_directory(&mut self) {
        let name = self.files_new_dir_name.trim();
        if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
            self.files_status = "Enter a single directory name without path separators.".into();
            return;
        }
        let Some((connection, capabilities)) = self.remote_operation_context() else {
            self.files_status = "Select and refresh a host before creating a directory.".into();
            return;
        };
        let path = remote_child_path(&self.files_path, name);
        match self
            .remote_files
            .create_dir(&connection, &capabilities, &path)
        {
            Ok(()) => {
                self.files_create_dir_open = false;
                self.files_new_dir_name.clear();
                self.refresh_files(false);
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    pub(super) fn rename_selected_remote(&mut self) {
        let name = self.files_rename_name.trim();
        if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
            self.files_status = "Enter a single new name without path separators.".into();
            return;
        }
        let Some(source) = self.files_selected.clone() else {
            return;
        };
        let Some((connection, capabilities)) = self.remote_operation_context() else {
            self.files_status = "Select and refresh a host before renaming.".into();
            return;
        };
        let destination = remote_child_path(&self.files_path, name);
        match self
            .remote_files
            .rename(&connection, &capabilities, &source, &destination)
        {
            Ok(()) => {
                self.files_rename_open = false;
                self.files_selected = None;
                self.refresh_files(false);
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    pub(super) fn delete_selected_remote(&mut self) {
        let Some(entry) = self
            .files_selected
            .as_ref()
            .and_then(|path| self.files_entries.iter().find(|entry| &entry.path == path))
            .cloned()
        else {
            return;
        };
        let Some((connection, capabilities)) = self.remote_operation_context() else {
            self.files_status = "Select and refresh a host before deleting.".into();
            return;
        };
        let result = if entry.entry_type == RemoteEntryType::Directory {
            self.remote_files
                .remove_dir(&connection, &capabilities, &entry.path)
        } else {
            self.remote_files
                .remove_file(&connection, &capabilities, &entry.path)
        };
        match result {
            Ok(()) => {
                self.files_delete_open = false;
                self.files_selected = None;
                self.refresh_files(false);
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }
}
