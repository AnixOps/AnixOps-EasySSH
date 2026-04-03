//! UI Components Module
//!
//! This module contains reusable UI components for the EasySSH Windows application.
//!
//! Components:
//! - `tab_bar`: Multi-terminal tab bar with drag-and-drop, context menu, and keyboard shortcuts

pub mod tab_bar;

// Re-export main types for convenience
pub use tab_bar::{
    SessionType, TabBar, TabBarBuilder, TabBarResponse, TabDisplayState, TabManager, TabState,
};
