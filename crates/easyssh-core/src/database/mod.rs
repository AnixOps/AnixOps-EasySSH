//! EasySSH Lite Database Module
//!
//! This module provides asynchronous SQLite database operations using sqlx.
//! It implements the storage layer for Lite version with:
//! - Server configuration storage
//! - Group management
//! - Application configuration
//! - Migration management
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::database::{Database, ServerRepository, GroupRepository};
//!
//! async fn example() -> anyhow::Result<()> {
//!     // Initialize database
//!     let db = Database::new("easyssh.db").await?;
//!     db.init().await?;
//!
//!     // Use repositories
//!     let servers = db.server_repository();
//!     let groups = db.group_repository();
//!
//!     Ok(())
//! }
//! ```

#![cfg(feature = "database")]

mod config_repository;
mod database;
mod error;
mod group_repository;
mod migrations;
mod models;
mod server_repository;

pub use config_repository::ConfigRepository;
pub use database::Database;
pub use error::{DatabaseError, Result};
pub use group_repository::{GroupRepository, GroupWithCount};
pub use migrations::{Migration, MigrationManager, MigrationStatus};
pub use models::{
    AppConfig, Group, NewGroup, NewServer, QueryOptions, Server, ServerFilters, ServerWithGroup,
    UpdateGroup, UpdateServer,
};
pub use server_repository::ServerRepository;

use std::path::PathBuf;

/// Get the default database path for the application.
///
/// Returns the platform-appropriate path for the EasySSH database file.
/// On most systems, this will be in the user's data directory under
/// `EasySSH/easyssh.db`.
///
/// # Example
///
/// ```rust
/// use easyssh_core::database::get_default_db_path;
///
/// let path = get_default_db_path();
/// println!("Database path: {:?}", path);
/// ```
pub fn get_default_db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    data_dir.join("EasySSH").join("easyssh.db")
}

/// Ensure the database directory exists.
///
/// Creates the parent directories for the database file if they don't exist.
///
/// # Errors
///
/// Returns `DatabaseError::Io` if directory creation fails.
pub fn ensure_db_directory(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
