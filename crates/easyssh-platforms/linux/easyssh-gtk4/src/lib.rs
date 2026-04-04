//! EasySSH GTK4 Native Client for Linux
//!
//! This crate provides a native GTK4/Libadwaita UI for EasySSH on Linux.
//!
//! # Features
//!
//! - Native GTK4 widgets
//! - Libadwaita styling for GNOME integration
//! - Embedded terminal support (Standard version)
//! - SFTP browser
//! - Server management
//!
//! # Editions
//!
//! - `lite` - Basic SSH connection management
//! - `standard` - Full featured with embedded terminal
//! - `pro` - Team collaboration features
//!
//! # Example
//!
//! ```rust,ignore
//! use easyssh_gtk4::application::EasySSHApplication;
//!
//! fn main() {
//!     let app = EasySSHApplication::new();
//!     app.run();
//! }
//! ```

// Public modules
pub mod app;
pub mod application;
pub mod models;
pub mod views;
pub mod widgets;
pub mod dialogs;
pub mod settings;
pub mod theme;

// Terminal module (Standard/Pro only)
#[cfg(feature = "embedded-terminal")]
pub mod terminal;

// Re-exports for convenience
pub use app::EasySSHApp;
pub use app::AppViewModel;
pub use application::EasySSHApplication;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "com.easyssh.EasySSH";