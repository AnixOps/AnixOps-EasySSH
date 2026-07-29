use super::*;

impl EasySshApp {
    pub(super) fn open_remote_text_file(&mut self, path: String) {
        if !self.config.experimental.remote_text_editing {
            self.files_status = "Remote editing is disabled in Experimental settings.".into();
            return;
        }
        let Some(connection) = self.files_connection_record() else {
            self.files_status = "Select a host to open a file.".into();
            return;
        };
        let Some(capabilities) = self.files_capabilities.clone() else {
            self.files_status = "Refresh the directory before opening a file.".into();
            return;
        };
        let mut initial_remote = match self.remote_files.stat(&connection, &capabilities, &path) {
            Ok(state) => state,
            Err(error) => {
                self.files_status = error.to_string();
                return;
            }
        };
        if capabilities.has_sha256 {
            initial_remote.sha256 = self
                .remote_files
                .calculate_hash(&connection, &capabilities, &path)
                .ok();
        }
        let workspace = match self.temporary_workspace.as_ref() {
            Some(workspace) => workspace,
            None => match WorkspaceTempManager::create() {
                Ok(workspace) => {
                    self.temporary_workspace = Some(workspace);
                    self.temporary_workspace
                        .as_ref()
                        .expect("workspace inserted")
                }
                Err(error) => {
                    self.files_status = format!("Unable to create local working copy: {error}");
                    return;
                }
            },
        };
        let mut session = match workspace.create_session(&path, initial_remote) {
            Ok(session) => session,
            Err(error) => {
                self.files_status = format!("Unable to create local working copy: {error}");
                return;
            }
        };
        let transfer = Transfer::new(
            TransferDirection::Download,
            session.local_path.clone(),
            path,
            false,
        );
        let result = ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp);
        if let Err(error) = result {
            self.files_status = format!("Download failed: {error}");
            return;
        }
        let bytes = match fs::read(&session.local_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.files_status = format!("Unable to read downloaded working copy: {error}");
                return;
            }
        };
        if bytes.len() > 2 * 1024 * 1024 {
            self.files_status = "File is larger than the 2 MiB editor protection limit.".into();
            return;
        }
        session.file_info = easyssh_core::inspect_text(&bytes);
        if session.file_info.is_binary {
            self.files_status = "Binary file downloaded but not opened in the text editor.".into();
            return;
        }
        let content = match String::from_utf8(bytes) {
            Ok(content) => content,
            Err(_) => {
                self.files_status = "The file is not UTF-8 text; use Download instead.".into();
                return;
            }
        };
        self.file_editor_text = content;
        if let Err(error) = fs::copy(&session.local_path, &session.base_path) {
            self.files_status = format!("Unable to preserve initial working copy: {error}");
            return;
        }
        self.file_edit_session = Some(session);
        self.file_editor_status = FileEditorStatus::SavedLocally;
        self.file_editor_find.clear();
        self.file_editor_replace.clear();
        self.file_editor_match_index = 0;
        self.file_editor_go_to_line = 1;
        self.file_editor_cursor_line = 1;
        self.file_editor_pending_selection = None;
        self.file_editor_external_change = None;
        self.file_editor_last_modified = self
            .file_edit_session
            .as_ref()
            .and_then(|session| fs::metadata(&session.local_path).ok())
            .and_then(|metadata| metadata.modified().ok());
        self.files_status = "Opened a local working copy downloaded with system scp.".into();
    }

    pub(super) fn open_remote_image_preview(&mut self, path: String) {
        if !self.config.experimental.image_preview {
            self.files_status = "Image preview is disabled in Experimental settings.".into();
            return;
        }
        let Some(connection) = self.files_connection_record() else {
            self.files_status = "Select a host to preview a file.".into();
            return;
        };
        let Some(capabilities) = self.files_capabilities.clone() else {
            self.files_status = "Refresh the directory before previewing a file.".into();
            return;
        };
        let state = match self.remote_files.stat(&connection, &capabilities, &path) {
            Ok(state) => state,
            Err(error) => {
                self.files_status = error.to_string();
                return;
            }
        };
        if state.size > 5 * 1024 * 1024 {
            self.files_status = "Image preview is limited to files smaller than 5 MiB.".into();
            return;
        }
        let workspace = match self.temporary_workspace.as_ref() {
            Some(workspace) => workspace,
            None => match WorkspaceTempManager::create() {
                Ok(workspace) => {
                    self.temporary_workspace = Some(workspace);
                    self.temporary_workspace
                        .as_ref()
                        .expect("workspace inserted")
                }
                Err(error) => {
                    self.files_status = format!("Unable to create preview copy: {error}");
                    return;
                }
            },
        };
        let session = match workspace.create_session(&path, state) {
            Ok(session) => session,
            Err(error) => {
                self.files_status = format!("Unable to create preview copy: {error}");
                return;
            }
        };
        let transfer = Transfer::new(
            TransferDirection::Download,
            session.local_path.clone(),
            path.clone(),
            false,
        );
        let result = ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp);
        if let Err(error) = result {
            self.files_status = format!("Preview download failed: {error}");
            return;
        }
        let bytes = match fs::read(&session.local_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.files_status = format!("Unable to read preview copy: {error}");
                return;
            }
        };
        match image::load_from_memory(&bytes) {
            Ok(image) => {
                let image = image.to_rgba8();
                let dimensions = [image.width() as usize, image.height() as usize];
                self.file_preview_image = Some((
                    path,
                    egui::ColorImage::from_rgba_unmultiplied(dimensions, image.as_raw()),
                ));
                self.file_preview_texture = None;
                self.files_status = "Image preview downloaded with system scp.".into();
            }
            Err(error) => self.files_status = format!("Unsupported image: {error}"),
        }
    }

    pub(super) fn open_editor_in_system_app(&mut self) {
        let Some(session) = &self.file_edit_session else {
            return;
        };
        let result = if cfg!(windows) {
            Command::new("explorer.exe")
                .arg(&session.local_path)
                .spawn()
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(&session.local_path).spawn()
        } else {
            Command::new("xdg-open").arg(&session.local_path).spawn()
        };
        if let Err(error) = result {
            self.files_status = format!("Unable to open the local working copy: {error}");
        }
    }

    pub(super) fn save_editor_locally(&mut self) -> bool {
        let Some(session) = &self.file_edit_session else {
            return false;
        };
        let text = match session.file_info.line_ending {
            LineEnding::Lf => self.file_editor_text.clone(),
            LineEnding::Crlf => self
                .file_editor_text
                .replace("\r\n", "\n")
                .replace('\n', "\r\n"),
        };
        match fs::write(&session.local_path, text) {
            Ok(()) => {
                self.file_editor_status = FileEditorStatus::SavedLocally;
                self.file_editor_last_modified = fs::metadata(&session.local_path)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok());
                true
            }
            Err(error) => {
                self.file_editor_status = FileEditorStatus::UploadFailed;
                self.files_status = format!("Unable to save local working copy: {error}");
                false
            }
        }
    }

    pub(super) fn editor_text_for_local_copy(&self) -> String {
        let line_ending = self
            .file_edit_session
            .as_ref()
            .map(|session| session.file_info.line_ending)
            .unwrap_or(LineEnding::Lf);
        match line_ending {
            LineEnding::Lf => self.file_editor_text.clone(),
            LineEnding::Crlf => self
                .file_editor_text
                .replace("\r\n", "\n")
                .replace('\n', "\r\n"),
        }
    }

    pub(super) fn find_editor_matches(&self) -> Vec<(usize, usize)> {
        if self.file_editor_find.is_empty() {
            return Vec::new();
        }
        self.file_editor_text
            .match_indices(&self.file_editor_find)
            .map(|(start, matched)| {
                let start = self.file_editor_text[..start].chars().count();
                (start, start + matched.chars().count())
            })
            .collect()
    }

    pub(super) fn select_editor_match(&mut self, backwards: bool) {
        let matches = self.find_editor_matches();
        if matches.is_empty() {
            self.file_editor_match_index = 0;
            self.files_status = "No matching text in this working copy.".into();
            return;
        }
        if backwards {
            self.file_editor_match_index = if self.file_editor_match_index == 0 {
                matches.len() - 1
            } else {
                self.file_editor_match_index - 1
            };
        } else {
            self.file_editor_match_index = (self.file_editor_match_index + 1) % matches.len();
        }
        self.file_editor_pending_selection = Some(matches[self.file_editor_match_index]);
    }

    pub(super) fn replace_all_editor_matches(&mut self) {
        if self.file_editor_find.is_empty() {
            return;
        }
        let count = self
            .file_editor_text
            .matches(&self.file_editor_find)
            .count();
        if count == 0 {
            self.files_status = "No matching text to replace.".into();
            return;
        }
        self.file_editor_text = self
            .file_editor_text
            .replace(&self.file_editor_find, &self.file_editor_replace);
        self.file_editor_status = FileEditorStatus::LocalModified;
        self.file_editor_match_index = 0;
        self.files_status = format!("Replaced {count} match(es) in the local working copy.");
    }

    pub(super) fn go_to_editor_line(&mut self) {
        let line_count = self.file_editor_text.lines().count().max(1);
        let target_line = self.file_editor_go_to_line.clamp(1, line_count);
        self.file_editor_go_to_line = target_line;
        let character_index = self
            .file_editor_text
            .split_inclusive('\n')
            .take(target_line.saturating_sub(1))
            .map(|line| line.chars().count())
            .sum();
        self.file_editor_pending_selection = Some((character_index, character_index));
    }

    pub(super) fn check_external_editor_change(&mut self) {
        let Some(session) = self.file_edit_session.as_ref() else {
            return;
        };
        let Some(modified) = fs::metadata(&session.local_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
        else {
            return;
        };
        if self.file_editor_last_modified == Some(modified) {
            return;
        }
        self.file_editor_last_modified = Some(modified);
        let Ok(bytes) = fs::read(&session.local_path) else {
            return;
        };
        let info = easyssh_core::inspect_text(&bytes);
        if info.is_binary {
            self.files_status =
                "External editor wrote unsupported non-UTF-8 or binary content.".into();
            return;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            self.files_status =
                "External editor wrote unsupported non-UTF-8 or binary content.".into();
            return;
        };
        if text == self.editor_text_for_local_copy() {
            return;
        }
        if self.file_editor_status == FileEditorStatus::LocalModified {
            self.file_editor_external_change = Some(text);
            self.files_status =
                "External editor changed the working copy; choose which local version to keep."
                    .into();
        } else {
            self.file_editor_text = text;
            self.file_editor_status = FileEditorStatus::SavedLocally;
            self.files_status =
                "External editor saved the local working copy. Upload changes when ready.".into();
        }
    }

    pub(super) fn download_remote_text_for_conflict(
        &mut self,
        connection: &Connection,
        remote_path: &str,
        state: easyssh_core::RemoteFileState,
    ) -> Option<String> {
        let workspace = self.temporary_workspace.as_ref()?;
        let session = workspace.create_session(remote_path, state).ok()?;
        let transfer = Transfer::new(
            TransferDirection::Download,
            session.local_path.clone(),
            remote_path.to_owned(),
            false,
        );
        ScpInvocation::build(&self.openssh, connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp)
            .ok()?;
        let bytes = fs::read(session.local_path).ok()?;
        if bytes.len() > 2 * 1024 * 1024 || easyssh_core::inspect_text(&bytes).is_binary {
            return None;
        }
        String::from_utf8(bytes).ok()
    }
}
