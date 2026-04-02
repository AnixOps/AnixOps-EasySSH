//! Team module - Team management for Pro version

#![cfg(feature = "team")]

mod types;
mod manager;
mod permissions;

pub use types::*;
pub use manager::TeamManager;
pub use permissions::{TeamPermission, check_permission};
