// Constant values for the EasySSH application

// App metadata
export const APP_NAME = 'EasySSH';
export const APP_VERSION = '0.2.0';

// Default values
export const DEFAULT_SSH_PORT = 22;
export const DEFAULT_THEME = 'dark' as const;
export const DEFAULT_PRODUCT_MODE = 'lite' as const;

// UI constants
export const SIDEBAR_WIDTH = 320; // pixels
export const RIGHT_PANEL_WIDTH = 320; // pixels
export const HEADER_HEIGHT = 64; // pixels

// Pagination defaults
export const DEFAULT_PAGE_SIZE = 20;
export const MAX_PAGE_SIZE = 100;

// Terminal defaults
export const DEFAULT_TERMINAL_FONT_SIZE = 14;
export const DEFAULT_TERMINAL_FONT_FAMILY = 'JetBrains Mono, Menlo, Monaco, Consolas, monospace';

// Storage keys
export const STORAGE_KEYS = {
  UI_STATE: 'easyssh-ui-state',
  COMMAND_HISTORY: 'easyssh-command-history',
  TEAM_STATE: 'easyssh-team-state',
} as const;

// API timeout (ms)
export const API_TIMEOUT = 30000;

// SSH connection defaults
export const SSH_DEFAULTS = {
  port: 22,
  timeout: 30000,
  keepaliveInterval: 10000,
  keepaliveCountMax: 3,
} as const;
