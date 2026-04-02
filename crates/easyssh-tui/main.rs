//! EasySSH TUI - Terminal User Interface for EasySSH Lite
//!
//! This crate provides a terminal-based user interface for managing SSH connections.
//! Built with ratatui and crossterm for cross-platform terminal control.

mod app;
mod events;
mod keybindings;
mod ui;

use app::{App, AppResult};
use events::{Event, EventHandler};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use ui::Ui;

#[tokio::main]
async fn main() -> AppResult<()> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    // Initialize terminal
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create application state
    let mut app = App::new()?;
    app.init().await?;

    // Create event handler
    let mut events = EventHandler::new(250);

    // Create UI renderer
    let mut ui = Ui::new();

    // Main loop
    while app.running {
        // Draw UI
        terminal.draw(|f| ui.render(f, &mut app))?;

        // Handle events
        match events.next().await? {
            Event::Tick => app.tick().await,
            Event::Key(key) => app.handle_key(key).await,
            Event::Mouse(mouse) => app.handle_mouse(mouse).await,
            Event::Resize(w, h) => app.handle_resize(w, h).await,
        }?;
    }

    // Restore terminal
    app.cleanup()?;

    Ok(())
}
