use super::*;

#[derive(Clone)]
pub(super) enum CommandAction {
    NewHost,
    OpenSync,
    Switch(Workspace),
    Host(String, String),
    Connect(String, String, bool),
    Snippet(String, String),
    Session(String, String),
}
impl CommandAction {
    pub(super) fn label(&self) -> String {
        match self {
            Self::NewHost => format!("{} New host", icon::PLUS),
            Self::OpenSync => format!("{} Open Git metadata sync", icon::ARROWS_CLOCKWISE),
            Self::Switch(Workspace::Home) => "Go to Home".into(),
            Self::Switch(Workspace::Hosts) => "Go to Hosts".into(),
            Self::Switch(Workspace::Files) => "Go to Files".into(),
            Self::Switch(Workspace::Snippets) => "Go to Snippets".into(),
            Self::Switch(Workspace::Forwarding) => "Go to Port forwarding".into(),
            Self::Switch(Workspace::Transfers) => "Go to Transfers".into(),
            Self::Switch(Workspace::Keys) => "Go to Keys".into(),
            Self::Switch(Workspace::Settings) => "Go to Settings".into(),
            Self::Host(_, name) => format!("Open host: {name}"),
            Self::Connect(_, name, false) => format!("Connect: {name}"),
            Self::Connect(_, name, true) => format!("Detailed log: {name}"),
            Self::Snippet(_, name) => format!("Copy snippet: {name}"),
            Self::Session(_, name) => format!("Reconnect session: {name}"),
        }
    }
}

pub(super) fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    crate::ui::components::icon_button(ui, glyph, tooltip)
}
pub(super) fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(10.0);
    ui.label(egui::RichText::new(label).small().strong().color(BLUE));
}
pub(super) fn detail(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(value);
        });
    });
}
pub(super) fn edit_lines(ui: &mut egui::Ui, values: &mut Vec<String>) {
    let mut text = values.join("\n");
    if ui
        .add_sized(
            [ui.available_width(), 46.0],
            egui::TextEdit::multiline(&mut text).desired_rows(2),
        )
        .changed()
    {
        *values = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
    }
}
pub(super) fn target_text(host: &Connection) -> String {
    match &host.target {
        ConnectionTarget::Alias { alias } => alias.clone(),
        ConnectionTarget::Endpoint {
            hostname,
            username,
            port,
        } => match username.as_deref().filter(|u| !u.is_empty()) {
            Some(user) => format!("{user}@{hostname}:{port}"),
            None => format!("{hostname}:{port}"),
        },
    }
}

/// Parses the common `user@host:port` form without retaining raw command text.
pub(super) fn parse_quick_target(
    value: &str,
    fallback_user: &str,
    fallback_port: u16,
) -> (Option<String>, String, u16) {
    let value = value.trim();
    let (user, endpoint) = value
        .rsplit_once('@')
        .map_or((None, value), |(user, endpoint)| {
            (
                (!user.trim().is_empty()).then(|| user.trim().to_owned()),
                endpoint,
            )
        });
    let (hostname, port) = endpoint
        .rsplit_once(':')
        .and_then(|(host, port)| {
            port.parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .map(|port| (host, port))
        })
        .unwrap_or((endpoint, fallback_port));
    (
        user.or_else(|| {
            (!fallback_user.trim().is_empty()).then(|| fallback_user.trim().to_owned())
        }),
        hostname.trim().to_owned(),
        port,
    )
}
pub(super) fn relative_time(time: DateTime<Utc>) -> String {
    let seconds = (Utc::now() - time).num_seconds().max(0);
    if seconds < 60 {
        "now".into()
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86_400)
    }
}
pub(super) fn contains(value: &str, query: &str) -> bool {
    query.trim().is_empty() || value.to_lowercase().contains(&query.trim().to_lowercase())
}
pub(super) fn host_matches(host: &Connection, query: &str, group: &str) -> bool {
    contains(
        &format!(
            "{} {} {} {}",
            host.name,
            target_text(host),
            host.tags.join(" "),
            group
        ),
        query,
    )
}
pub(super) fn single_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().to_owned()
}
pub(super) fn count_forwards(values: &[String]) -> String {
    if values.is_empty() {
        "None".into()
    } else {
        values.len().to_string()
    }
}

pub(super) fn format_bytes(size: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = size as f64;
    let mut index = 0;
    while value >= 1024.0 && index + 1 < UNITS.len() {
        value /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{} {}", size, UNITS[index])
    } else {
        format!("{value:.1} {}", UNITS[index])
    }
}

pub(super) fn remote_temporary_sibling(path: &str, session_id: &str) -> String {
    let (directory, name) = path.rsplit_once('/').unwrap_or((".", path));
    let directory = if directory.is_empty() { "/" } else { directory };
    format!("{directory}/.easyssh-{name}-{session_id}.tmp")
}

pub(super) fn remote_child_path(directory: &str, name: &str) -> String {
    if directory == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", directory.trim_end_matches('/'), name)
    }
}

pub(super) fn is_previewable_image(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
            )
        })
}

pub(super) fn sync_status_label(status: SyncStatus) -> &'static str {
    match status {
        SyncStatus::Unconfigured => "Sync: not configured",
        SyncStatus::Clean => "Sync: clean",
        SyncStatus::LocalChanges => "Sync: local changes",
        SyncStatus::RemoteUpdates => "Sync: remote updates",
        SyncStatus::Conflict => "Sync: conflict",
        SyncStatus::Failed => "Sync: unavailable",
    }
}

pub(super) fn transfer_status_badge(status: TransferStatus) -> (&'static str, egui::Color32) {
    match status {
        TransferStatus::Queued => ("Queued", AMBER),
        TransferStatus::Pending => ("Pending", AMBER),
        TransferStatus::Authorizing => ("Authorizing", AMBER),
        TransferStatus::Transferring => ("Transferring", BLUE),
        TransferStatus::Completed => ("Completed", GREEN),
        TransferStatus::Failed => ("Failed", RED),
        TransferStatus::Cancelled => ("Cancelled", AMBER),
        TransferStatus::Interrupted => ("Interrupted", RED),
    }
}

#[allow(dead_code)]
pub(super) fn apply_theme(ctx: &egui::Context, theme: Theme, density: DisplayDensity) {
    let scale = match density {
        DisplayDensity::Compact => 0.9,
        DisplayDensity::Comfortable => 1.0,
        DisplayDensity::Large => 1.15,
    };
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(16.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(16.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(22.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(13.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(15.0 * scale),
    );
    style.spacing.interact_size.y = 32.0 * scale;
    style.spacing.button_padding = egui::vec2(8.0 * scale, 4.0 * scale);
    ctx.set_style(style);
    let dark = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
    };
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = if dark {
        egui::Color32::from_rgb(18, 20, 27)
    } else {
        egui::Color32::from_rgb(246, 247, 251)
    };
    visuals.window_fill = if dark {
        egui::Color32::from_rgb(30, 33, 43)
    } else {
        egui::Color32::WHITE
    };
    visuals.extreme_bg_color = visuals.window_fill;
    visuals.faint_bg_color = if dark {
        egui::Color32::from_rgb(25, 28, 38)
    } else {
        egui::Color32::from_rgb(236, 238, 246)
    };
    visuals.selection.bg_fill = BLUE.gamma_multiply(0.6);
    visuals.widgets.hovered.bg_stroke.color = BLUE;
    visuals.widgets.active.bg_stroke.color = BLUE;
    visuals.hyperlink_color = BLUE;
    visuals.window_rounding = egui::Rounding::same(6.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    ctx.set_visuals(visuals);
}
