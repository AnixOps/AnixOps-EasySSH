/**
 * Utility functions for EasySSH
 * @module utils
 */

// Types are imported for documentation purposes

// =============================================================================
// Date & Time Utilities
// =============================================================================

/**
 * Format timestamp to relative time string
 * @param timestamp - Unix timestamp in milliseconds
 * @returns Relative time string (e.g., "2 hours ago")
 */
export function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  const weeks = Math.floor(days / 7);
  const months = Math.floor(days / 30);
  const years = Math.floor(days / 365);

  if (seconds < 60) return '刚刚';
  if (minutes < 60) return `${minutes} 分钟前`;
  if (hours < 24) return `${hours} 小时前`;
  if (days < 7) return `${days} 天前`;
  if (weeks < 4) return `${weeks} 周前`;
  if (months < 12) return `${months} 月前`;
  return `${years} 年前`;
}

/**
 * Format timestamp to locale date string
 * @param timestamp - Unix timestamp in milliseconds
 * @returns Formatted date string
 */
export function formatDateTime(timestamp: number): string {
  return new Date(timestamp).toLocaleString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * Format duration in milliseconds to human-readable string
 * @param ms - Duration in milliseconds
 * @returns Formatted duration string
 */
export function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}天 ${hours % 24}小时`;
  if (hours > 0) return `${hours}小时 ${minutes % 60}分钟`;
  if (minutes > 0) return `${minutes}分钟`;
  return `${seconds}秒`;
}

// =============================================================================
// String Utilities
// =============================================================================

/**
 * Generate a unique identifier
 * @returns UUID string
 */
export function generateId(): string {
  return crypto.randomUUID();
}

/**
 * Truncate string with ellipsis
 * @param str - String to truncate
 * @param maxLength - Maximum length
 * @returns Truncated string
 */
export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 3) + '...';
}

/**
 * Convert string to kebab-case
 * @param str - String to convert
 * @returns Kebab-case string
 */
export function toKebabCase(str: string): string {
  return str
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .replace(/[\s_]+/g, '-')
    .toLowerCase();
}

/**
 * Capitalize first letter of string
 * @param str - String to capitalize
 * @returns Capitalized string
 */
export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

// =============================================================================
// Keyboard Utilities
// =============================================================================

/**
 * Parse keyboard shortcut string
 * @param shortcut - Shortcut string (e.g., "Cmd+Shift+K")
 * @returns Parsed shortcut object
 */
export function parseKeyboardShortcut(shortcut: string): {
  key: string;
  modifiers: { cmd: boolean; shift: boolean; alt: boolean; ctrl: boolean };
} {
  const parts = shortcut.split('+').map(p => p.trim().toLowerCase());
  const key = parts.find(p => !['cmd', 'ctrl', 'alt', 'shift', 'meta'].includes(p)) || '';

  return {
    key: key.toUpperCase(),
    modifiers: {
      cmd: parts.includes('cmd') || parts.includes('meta'),
      ctrl: parts.includes('ctrl'),
      alt: parts.includes('alt'),
      shift: parts.includes('shift'),
    },
  };
}

/**
 * Format keyboard shortcut for display
 * @param shortcut - Shortcut string
 * @returns Formatted string with platform-specific symbols
 */
export function formatKeyboardShortcut(shortcut: string): string {
  const isMac = navigator.platform.toLowerCase().includes('mac');
  const parts = shortcut.split('+').map(p => p.trim());

  return parts
    .map(part => {
      const lower = part.toLowerCase();
      if (lower === 'cmd' || lower === 'meta') return isMac ? '⌘' : 'Win';
      if (lower === 'ctrl') return isMac ? '⌃' : 'Ctrl';
      if (lower === 'alt') return isMac ? '⌥' : 'Alt';
      if (lower === 'shift') return isMac ? '⇧' : 'Shift';
      return part.toUpperCase();
    })
    .join(isMac ? ' ' : '+');
}

/**
 * Check if keyboard event matches shortcut
 * @param event - Keyboard event
 * @param shortcut - Shortcut to match
 * @returns Whether the event matches
 */
export function matchesShortcut(event: KeyboardEvent, shortcut: string): boolean {
  const parsed = parseKeyboardShortcut(shortcut);
  const keyMatch = event.key.toUpperCase() === parsed.key;
  const cmdMatch = event.metaKey === parsed.modifiers.cmd;
  const ctrlMatch = event.ctrlKey === parsed.modifiers.ctrl;
  const altMatch = event.altKey === parsed.modifiers.alt;
  const shiftMatch = event.shiftKey === parsed.modifiers.shift;

  return keyMatch && cmdMatch && ctrlMatch && altMatch && shiftMatch;
}

// =============================================================================
// DOM Utilities
// =============================================================================

/**
 * Focus trap for modal dialogs
 * @param container - Container element
 * @returns Cleanup function
 */
export function createFocusTrap(container: HTMLElement): () => void {
  const focusableElements = container.querySelectorAll<HTMLElement>(
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
  );

  const firstElement = focusableElements[0];
  const lastElement = focusableElements[focusableElements.length - 1];

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key !== 'Tab') return;

    if (e.shiftKey && document.activeElement === firstElement) {
      e.preventDefault();
      lastElement?.focus();
    } else if (!e.shiftKey && document.activeElement === lastElement) {
      e.preventDefault();
      firstElement?.focus();
    }
  };

  container.addEventListener('keydown', handleKeyDown);
  firstElement?.focus();

  return () => container.removeEventListener('keydown', handleKeyDown);
}

/**
 * Click outside handler
 * @param element - Element to watch
 * @param callback - Callback when clicked outside
 * @returns Cleanup function
 */
export function onClickOutside(
  element: HTMLElement,
  callback: () => void
): () => void {
  const handleClick = (e: MouseEvent) => {
    if (!element.contains(e.target as Node)) {
      callback();
    }
  };

  document.addEventListener('mousedown', handleClick);
  return () => document.removeEventListener('mousedown', handleClick);
}

// =============================================================================
// Performance Utilities
// =============================================================================

/**
 * Debounce function
 * @param fn - Function to debounce
 * @param delay - Delay in milliseconds
 * @returns Debounced function
 */
export function debounce<T extends (...args: unknown[]) => unknown>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timeoutId: ReturnType<typeof setTimeout>;

  return (...args: Parameters<T>) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  };
}

/**
 * Throttle function
 * @param fn - Function to throttle
 * @param limit - Time limit in milliseconds
 * @returns Throttled function
 */
export function throttle<T extends (...args: unknown[]) => unknown>(
  fn: T,
  limit: number
): (...args: Parameters<T>) => void {
  let inThrottle = false;

  return (...args: Parameters<T>) => {
    if (!inThrottle) {
      fn(...args);
      inThrottle = true;
      setTimeout(() => (inThrottle = false), limit);
    }
  };
}

// =============================================================================
// Color Utilities
// =============================================================================

/**
 * Generate consistent color from string
 * @param str - Input string
 * @returns Hex color string
 */
export function stringToColor(str: string): string {
  const colors = [
    '#0a84ff', // blue
    '#30d158', // green
    '#5e5ce6', // indigo
    '#ff9f0a', // orange
    '#ff375f', // pink
    '#bf5af2', // purple
    '#ff453a', // red
    '#64d2ff', // teal
    '#ffd60a', // yellow
  ];

  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash);
  }

  return colors[Math.abs(hash) % colors.length];
}

/**
 * Adjust color opacity
 * @param color - Hex color
 * @param opacity - Opacity value (0-1)
 * @returns RGBA color string
 */
export function withOpacity(color: string, opacity: number): string {
  const hex = color.replace('#', '');
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${opacity})`;
}

// =============================================================================
// Validation Utilities
// =============================================================================

/**
 * Validate hostname or IP address
 * @param value - Value to validate
 * @returns Whether valid
 */
export function isValidHost(value: string): boolean {
  // IP address regex
  const ipRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
  // Hostname regex
  const hostnameRegex = /^[a-zA-Z0-9][a-zA-Z0-9-_.]*[a-zA-Z0-9]$/;

  return ipRegex.test(value) || hostnameRegex.test(value);
}

/**
 * Validate port number
 * @param port - Port number
 * @returns Whether valid
 */
export function isValidPort(port: number): boolean {
  return Number.isInteger(port) && port > 0 && port <= 65535;
}
