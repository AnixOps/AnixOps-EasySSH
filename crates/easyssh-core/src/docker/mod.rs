//! Docker module - Remote Docker container management via SSH

#![cfg(feature = "docker")]

mod client;
mod containers;
mod images;
mod types;

pub use client::DockerManager;
pub use types::*;
