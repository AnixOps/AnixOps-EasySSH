//! Server List Component
//!
//! Renders the central server list area using virtual scrolling for large datasets.
//! Features:
//! - Server name, host, and port display
//! - Selection highlighting with theme support
//! - Status indicators with colors
//! - Virtual scrolling for performance with large lists
//! - Theme-aware styling

use crate::app::{App, Focus};
use crate::virtual_list::{ServerListItem, render_virtual_server_list};

pub fn render(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, app: &mut App, theme: &crate::theme::ColorPalette) {
    let is_focused = app.focus == Focus::ServerList;

    // Build server list items from filtered servers
    let items: Vec<ServerListItem> = app
        .filtered_servers
        .iter()
        .enumerate()
        .map(|(display_index, &server_index)| {
            let server = &app.servers[server_index];
            ServerListItem {
                index: display_index,
                name: server.name.clone(),
                connection: format!("{}@{}:{}", server.username, server.host, server.port),
                group: server
                    .group_id
                    .as_ref()
                    .and_then(|gid| {
                        app.groups
                            .iter()
                            .find(|g| &g.id == gid)
                            .map(|g| g.name.clone())
                    })
                    .unwrap_or_else(|| "-".to_string()),
                status: server.status.clone(),
                is_selected: app.selected_server == display_index,
            }
        })
        .collect();

    // Render using virtual list for performance
    render_virtual_server_list(
        frame,
        area,
        &items,
        &mut app.server_list_state,
        is_focused,
        theme,
    );
}
