//! Sync module - End-to-end encrypted configuration synchronization
//!
//! This module provides cross-device synchronization with E2EE (AES-256-GCM).

#![cfg(feature = "sync")]

mod types;
mod engine;
mod conflict;
mod providers;

// Re-export all public types
pub use types::*;
pub use engine::SyncManager;
pub use conflict::{SyncConflict, FieldConflict, SyncConflictResolution};
pub use providers::{
    SyncProviderImpl, LocalSyncHandler, DisabledProvider, ICloudProvider,
    GoogleDriveProvider, OneDriveProvider, DropBoxProvider, SelfHostedProvider,
    LocalNetworkProvider, LocalFileProvider,
};
