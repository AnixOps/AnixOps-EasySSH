/**
 * Common types used across the application
 */

// Server status types
export type ServerStatus = 'online' | 'offline' | 'warning' | 'unknown';

// Auth types
export type AuthType = 'agent' | 'key' | 'password';

// Theme types
export type Theme = 'light' | 'dark';

// Product mode types
export type ProductMode = 'lite' | 'standard' | 'pro';

// Loading state
export interface LoadingState {
  isLoading: boolean;
  error: string | null;
}

// Pagination
export interface PaginationParams {
  page: number;
  pageSize: number;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// Sorting
export interface SortParams {
  field: string;
  direction: 'asc' | 'desc';
}

// Filter
export interface FilterParams {
  query: string;
  status?: string;
  groupId?: string;
}
