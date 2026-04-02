//! Team module - Team management for Pro version

#![cfg(feature = "team")]

mod manager;
mod permissions;
mod types;

pub use manager::TeamManager;
pub use permissions::{check_permission, TeamPermission};
pub use types::*;
