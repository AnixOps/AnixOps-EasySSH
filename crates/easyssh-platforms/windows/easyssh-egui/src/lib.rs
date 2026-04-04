//! EasySSH Standard Edition - egui-based Embedded Terminal for Windows
//!
//! This crate provides a high-performance embedded terminal UI component
//! using pure Rust with egui framework for the Standard edition.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │              EasySSHApp (app.rs)            │
//! │   - Terminal Tabs Management                │
//! │   - Sidebar / Connection List              │
//! │   - Search Panel                           │
//! └─────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────┐
//! │           TerminalView (terminal/view.rs)   │
//! │   - Key: {connection_id}-{session_id}      │
//! │   - Buffer Management                      │
//! │   - Input/Output Handling                  │
//! │   - Selection & Search                     │
//! └─────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────┐
//! │          TerminalBuffer (buffer.rs)        │
//! │   - VecDeque<TermLine>                     │
//! │   - FIFO scrollback (max 10000 lines)      │
//! │   - Cell-based styling                     │
//! └─────────────────────────────────────────────┘
//!                      │
//!                      ▼
//! ┌─────────────────────────────────────────────┐
//! │         TerminalRenderer (renderer.rs)     │
//! │   - egui Painter                           │
//! │   - Font metrics                           │
//! │   - Color scheme                           │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! # Key-Driven Reset Pattern
//!
//! All terminal views use a unique key format: `{connection_id}-{session_id}`
//! When the key changes, the old terminal widget is destroyed and a new one
//! is created, ensuring proper cleanup of handles and subscriptions.
//!
//! # Constraints
//!
//! - Terminal output processing MUST NOT block UI thread
//! - All handles MUST be cleaned up when widget is destroyed
//! - Scroll buffer limited to 10000 lines (Standard edition)
//! - Clipboard operations MUST be supported

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod app;
pub mod terminal;
pub mod platform;

pub use app::EasySSHApp;
pub use terminal::view::TerminalView;
pub use terminal::buffer::TerminalBuffer;
pub use terminal::renderer::TerminalRenderer;

/// Default terminal dimensions for Standard edition
pub const DEFAULT_COLS: u16 = 80;
/// Default number of rows for terminal
pub const DEFAULT_ROWS: u16 = 24;

/// Maximum scrollback buffer lines (Standard edition)
pub const MAX_SCROLLBACK_LINES: usize = 10000;

/// Target frame rate for rendering
pub const TARGET_FPS: u32 = 60;

/// Frame time in milliseconds
pub const FRAME_TIME_MS: f64 = 1000.0 / TARGET_FPS as f64;