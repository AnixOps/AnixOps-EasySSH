//! GTK4 Terminal Module for Linux Standard Version
//!
//! This module provides embedded terminal functionality for the GTK4 platform,
//! implementing the Standard version requirements per SYSTEM_INVARIANTS.md.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           TerminalView                   │
//! │    (GTK4 Widget Container)               │
//! │    ┌─────────────────────────────┐      │
//! │    │ TerminalBuffer              │      │
//! │    │ (TextBuffer + Search)       │      │
//! │    └─────────────────────────────┘      │
//! │    ┌─────────────────────────────┐      │
//! │    │ TerminalSearchBar           │      │
//! │    │ (Search UI)                 │      │
//! │    └─────────────────────────────┘      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Constraints (SYSTEM_INVARIANTS.md Section 6)
//!
//! - Key format: `{connection_id}-{session_id}` (Section 0.2)
//! - Component destruction must clean all handles (Section 0.2)
//! - Output handling MUST NOT block UI thread (Section 1.1)
//! - Scroll buffer limit: 10000 lines (Section 1.2)
//!
//! # Features
//!
//! - PTY session integration
//! - ANSI color support (16 base colors + 256 colors)
//! - Search with regex support
//! - Key-driven reset support
//! - Command history navigation

mod view;
mod buffer;
mod input;
mod search;
mod style;

pub use view::TerminalView;
pub use buffer::{TerminalBuffer, SearchMatch};
pub use search::TerminalSearchBar;
pub use style::{TerminalStyle, CursorStyle};

// Re-export core types for convenience
#[cfg(feature = "embedded-terminal")]
pub use easyssh_core::terminal::{
    PtyHandle, PtyManager, PtyConfig, PtySize, PtyState,
    ScrollBuffer, Line,
};