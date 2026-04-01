// Session Recording Components
// Asciinema-compatible terminal recording system

export { SessionPlayer, type SessionPlayerProps, type SessionPlayerHandle } from './SessionPlayer';
export { RecordingManager, type RecordingManagerProps } from './RecordingManager';
export { ExportManager, type ExportManagerProps, type ExportOptions, type ExportFormat } from './ExportManager';

// Re-export types from Rust core
export type {
  AsciinemaHeader,
  AsciinemaEvent,
  AsciinemaEventType,
  SessionMark,
  RecordingState,
  RecordingConfig,
  RecordingMetadata,
  PlaybackState,
  PlaybackSpeed,
  SearchResult,
  MatchType,
  ExportFormat as RustExportFormat,
  ExportOptions as RustExportOptions,
  CloudShareConfig,
} from '../../../core/src/session_recording';

// Constants
export const DEFAULT_ASCIINEMA_VERSION = 2;
export const SUPPORTED_SPEEDS: PlaybackSpeed[] = [0.5, 1, 1.5, 2, 4];
export const DEFAULT_THEME = {
  background: '#1e1e1e',
  foreground: '#d4d4d4',
  cursor: '#d4d4d4',
  selectionBackground: '#264f78',
};

// Utility functions
export const formatDuration = (seconds: number): string => {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const ms = Math.floor((seconds % 1) * 100);
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
};

export const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
};

export const formatTimestamp = (timestamp: number): string => {
  return new Date(timestamp * 1000).toLocaleString();
};

// Keyboard shortcuts helper
export const PLAYER_SHORTCUTS = {
  SPACE: 'Play/Pause',
  LEFT: 'Seek backward 5s',
  RIGHT: 'Seek forward 5s',
  '1': '0.5x speed',
  '2': '1x speed',
  '3': '1.5x speed',
  '4': '2x speed',
  '5': '4x speed',
};
