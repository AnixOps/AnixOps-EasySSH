use super::*;

impl EasySshApp {
    pub(super) fn upload_editor_changes(&mut self) {
        let Some(session) = self.file_edit_session.clone() else {
            return;
        };
        let Some(connection) = self.files_connection_record() else {
            self.file_editor_status = FileEditorStatus::UploadFailed;
            return;
        };
        let Some(capabilities) = self.files_capabilities.clone() else {
            self.file_editor_status = FileEditorStatus::UploadFailed;
            return;
        };
        if !self.save_editor_locally() {
            return;
        }
        self.file_editor_status = FileEditorStatus::CheckingRemote;
        let mut current =
            match self
                .remote_files
                .stat(&connection, &capabilities, &session.remote_path)
            {
                Ok(state) => state,
                Err(error) => {
                    self.file_editor_status = FileEditorStatus::Conflict;
                    self.files_status =
                        format!("Remote file is no longer safe to overwrite: {error}");
                    return;
                }
            };
        if session.initial_remote.sha256.is_some() {
            current.sha256 = self
                .remote_files
                .calculate_hash(&connection, &capabilities, &session.remote_path)
                .ok();
        }
        if easyssh_core::remote_state_changed(&session.initial_remote, &current) {
            self.file_editor_status = FileEditorStatus::Conflict;
            self.file_conflict_remote =
                self.download_remote_text_for_conflict(&connection, &session.remote_path, current);
            self.file_conflict_open = true;
            self.files_status =
                "Remote file changed since it was downloaded. Local changes were preserved.".into();
            return;
        }
        let temporary_path = remote_temporary_sibling(&session.remote_path, &session.id);
        let transfer = Transfer::new(
            TransferDirection::Upload,
            session.local_path.clone(),
            temporary_path.clone(),
            false,
        );
        self.file_editor_status = FileEditorStatus::Uploading;
        let result = ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp)
            .and_then(|_| {
                self.remote_files
                    .atomic_replace(
                        &connection,
                        &capabilities,
                        &temporary_path,
                        &session.remote_path,
                    )
                    .map_err(|error| easyssh_core::OpenSshError::Failed(error.to_string()))
            });
        match result {
            Ok(()) => {
                self.file_editor_status = FileEditorStatus::SavedRemotely;
                self.files_status = "Saved remotely using a same-directory temporary file.".into();
                self.refresh_files(false);
            }
            Err(error) => {
                self.file_editor_status = FileEditorStatus::UploadFailed;
                self.files_status =
                    format!("Upload failed; the local working copy was retained: {error}");
            }
        }
    }
}
