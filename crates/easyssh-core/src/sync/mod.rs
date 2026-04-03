//! Sync module - End-to-end encrypted configuration synchronization
//!
//! This module provides cross-device synchronization with E2EE (AES-256-GCM).

mod conflict;
mod engine;
mod providers;
mod types;

// Re-export all public types
pub use conflict::{FieldConflict, SyncConflict, SyncConflictResolution};
pub use engine::SyncManager;
pub use providers::{
    DisabledProvider, DropBoxProvider, GoogleDriveProvider, ICloudProvider, LocalFileProvider,
    LocalNetworkProvider, LocalSyncHandler, OneDriveProvider, SelfHostedProvider, SyncProviderImpl,
};
pub use types::*;
