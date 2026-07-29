pub mod diagnostics {
    use easyssh_core::{AgentDiagnostics, OpenSsh};
    use eframe::egui;
    use std::sync::mpsc::{self, Receiver};

    #[derive(Debug, Clone)]
    pub enum Status {
        Idle,
        Loading,
        Ready(AgentDiagnostics),
        Failed(String),
    }
    pub struct State {
        pub status: Status,
        receiver: Option<Receiver<Result<AgentDiagnostics, String>>>,
    }
    impl Default for State {
        fn default() -> Self {
            Self {
                status: Status::Idle,
                receiver: None,
            }
        }
    }
    impl State {
        pub fn request(&mut self, ctx: &egui::Context) -> bool {
            if matches!(self.status, Status::Loading) {
                return false;
            }
            let (sender, receiver) = mpsc::channel();
            self.receiver = Some(receiver);
            self.status = Status::Loading;
            let repaint = ctx.clone();
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(|| OpenSsh.diagnostics(None))
                    .map_err(|_| "Diagnostics worker failed.".to_owned());
                let _ = sender.send(result);
                repaint.request_repaint();
            });
            true
        }
        pub fn poll(&mut self) {
            let Some(receiver) = &self.receiver else {
                return;
            };
            if let Ok(result) = receiver.try_recv() {
                self.status = match result {
                    Ok(report) => Status::Ready(report),
                    Err(error) => Status::Failed(redact_failure(&error)),
                };
                self.receiver = None;
            }
        }
        pub fn agent_available(&self) -> Option<bool> {
            match &self.status {
                Status::Ready(report) => report.agent_keys.as_ref().map(|keys| keys.available),
                _ => None,
            }
        }
    }

    fn redact_failure(_: &str) -> String {
        "Diagnostics could not be collected.".into()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn only_one_request_can_be_in_flight() {
            let mut state = State::default();
            let ctx = egui::Context::default();
            assert!(state.request(&ctx));
            assert!(!state.request(&ctx));
        }
        #[test]
        fn failure_text_is_redacted() {
            assert_eq!(
                redact_failure("C:\\Users\\name\\.ssh\\id_ed25519"),
                "Diagnostics could not be collected."
            );
        }
    }
}

pub mod host_form {
    use easyssh_core::security::{validate_connection, ValidationError};
    use easyssh_core::Connection;
    #[derive(Debug, Clone)]
    pub struct State {
        pub initial: Connection,
        pub draft: Connection,
        pub validation: Option<String>,
        pub confirm_discard: bool,
    }
    impl State {
        pub fn existing(connection: Connection) -> Self {
            Self {
                draft: connection.clone(),
                initial: connection,
                validation: None,
                confirm_discard: false,
            }
        }
        pub fn new(connection: Connection) -> Self {
            Self {
                initial: connection.clone(),
                draft: connection,
                validation: None,
                confirm_discard: false,
            }
        }
        pub fn dirty(&self) -> bool {
            self.initial != self.draft
        }
        pub fn validate(&mut self) -> Result<(), ValidationError> {
            let result = validate_connection(&self.draft).map_err(|error| {
                self.validation = Some(error.to_string());
                error
            });
            if result.is_ok() {
                self.validation = None;
            }
            result
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn existing_draft_tracks_changes_without_mutating_original() {
            let original = Connection::alias("Original", "prod");
            let mut form = State::existing(original.clone());
            form.draft.name = "Changed".into();
            assert!(form.dirty());
            assert_eq!(form.initial.name, "Original");
        }

        #[test]
        fn new_draft_tracks_unsaved_changes() {
            let mut form = State::new(Connection::alias("New", "new"));
            form.draft.name = "Changed".into();
            assert!(form.dirty());
        }
    }
}
