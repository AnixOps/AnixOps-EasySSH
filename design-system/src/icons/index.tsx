import React from 'react';
import { cn } from '../utils';

/**
 * Icon System
 *
 * React component for displaying Lucide icons with theme support.
 * Icons are loaded as SVG for crisp rendering at any size.
 */

export type IconName =
  // Connection & Network
  | 'server'
  | 'database'
  | 'globe'
  | 'network'
  | 'wifi'
  | 'wifi-off'
  | 'cloud'
  | 'cloud-offline'
  // Actions
  | 'play'
  | 'pause'
  | 'stop'
  | 'refresh'
  | 'reload'
  // Navigation
  | 'home'
  | 'chevron-left'
  | 'chevron-right'
  | 'chevron-up'
  | 'chevron-down'
  | 'arrow-left'
  | 'arrow-right'
  | 'arrow-up'
  | 'arrow-down'
  | 'menu'
  | 'close'
  | 'maximize'
  | 'minimize'
  | 'expand'
  | 'collapse'
  // Files & Folders
  | 'folder'
  | 'folder-open'
  | 'file'
  | 'file-text'
  | 'file-code'
  | 'file-key'
  | 'upload'
  | 'download'
  | 'save'
  | 'trash'
  // Terminal & Code
  | 'terminal'
  | 'code'
  | 'command'
  | 'prompt'
  | 'cursor'
  // Settings & Tools
  | 'settings'
  | 'sliders'
  | 'filter'
  | 'search'
  | 'zoom-in'
  | 'zoom-out'
  | 'wrench'
  // Security
  | 'lock'
  | 'unlock'
  | 'key'
  | 'shield'
  | 'shield-check'
  | 'shield-alert'
  | 'eye'
  | 'eye-off'
  // Status & Feedback
  | 'check'
  | 'check-circle'
  | 'x'
  | 'x-circle'
  | 'alert-circle'
  | 'alert-triangle'
  | 'info'
  | 'help-circle'
  | 'bell'
  | 'bell-off'
  // User & Account
  | 'user'
  | 'users'
  | 'user-plus'
  | 'user-minus'
  | 'user-check'
  | 'log-in'
  | 'log-out'
  // Communication
  | 'mail'
  | 'message-square'
  | 'message-circle'
  | 'chat'
  | 'phone'
  | 'video'
  | 'share'
  | 'share-2'
  | 'link'
  | 'unlink'
  | 'external-link'
  | 'copy'
  | 'clipboard'
  // Time
  | 'clock'
  | 'calendar'
  | 'calendar-check'
  | 'calendar-x'
  | 'timer'
  | 'history'
  // Layout
  | 'layout'
  | 'layout-grid'
  | 'layout-list'
  | 'sidebar'
  | 'columns'
  | 'rows'
  | 'grid'
  | 'list'
  // Indicators
  | 'circle'
  | 'square'
  | 'star'
  | 'heart'
  | 'thumbs-up'
  | 'thumbs-down'
  | 'flag'
  | 'bookmark'
  | 'pin'
  | 'zap'
  | 'activity'
  | 'pulse'
  // Editor
  | 'edit'
  | 'edit-2'
  | 'edit-3'
  | 'pen'
  | 'pencil'
  | 'type'
  | 'bold'
  | 'italic'
  | 'underline'
  | 'align-left'
  | 'align-center'
  | 'align-right'
  // Data & Analytics
  | 'bar-chart'
  | 'bar-chart-2'
  | 'line-chart'
  | 'pie-chart'
  | 'trending-up'
  | 'trending-down'
  | 'activity'
  | 'gauge'
  // Development
  | 'git-branch'
  | 'git-commit'
  | 'git-merge'
  | 'git-pull-request'
  | 'github'
  | 'container'
  | 'cpu'
  | 'hard-drive'
  | 'disc'
  | 'monitor'
  // Other
  | 'more-horizontal'
  | 'more-vertical'
  | 'dots'
  | 'layers'
  | 'box'
  | 'archive'
  | 'inbox'
  | 'send'
  | 'package'
  | 'map'
  | 'map-pin'
  | 'navigation'
  | 'compass'
  | 'target'
  | 'focus'
  | 'maximize-2'
  | 'minimize-2'
  | 'move'
  | 'rotate-ccw'
  | 'rotate-cw'
  | 'minus'
  | 'plus'
  | 'star';

/**
 * Icon SVG paths mapping
 * These are simplified SVG paths for common icons
 */
const iconPaths: Record<IconName, string> = {
  server: 'M20 7H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2zM2 11h20',
  database: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z',
  globe: 'M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zM2 12h20',
  network: 'M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z',
  wifi: 'M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01',
  'wifi-off': 'M12 20h.01M8.53 16.11a6 6 0 0 1 6.95 0M6 13.55A11 11 0 0 1 12 11c2.64 0 5.08.96 7 2.54M3 3l18 18',
  cloud: 'M17.5 19c0-1.7-1.3-3-3-3h-11c-1.7 0-3 1.3-3 3 0 1.7 1.3 3 3 3h11c1.7 0 3-1.3 3-3z',
  'cloud-offline': 'M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z',
  play: 'm5 3 14 9-14 9V3z',
  pause: 'M10 4H6v16h4V4zm8 0h-4v16h4V4z',
  stop: 'M4 4h16v16H4z',
  refresh: 'M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8',
  reload: 'M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8',
  home: 'm3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
  'chevron-left': 'm15 18-6-6 6-6',
  'chevron-right': 'm9 18 6-6-6-6',
  'chevron-up': 'm18 15-6-6-6 6',
  'chevron-down': 'm6 9 6 6 6-6',
  'arrow-left': 'm12 19-7-7 7-7m7 7H5',
  'arrow-right': 'M5 12h14m-7-7 7 7-7 7',
  'arrow-up': 'm18 15-6-6-6 6',
  'arrow-down': 'm6 9 6 6 6-6',
  menu: 'M4 12h16M4 18h16M4 6h16',
  close: 'M18 6 6 18M6 6l12 12',
  maximize: 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
  minimize: 'M4 14h6v6M20 10h-6V4',
  expand: 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
  collapse: 'M4 14h6v6M20 10h-6V4',
  folder: 'M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z',
  'folder-open': 'M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z',
  file: 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z',
  'file-text': 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z',
  'file-code': 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z',
  'file-key': 'M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z',
  upload: 'M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4m14-7-5-5-5 5m5-5v12',
  download: 'M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4m4-5 5 5 5-5M12 15V3',
  save: 'M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z',
  trash: 'M3 6h18m-2 0v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6',
  terminal: 'm4 17 6-6-6-6m8 14h8',
  code: 'm16 18 6-6-6-6M8 6l-6 6 6 6',
  command: 'M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3',
  prompt: '>',
  cursor: '|',
  settings: 'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z',
  sliders: 'M4 21v-7m0-4V3m8 18v-9m0-4V3m8 18v-5m0-4V3M1 14h6m2-6h6m2 6h6',
  filter: 'M22 3H2l8 9.46V19l4 2v-8.54L22 3z',
  search: 'm21 21-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z',
  'zoom-in': 'm21 21-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM11 8v6M8 11h6',
  'zoom-out': 'm21 21-4.3-4.3M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM8 11h6',
  wrench: 'M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z',
  lock: 'M19 11H5a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-6a2 2 0 0 0-2-2zm0 0V7a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v4',
  unlock: 'M7 11V7a5 5 0 0 1 9.9-1',
  key: 'm21 2-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0 3 3L22 7l-3-3m-3.5 3.5L19 4',
  shield: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z',
  'shield-check': 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10zM9 12l2 2 4-4',
  'shield-alert': 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10zM12 8v4M12 16h.01',
  eye: 'M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z',
  'eye-off': 'M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24',
  check: 'M20 6 9 17l-5-5',
  'check-circle': 'M22 11.08V12a10 10 0 1 1-5.93-9.14M22 4 12 14.01l-3-3',
  x: 'M18 6 6 18M6 6l12 12',
  'x-circle': 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM15 9l-6 6M9 9l6 6',
  'alert-circle': 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 8v4M12 16h.01',
  'alert-triangle': 'm21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3zM12 9v4M12 17h.01',
  info: 'M12 16v-4M12 8h.01M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z',
  'help-circle': 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z',
  bell: 'M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9',
  'bell-off': 'M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9M4.73 4.73 20 20',
  user: 'M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2',
  users: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75',
  'user-plus': 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3.13a4 4 0 0 1 0 7.75M22 21v-2',
  'user-minus': 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3.13a4 4 0 0 1 0 7.75M22 21v-2',
  'user-check': 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3.13a4 4 0 0 1 0 7.75M22 21l-3-3m0 0 3-3m-3 3h6',
  'log-in': 'M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4M10 17l5-5-5-5M13.8 12H3',
  'log-out': 'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9',
  mail: 'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z',
  'message-square': 'M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z',
  'message-circle': 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z',
  chat: 'M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z',
  phone: 'M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z',
  video: 'm22 8-6 4 6 4V8zM2 5h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z',
  share: 'M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8m-4-6-4-4-4 4m4-4v13',
  'share-2': 'M18 16.08a2.12 2.12 0 0 0-1.56.5l-5.15-2.58a2.12 2.12 0 0 0 0-.95l5.15-2.58A2.12 2.12 0 1 0 15 6.1L9.85 8.68a2.12 2.12 0 1 0 0 2.63L15 13.9a2.12 2.12 0 1 0 3-1.83z',
  link: 'M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71',
  unlink: 'M18.364 18.364A9 9 0 0 0 5.636 5.636m12.728 12.728A9 9 0 0 1 5.636 5.636m12.728 12.728L5.636 5.636',
  'external-link': 'M15 3h6v6M9 21 3 15l12-12',
  copy: 'M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2',
  clipboard: 'M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2',
  clock: 'M12 6v6l4 2m6-2A10 10 0 1 1 2 12a10 10 0 0 1 20 0z',
  calendar: 'M8 2v4m8-4v4M3 10h18M4 10v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V10',
  'calendar-check': 'M8 2v4m8-4v4M3 10h18M4 10v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V10m-9 5 2 2 4-4',
  'calendar-x': 'M8 2v4m8-4v4M3 10h18M4 10v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V10m-11 5 4 4m0-4-4 4',
  timer: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v6l4 2',
  history: 'M3 3v5h5M3.05 13A9 9 0 1 0 6 5.3L3 8',
  layout: 'M19 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2z',
  'layout-grid': 'M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z',
  'layout-list': 'M3 14h18M3 19h18M3 9h18M3 4h18',
  sidebar: 'M4 4h16v16H4zM9 4v16',
  columns: 'M4 4h16v16H4zM12 4v16',
  rows: 'M4 4h16v16H4zM4 12h16',
  grid: 'M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z',
  list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
  circle: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z',
  square: 'M3 3h18v18H3z',
  star: 'm12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z',
  heart: 'M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z',
  'thumbs-up': 'M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3',
  'thumbs-down': 'M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7 0h3a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-3',
  flag: 'M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1zM4 22v-7',
  bookmark: 'm19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z',
  pin: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z',
  zap: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z',
  activity: 'M22 12h-4l-3 9L9 3l-3 9H2',
  pulse: 'M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.78 4.78 4 4 0 0 1-6.74 0 4 4 0 0 1-4.78-4.78 4 4 0 0 1 0-6.74Z',
  edit: 'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z',
  'edit-2': 'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z',
  'edit-3': 'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z',
  pen: 'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z',
  pencil: 'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z',
  type: 'M4 7V4h16v3M9 20h6M12 4v16',
  bold: 'M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6zM6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z',
  italic: 'M19 4h-9M14 20H5M15 4 9 20',
  underline: 'M6 3v7a6 6 0 0 0 6 6 6 6 0 0 0 6-6V3M4 21h16',
  'align-left': 'M17 10H3M21 6H3M21 14H3M17 18H3',
  'align-center': 'M21 10H3M21 6H3M21 14H3M21 18H3',
  'align-right': 'M21 10H7M21 6H3M21 14H3M21 18H7',
  'bar-chart': 'M18 20V10M12 20V4M6 20v-6',
  'bar-chart-2': 'M18 20V10M12 20V4M6 20v-6',
  'line-chart': 'M3 3v18h18M18 17V9M13 17V5M8 17v-3',
  'pie-chart': 'M21.21 15.89A10 10 0 1 1 8 2.83',
  'trending-up': 'M23 6l-9.5 9.5-5-5L1 18M17 6h6v6',
  'trending-down': 'M23 18l-9.5-9.5-5 5L1 6M17 18h6v-6',
  'git-branch': 'M6 3v12M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 9a9 9 0 0 1-9 9',
  'git-commit': 'M12 3v18M7 8a5 5 0 1 0 10 0 5 5 0 0 0-10 0z',
  'git-merge': 'M7 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM18 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM7 8v8a9 9 0 0 0 9 9',
  'git-pull-request': 'M18 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 8V3h12v12',
  github: 'M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22',
  container: 'M2 6h20v12H2zM6 6v12M18 6v12M10 6v12M14 6v12',
  cpu: 'M4 4h16v16H4zM9 9h6v6H9zM9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 15h3M1 9h3M1 15h3',
  'hard-drive': 'M22 12h-4l-3 9L9 3l-3 9H2',
  disc: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z',
  monitor: 'M20 3H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2zM12 17v4',
  'more-horizontal': 'M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0-5a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0 10a1 1 0 1 0 0-2 1 1 0 0 0 0 2z',
  'more-vertical': 'M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0-5a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0 10a1 1 0 1 0 0-2 1 1 0 0 0 0 2z',
  dots: 'M12 13a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0-5a1 1 0 1 0 0-2 1 1 0 0 0 0 2zm0 10a1 1 0 1 0 0-2 1 1 0 0 0 0 2z',
  layers: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5',
  box: 'M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z',
  archive: 'M20 9v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V9M20 9l-8-5-8 5M20 9H4M9 22V12h6v10',
  inbox: 'M22 12h-6l-2 3H10l-2-3H2M22 12v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-6',
  send: 'M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z',
  package: 'M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z',
  map: 'M1 6v16l7-4 8 4 7-4V2l-7 4-8-4-7 4z',
  'map-pin': 'M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0zM10 10a4 4 0 1 0 8 0 4 4 0 0 0-8 0z',
  navigation: 'M3 11l19-9-9 19-2-8-8-2z',
  compass: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM16.24 7.76l-2.12 6.36-6.36 2.12 2.12-6.36 6.36-2.12z',
  target: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v12M6 12h12',
  focus: 'M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10zM12 6v12M6 12h12',
  'maximize-2': 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
  'minimize-2': 'M4 14h6v6M20 10h-6V4',
  move: 'M5 9l-3 3 3 3M9 5l3-3 3 3M19 9l3 3-3 3M9 19l3 3 3-3M2 12h20M12 2v20',
  'rotate-ccw': 'M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8',
  'rotate-cw': 'M21 12a9 9 0 1 1-9-9 9.75 9.75 0 0 1 6.74 2.74L21 8',
  minus: 'M5 12h14',
  plus: 'M12 5v14M5 12h14',
};

export interface IconProps extends React.SVGAttributes<SVGSVGElement> {
  /** Icon name */
  name: IconName;
  /** Icon size */
  size?: number;
  /** Additional className */
  className?: string;
  /** Stroke width */
  strokeWidth?: number;
}

export const Icon: React.FC<IconProps> = ({
  name,
  size = 20,
  className,
  strokeWidth = 2,
  ...props
}) => {
  const path = iconPaths[name];

  if (!path) {
    console.warn(`Icon "${name}" not found`);
    return null;
  }

  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={cn('inline-block', className)}
      aria-hidden="true"
      {...props}
    >
      {path.split(/(?=[MC])/)[0] === path ? (
        // Single path
        <path d={path} />
      ) : (
        // Multiple paths (split by command)
        path.split(/(?=[MC])/g).map((p, i) => <path key={i} d={p} />)
      )}
    </svg>
  );
};

/**
 * Icon with label for accessibility
 */
export interface IconWithLabelProps extends IconProps {
  /** Accessible label */
  label: string;
}

export const IconWithLabel: React.FC<IconWithLabelProps> = ({ label, ...props }) => {
  return (
    <span className="inline-flex items-center gap-1.5">
      <Icon {...props} />
      <span>{label}</span>
    </span>
  );
};

export default Icon;
