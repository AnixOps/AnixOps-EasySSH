//! Core Services Module
//!
//! This module provides core business logic services for EasySSH.

pub mod group_service;
pub mod search_service;
pub mod server_service;

pub use group_service::{
    AsyncGroupService, BatchOperationResult, GroupImportResult, GroupResult, GroupService,
    GroupServiceError,
};

pub use search_service::{
    AuthMethod as SearchAuthMethod, ConnectionStatus, SearchHistoryEntry, SearchQuery,
    SearchQueryBuilder, SearchResult, SearchService, SortBy, SortOrder,
};

pub use server_service::{
    AsyncServerService, ConnectionTestResult, ServerImportResult, ServerResult, ServerService,
    ServerServiceError, ServerStats, TransactionError, TransactionResult,
};
