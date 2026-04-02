//! Core Services Module
//!
//! This module provides core business logic services for EasySSH.

pub mod search_service;
pub mod server_service;

pub use search_service::{
    AuthMethod as SearchAuthMethod, ConnectionStatus, SearchHistoryEntry, SearchQuery,
    SearchQueryBuilder, SearchResult, SearchService, SortBy, SortOrder,
};

pub use server_service::{
    AsyncServerService, ConnectionTestResult, ServerService, ServerServiceError,
};
