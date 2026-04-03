//! Connection Management Module
//!
//! This module provides connection management functionality including:
//! - Automatic reconnection with exponential backoff
//! - Heartbeat monitoring
//! - Connection state tracking
//!
//! # Architecture
//!
//! The module follows the system invariants defined in SYSTEM_INVARIANTS.md:
//! - All reconnection operations are orchestrated through `ReconnectOrchestrator`
//! - Heartbeat monitoring is separate from reconnection logic
//! - User-initiated disconnection does not trigger automatic reconnection
//!
//! # Example
//!
//! ```rust
//! use easyssh_core::connection::{ReconnectOrchestrator, ReconnectConfig};
//! use std::time::Duration;
//!
//! let config = ReconnectConfig {
//!     max_retries: 10,
//!     base_delay: Duration::from_secs(1),
//!     max_delay: Duration::from_secs(60),
//!     jitter: 0.3,
//! };
//!
//! let orchestrator = ReconnectOrchestrator::new(config);
//! ```

pub mod reconnect;

// Re-export main types for convenience
pub use reconnect::{
    HeartbeatConfig, HeartbeatMonitor, HeartbeatStatus, ReconnectConfig, ReconnectEvent,
    ReconnectOrchestrator, ReconnectState, ReconnectTrigger,
};