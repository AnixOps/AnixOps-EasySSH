use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct UiTestMode {
    pub root: PathBuf,
    pub token: String,
}

impl UiTestMode {
    pub fn from_args() -> Result<Option<Self>, String> {
        let args = std::env::args().collect::<Vec<_>>();
        let Some(index) = args.iter().position(|arg| arg == "--ui-test-root") else {
            return Ok(None);
        };
        let root = PathBuf::from(
            args.get(index + 1)
                .ok_or("--ui-test-root requires a path")?,
        );
        if !root.is_absolute()
            || root
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err("ui-test root must be an absolute normalized path".into());
        }
        for name in ["config", "data", "logs", "screenshots"] {
            fs::create_dir_all(root.join(name)).map_err(|error| error.to_string())?;
        }
        let token = args
            .iter()
            .position(|arg| arg == "--ui-test-token")
            .and_then(|position| args.get(position + 1))
            .cloned()
            .ok_or("--ui-test-token is required")?;
        if token.len() < 16 {
            return Err("ui-test token is too short".into());
        }
        fs::write(
            root.join("metadata.json"),
            format!("{{\"state\":\"starting\",\"token\":\"{}\"}}", token),
        )
        .map_err(|error| error.to_string())?;
        Ok(Some(Self { root, token }))
    }

    pub fn mark_ready(&self) {
        let _ = fs::write(
            self.root.join("metadata.json"),
            format!(
                "{{\"state\":\"ready\",\"token\":\"{}\",\"pid\":{},\"title\":\"EasySSH [UI Test]\",\"width\":1280,\"height\":800}}",
                self.token, std::process::id()
            ),
        );
        self.log("ready");
    }

    pub fn stop_requested(&self) -> bool {
        self.root.join("stop.request").is_file()
    }

    fn log(&self, event: &str) {
        let _ = fs::write(
            self.root.join("logs").join("app.log"),
            format!("ui_test {event}\n"),
        );
    }
}
