# EasySSH Integration Architecture Plan

> Technical Lead & Integration Architecture Document
> Version: 0.1.0 | Date: 2026-03-31

---

## Executive Summary

This document defines the integration strategy for EasySSH, coordinating between:
- **Core Rust Library** (`easyssh-core`)
- **Platform Native UIs** (Windows WinUI, Linux GTK4, macOS SwiftUI - existing)
- **New Tauri-based Frontend** (React + TypeScript - to be created)

The goal is to establish a unified architecture that supports all three product tiers (Lite, Standard, Pro) with shared business logic in Rust and a modern React frontend for the Standard/Pro desktop experience.

---

## 1. Current Architecture Assessment

### 1.1 Existing Components

| Component | Status | Technology | Notes |
|-----------|--------|------------|-------|
| `easyssh-core` | ✅ Working | Rust | SQLite, SSH2, crypto, keychain |
| TUI | ✅ Working | Rust/ratatui | Cross-platform CLI |
| Windows UI | 🚧 Partial | egui | Functional but limited |
| Linux GTK4 | 🚧 Skeleton | GTK4/libadwaita | Basic structure only |
| macOS SwiftUI | 📋 Planned | SwiftUI | Skeleton only |
| Web Frontend | ❌ Missing | - | **This plan addresses this** |

### 1.2 Core Library Capabilities

```rust
// Current public API (from core/src/lib.rs)
pub struct AppState {
    pub db: StdMutex<Option<db::Database>>,
    pub ssh_manager: Mutex<SshSessionManager>,
    pub sftp_manager: Mutex<SftpSessionManager>,  // feature-gated
}

// Key operations already implemented:
- get_servers() / add_server() / update_server() / delete_server()
- get_groups() / add_group() / update_group() / delete_group()
- init_database()
- connect_server()  // Lite: native terminal
- ssh_connect()     // Standard: embedded session
- ssh_execute() / ssh_disconnect()
- execute_stream()  // Streaming for terminal
- write_shell_input()
- interrupt_command()
```

### 1.3 Data Models (Already Defined)

```rust
// ServerRecord - existing in core/src/db.rs
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,  // "password" | "key" | "agent"
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

// Additional models: GroupRecord, HostRecord, SessionRecord, etc.
```

---

## 2. Integration Strategy

### 2.1 Dual-Path Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        EasySSH Product Family                    │
├─────────────────────────────────────────────────────────────────┤
│  Lite        → Native UIs (WinUI/GTK/SwiftUI) + Native Terminal │
│  Standard    → Tauri + React + xterm.js (Embedded Terminal)    │
│  Pro         → Tauri + React + Team Features + SSO             │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────┴───────────────────────────────────┐
│                      Unified Rust Core                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   SQLite    │  │ SSH/Mux     │  │  Crypto     │            │
│  │   Database  │  │ Sessions    │  │  (Argon2id  │            │
│  │             │  │             │  │  + AES-GCM) │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │  Keychain   │  │   SFTP      │  │  Pro/Tail   │            │
│  │  keyring    │  │  (optional) │  │ scale/Team  │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Tauri Architecture (Standard/Pro)

```
┌─────────────────────────────────────────────────────────────────┐
│                     React Frontend (TypeScript)                  │
├─────────────────────────────────────────────────────────────────┤
│  App Layer:      App.tsx, Router, Providers                     │
│  Features:       Servers, Terminal, SFTP, Team, Settings         │
│  Components:     Layout, Navigation, Terminal, Forms            │
│  Stores:         Zustand (uiStore, serverStore, sessionStore)   │
├─────────────────────────────────────────────────────────────────┤
│  Tauri IPC Bridge                                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  invoke('get_servers')  →  Rust Command Handler        │   │
│  │  invoke('ssh_connect')  →  SSH Session Manager         │   │
│  │  events (terminal data) →  xterm.js                     │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Tauri Runtime                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   Commands  │  │   Events    │  │   Menu/Tray │            │
│  │   (tauri)   │  │   (tauri)   │  │   (tauri)   │            │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘            │
│         │                │                │                    │
│         └────────────────┴────────────────┘                    │
│                          │                                      │
│                   ┌──────┴──────┐                               │
│                   │ easyssh_core│ ←── FFI / Direct Import       │
│                   │   (Rust)    │                               │
│                   └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Tauri Command API Specification

### 3.1 Command Bindings

```rust
// src-tauri/src/commands.rs
// These commands wrap the core library for Tauri frontend consumption

use tauri::State;
use easyssh_core::{AppState, LiteError};

// ==================== Server Management ====================

#[tauri::command]
async fn get_servers(state: State<'_, AppState>) -> Result<Vec<ServerDto>, LiteError> {
    let servers = easyssh_core::get_servers(&state)?;
    Ok(servers.into_iter().map(ServerDto::from).collect())
}

#[tauri::command]
async fn add_server(
    state: State<'_, AppState>,
    server: NewServerDto,
) -> Result<(), LiteError> {
    let new_server = NewServer {
        id: uuid::Uuid::new_v4().to_string(),
        name: server.name,
        host: server.host,
        port: server.port,
        username: server.username,
        auth_type: server.auth_type,
        identity_file: server.identity_file,
        group_id: server.group_id,
        status: "unknown".to_string(),
    };
    easyssh_core::add_server(&state, &new_server)
}

#[tauri::command]
async fn update_server(
    state: State<'_, AppState>,
    server: UpdateServerDto,
) -> Result<(), LiteError> {
    let update = UpdateServer {
        id: server.id,
        name: server.name,
        host: server.host,
        port: server.port,
        username: server.username,
        auth_type: server.auth_type,
        identity_file: server.identity_file,
        group_id: server.group_id,
        status: server.status,
    };
    easyssh_core::update_server(&state, &update)
}

#[tauri::command]
async fn delete_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), LiteError> {
    easyssh_core::delete_server(&state, &id)
}

// ==================== Group Management ====================

#[tauri::command]
async fn get_groups(state: State<'_, AppState>) -> Result<Vec<GroupDto>, LiteError> {
    let groups = easyssh_core::get_groups(&state)?;
    Ok(groups.into_iter().map(GroupDto::from).collect())
}

#[tauri::command]
async fn add_group(
    state: State<'_, AppState>,
    name: String,
) -> Result<GroupDto, LiteError> {
    let id = uuid::Uuid::new_v4().to_string();
    let group = NewGroup { id: id.clone(), name: name.clone() };
    easyssh_core::add_group(&state, &group)?;
    Ok(GroupDto { id, name, created_at: chrono_now(), updated_at: chrono_now() })
}

#[tauri::command]
async fn update_group(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), LiteError> {
    let group = UpdateGroup { id, name };
    easyssh_core::update_group(&state, &group)
}

#[tauri::command]
async fn delete_group(state: State<'_, AppState>, id: String) -> Result<(), LiteError> {
    easyssh_core::delete_group(&state, &id)
}

// ==================== SSH Session Management ====================

#[tauri::command]
async fn ssh_connect(
    state: State<'_, AppState>,
    server_id: String,
    password: Option<String>,
) -> Result<String, LiteError> {
    easyssh_core::ssh_connect(&state, &server_id, password.as_deref()).await
}

#[tauri::command]
async fn ssh_execute(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<String, LiteError> {
    easyssh_core::ssh_execute(&state, &session_id, &command).await
}

#[tauri::command]
async fn ssh_disconnect(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), LiteError> {
    easyssh_core::ssh_disconnect(&state, &session_id).await
}

#[tauri::command]
fn ssh_list_sessions(state: State<'_, AppState>) -> Vec<String> {
    easyssh_core::ssh_list_sessions(&state)
}

#[tauri::command]
async fn write_shell_input(
    state: State<'_, AppState>,
    session_id: String,
    input: Vec<u8>,
) -> Result<(), LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.write_shell_input(&session_id, &input).await
}

#[tauri::command]
async fn interrupt_command(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), LiteError> {
    let ssh_manager = state.ssh_manager.lock().await;
    ssh_manager.interrupt_command(&session_id).await
}

// ==================== SFTP Operations (Standard+) ====================

#[tauri::command]
#[cfg(feature = "sftp")]
async fn sftp_list_dir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<Vec<FileEntryDto>, LiteError> {
    // Implementation wraps core SFTP
}

#[tauri::command]
#[cfg(feature = "sftp")]
async fn sftp_download(
    state: State<'_, AppState>,
    session_id: String,
    remote_path: String,
) -> Result<Vec<u8>, LiteError> {
    // Download file content
}

#[tauri::command]
#[cfg(feature = "sftp")]
async fn sftp_upload(
    state: State<'_, AppState>,
    session_id: String,
    remote_path: String,
    content: Vec<u8>,
) -> Result<(), LiteError> {
    // Upload file content
}

// ==================== Terminal Streaming (Standard+) ====================

use tauri::ipc::Channel;

#[tauri::command]
async fn start_terminal_stream(
    state: State<'_, AppState>,
    session_id: String,
    on_data: Channel<String>,
) -> Result<(), LiteError> {
    let mut ssh_manager = state.ssh_manager.lock().await;
    let mut rx = ssh_manager.execute_stream(&session_id, "").await?;

    // Spawn task to forward output to frontend
    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            let _ = on_data.send(chunk);
        }
    });

    Ok(())
}

// ==================== Settings & Config ====================

#[tauri::command]
fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock.as_ref().ok_or(LiteError::Config("DB not initialized".into()))?;
    db.get_config(&key)
}

#[tauri::command]
fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), LiteError> {
    let db_lock = state.db.lock().unwrap();
    let db = db_lock.as_ref().ok_or(LiteError::Config("DB not initialized".into()))?;
    db.set_config(&key, &value)
}
```

### 3.2 TypeScript Types (Auto-generated via Specta)

```typescript
// src/types/commands.ts - Generated from Rust
// Uses specta to generate TypeScript definitions from Rust types

export interface ServerDto {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authType: 'password' | 'key' | 'agent';
  identityFile: string | null;
  groupId: string | null;
  status: 'online' | 'offline' | 'unknown' | 'maintenance';
  createdAt: string;
  updatedAt: string;
}

export interface NewServerDto {
  name: string;
  host: string;
  port: number;
  username: string;
  authType: 'password' | 'key' | 'agent';
  identityFile?: string;
  groupId?: string;
}

export interface UpdateServerDto {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authType: 'password' | 'key' | 'agent';
  identityFile?: string;
  groupId?: string;
  status: string;
}

export interface GroupDto {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface FileEntryDto {
  name: string;
  path: string;
  fileType: 'file' | 'directory' | 'symlink';
  size: number;
  modifiedTime: number;
  permissions: string;
}

export interface SessionInfoDto {
  id: string;
  serverId: string;
  serverName: string;
  host: string;
  username: string;
  connectedAt: string;
  status: 'connecting' | 'connected' | 'error' | 'disconnected';
}

// Error types
export type LiteError =
  | { type: 'Database'; message: string }
  | { type: 'Crypto'; message: string }
  | { type: 'Keychain'; message: string }
  | { type: 'Terminal'; message: string }
  | { type: 'Ssh'; message: string }
  | { type: 'Config'; message: string }
  | { type: 'Io'; message: string }
  | { type: 'Json'; message: string }
  | { type: 'ServerNotFound'; id: string }
  | { type: 'GroupNotFound'; id: string }
  | { type: 'AuthFailed' }
  | { type: 'InvalidMasterPassword' };

// Command function signatures
export const commands = {
  getServers: () => invoke<ServerDto[]>('get_servers'),
  addServer: (server: NewServerDto) => invoke<void>('add_server', { server }),
  updateServer: (server: UpdateServerDto) => invoke<void>('update_server', { server }),
  deleteServer: (id: string) => invoke<void>('delete_server', { id }),

  getGroups: () => invoke<GroupDto[]>('get_groups'),
  addGroup: (name: string) => invoke<GroupDto>('add_group', { name }),
  updateGroup: (id: string, name: string) => invoke<void>('update_group', { id, name }),
  deleteGroup: (id: string) => invoke<void>('delete_group', { id }),

  sshConnect: (serverId: string, password?: string) => invoke<string>('ssh_connect', { serverId, password }),
  sshExecute: (sessionId: string, command: string) => invoke<string>('ssh_execute', { sessionId, command }),
  sshDisconnect: (sessionId: string) => invoke<void>('ssh_disconnect', { sessionId }),
  sshListSessions: () => invoke<string[]>('ssh_list_sessions'),

  writeShellInput: (sessionId: string, input: number[]) => invoke<void>('write_shell_input', { sessionId, input }),
  interruptCommand: (sessionId: string) => invoke<void>('interrupt_command', { sessionId }),

  sftpListDir: (sessionId: string, path: string) => invoke<FileEntryDto[]>('sftp_list_dir', { sessionId, path }),
  sftpDownload: (sessionId: string, remotePath: string) => invoke<number[]>('sftp_download', { sessionId, remotePath }),
  sftpUpload: (sessionId: string, remotePath: string, content: number[]) => invoke<void>('sftp_upload', { sessionId, remotePath, content }),

  getSetting: (key: string) => invoke<string | null>('get_setting', { key }),
  setSetting: (key: string, value: string) => invoke<void>('set_setting', { key, value }),
};
```

---

## 4. State Management Architecture

### 4.1 Zustand Store Design

```typescript
// src/stores/serverStore.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { commands, ServerDto, NewServerDto, UpdateServerDto } from '@/types/commands';

interface ServerStore {
  // State
  servers: ServerDto[];
  groups: GroupDto[];
  selectedServerId: string | null;
  isLoading: boolean;
  error: string | null;

  // Computed (via selectors)
  selectedServer: ServerDto | undefined;
  serversByGroup: Map<string, ServerDto[]>;
  ungroupedServers: ServerDto[];

  // Actions
  loadServers: () => Promise<void>;
  loadGroups: () => Promise<void>;
  selectServer: (id: string | null) => void;
  addServer: (server: NewServerDto) => Promise<void>;
  updateServer: (server: UpdateServerDto) => Promise<void>;
  deleteServer: (id: string) => Promise<void>;
  addGroup: (name: string) => Promise<void>;
  updateGroup: (id: string, name: string) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;
  moveServerToGroup: (serverId: string, groupId: string | null) => Promise<void>;
}

export const useServerStore = create<ServerStore>()(
  persist(
    (set, get) => ({
      // Initial state
      servers: [],
      groups: [],
      selectedServerId: null,
      isLoading: false,
      error: null,

      // Computed
      get selectedServer() {
        const { servers, selectedServerId } = get();
        return servers.find(s => s.id === selectedServerId);
      },

      get serversByGroup() {
        const { servers } = get();
        const map = new Map<string, ServerDto[]>();
        for (const server of servers) {
          const groupId = server.groupId ?? '__ungrouped';
          if (!map.has(groupId)) {
            map.set(groupId, []);
          }
          map.get(groupId)!.push(server);
        }
        return map;
      },

      get ungroupedServers() {
        const { servers } = get();
        return servers.filter(s => s.groupId === null);
      },

      // Actions
      loadServers: async () => {
        set({ isLoading: true, error: null });
        try {
          const servers = await commands.getServers();
          set({ servers, isLoading: false });
        } catch (error) {
          set({ error: String(error), isLoading: false });
        }
      },

      loadGroups: async () => {
        set({ isLoading: true, error: null });
        try {
          const groups = await commands.getGroups();
          set({ groups, isLoading: false });
        } catch (error) {
          set({ error: String(error), isLoading: false });
        }
      },

      selectServer: (id) => set({ selectedServerId: id }),

      addServer: async (server) => {
        try {
          await commands.addServer(server);
          await get().loadServers();
        } catch (error) {
          set({ error: String(error) });
          throw error;
        }
      },

      updateServer: async (server) => {
        try {
          await commands.updateServer(server);
          await get().loadServers();
        } catch (error) {
          set({ error: String(error) });
          throw error;
        }
      },

      deleteServer: async (id) => {
        try {
          await commands.deleteServer(id);
          set(state => ({
            servers: state.servers.filter(s => s.id !== id),
            selectedServerId: state.selectedServerId === id ? null : state.selectedServerId,
          }));
        } catch (error) {
          set({ error: String(error) });
          throw error;
        }
      },

      addGroup: async (name) => {
        try {
          const group = await commands.addGroup(name);
          set(state => ({ groups: [...state.groups, group] }));
        } catch (error) {
          set({ error: String(error) });
          throw error;
        }
      },

      updateGroup: async (id, name) => {
        try {
          await commands.updateGroup(id, name);
          set(state => ({
            groups: state.groups.map(g => g.id === id ? { ...g, name } : g),
          }));
        } catch (error) {
          set({ error: String(error) });
          throw error;
        }
      },

      deleteGroup: async (id) => {
        try {
          await commands.deleteGroup(id);
          set(state => ({
            groups: state.groups.filter(g => g.id !== id),
            servers: state.servers.map(s =>
              s.groupId === id ? { ...s, groupId: null } : s
            ),
          }));
        } catch (error) {
          set({ error: String(error) });
          throw error;
        }
      },

      moveServerToGroup: async (serverId, groupId) => {
        const server = get().servers.find(s => s.id === serverId);
        if (!server) return;

        const update: UpdateServerDto = {
          ...server,
          groupId: groupId ?? undefined,
        };

        await get().updateServer(update);
      },
    }),
    {
      name: 'easyssh-server-store',
      partialize: (state) => ({ selectedServerId: state.selectedServerId }),
    }
  )
);
```

```typescript
// src/stores/sessionStore.ts
import { create } from 'zustand';
import { commands } from '@/types/commands';
import { listen } from '@tauri-apps/api/event';

interface Session {
  id: string;
  serverId: string;
  serverName: string;
  host: string;
  username: string;
  status: 'connecting' | 'connected' | 'error' | 'disconnected';
  error?: string;
  connectedAt: Date;
  terminalOutput: string;
}

interface SessionStore {
  sessions: Session[];
  activeSessionId: string | null;
  isConnecting: boolean;

  // Actions
  connect: (serverId: string, password?: string) => Promise<string>;
  disconnect: (sessionId: string) => Promise<void>;
  disconnectAll: () => Promise<void>;
  setActiveSession: (sessionId: string | null) => void;
  writeInput: (sessionId: string, input: string) => Promise<void>;
  appendOutput: (sessionId: string, data: string) => void;
  interrupt: (sessionId: string) => Promise<void>;
  renameSession: (sessionId: string, name: string) => void;
  reorderSessions: (sessionIds: string[]) => void;
}

export const useSessionStore = create<SessionStore>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  isConnecting: false,

  connect: async (serverId, password) => {
    const { servers } = useServerStore.getState();
    const server = servers.find(s => s.id === serverId);
    if (!server) throw new Error('Server not found');

    set({ isConnecting: true });

    try {
      const sessionId = await commands.sshConnect(serverId, password);

      const session: Session = {
        id: sessionId,
        serverId,
        serverName: server.name,
        host: server.host,
        username: server.username,
        status: 'connected',
        connectedAt: new Date(),
        terminalOutput: `Connected to ${server.name} (${server.username}@${server.host})\n`,
      };

      set(state => ({
        sessions: [...state.sessions, session],
        activeSessionId: sessionId,
        isConnecting: false,
      }));

      // Start listening for terminal output
      listen<string>(`terminal:${sessionId}`, (event) => {
        get().appendOutput(sessionId, event.payload);
      });

      // Start the stream
      await commands.startTerminalStream(sessionId);

      return sessionId;
    } catch (error) {
      set({ isConnecting: false });
      throw error;
    }
  },

  disconnect: async (sessionId) => {
    try {
      await commands.sshDisconnect(sessionId);
    } finally {
      set(state => {
        const newSessions = state.sessions.filter(s => s.id !== sessionId);
        return {
          sessions: newSessions,
          activeSessionId: state.activeSessionId === sessionId
            ? newSessions[newSessions.length - 1]?.id ?? null
            : state.activeSessionId,
        };
      });
    }
  },

  disconnectAll: async () => {
    const { sessions } = get();
    await Promise.all(sessions.map(s => commands.sshDisconnect(s.id)));
    set({ sessions: [], activeSessionId: null });
  },

  setActiveSession: (sessionId) => set({ activeSessionId: sessionId }),

  writeInput: async (sessionId, input) => {
    const bytes = new TextEncoder().encode(input + '\n');
    await commands.writeShellInput(sessionId, Array.from(bytes));

    set(state => ({
      sessions: state.sessions.map(s =>
        s.id === sessionId
          ? { ...s, terminalOutput: s.terminalOutput + `$ ${input}\n` }
          : s
      ),
    }));
  },

  appendOutput: (sessionId, data) => {
    set(state => ({
      sessions: state.sessions.map(s =>
        s.id === sessionId
          ? { ...s, terminalOutput: s.terminalOutput + data }
          : s
      ),
    }));
  },

  interrupt: async (sessionId) => {
    await commands.interruptCommand(sessionId);
  },

  renameSession: (sessionId, name) => {
    set(state => ({
      sessions: state.sessions.map(s =>
        s.id === sessionId ? { ...s, serverName: name } : s
      ),
    }));
  },

  reorderSessions: (sessionIds) => {
    set(state => {
      const sessionMap = new Map(state.sessions.map(s => [s.id, s]));
      const reordered = sessionIds
        .map(id => sessionMap.get(id))
        .filter((s): s is Session => s !== undefined);
      return { sessions: reordered };
    });
  },
}));
```

```typescript
// src/stores/uiStore.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

type ProductMode = 'lite' | 'standard' | 'pro';
type WorkspaceMode = 'vault' | 'terminal' | 'sftp' | 'team';
type Theme = 'light' | 'dark' | 'system';

interface UIState {
  // Product configuration
  productMode: ProductMode;

  // Workspace state
  workspaceMode: WorkspaceMode;
  sidebarCollapsed: boolean;
  rightPanelOpen: boolean;
  rightPanelTab: 'details' | 'sftp' | 'monitor' | 'snippets';

  // Layout
  terminalLayout: 'single' | 'split-h' | 'split-v' | 'grid';
  activeLayoutId: string | null;

  // Appearance
  theme: Theme;
  fontSize: number;
  fontFamily: string;

  // Modals/Dialogs
  isAddServerOpen: boolean;
  isSettingsOpen: boolean;
  isCommandPaletteOpen: boolean;

  // Actions
  setProductMode: (mode: ProductMode) => void;
  setWorkspaceMode: (mode: WorkspaceMode) => void;
  toggleSidebar: () => void;
  toggleRightPanel: () => void;
  setRightPanelTab: (tab: UIState['rightPanelTab']) => void;
  setTerminalLayout: (layout: UIState['terminalLayout']) => void;
  setTheme: (theme: Theme) => void;
  setFontSize: (size: number) => void;
  openAddServer: () => void;
  closeAddServer: () => void;
  openSettings: () => void;
  closeSettings: () => void;
  openCommandPalette: () => void;
  closeCommandPalette: () => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      productMode: 'standard',
      workspaceMode: 'vault',
      sidebarCollapsed: false,
      rightPanelOpen: false,
      rightPanelTab: 'details',
      terminalLayout: 'single',
      activeLayoutId: null,
      theme: 'system',
      fontSize: 14,
      fontFamily: 'JetBrains Mono, Fira Code, Consolas, monospace',
      isAddServerOpen: false,
      isSettingsOpen: false,
      isCommandPaletteOpen: false,

      setProductMode: (mode) => set({ productMode: mode }),
      setWorkspaceMode: (mode) => set({ workspaceMode: mode }),
      toggleSidebar: () => set(state => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      toggleRightPanel: () => set(state => ({ rightPanelOpen: !state.rightPanelOpen })),
      setRightPanelTab: (tab) => set({ rightPanelTab: tab }),
      setTerminalLayout: (layout) => set({ terminalLayout: layout }),
      setTheme: (theme) => set({ theme }),
      setFontSize: (fontSize) => set({ fontSize }),
      openAddServer: () => set({ isAddServerOpen: true }),
      closeAddServer: () => set({ isAddServerOpen: false }),
      openSettings: () => set({ isSettingsOpen: true }),
      closeSettings: () => set({ isSettingsOpen: false }),
      openCommandPalette: () => set({ isCommandPaletteOpen: true }),
      closeCommandPalette: () => set({ isCommandPaletteOpen: false }),
    }),
    {
      name: 'easyssh-ui-store',
      partialize: (state) => ({
        productMode: state.productMode,
        sidebarCollapsed: state.sidebarCollapsed,
        theme: state.theme,
        fontSize: state.fontSize,
        fontFamily: state.fontFamily,
        terminalLayout: state.terminalLayout,
      }),
    }
  )
);
```

---

## 5. Component Architecture

### 5.1 Directory Structure

```
src/
├── app/
│   ├── App.tsx                 # Main app component with providers
│   ├── routes.tsx              # React Router configuration
│   ├── providers/
│   │   ├── ThemeProvider.tsx   # Theme/color mode management
│   │   └── ToastProvider.tsx   # Notifications
│   └── hooks/
│       ├── useTauri.ts         # Tauri API wrappers
│       ├── useTerminal.ts      # xterm.js integration
│       └── useAsync.ts         # Async operation helpers
│
├── features/
│   ├── servers/
│   │   ├── components/
│   │   │   ├── ServerList.tsx
│   │   │   ├── ServerItem.tsx
│   │   │   ├── ServerGroup.tsx
│   │   │   ├── AddServerDialog.tsx
│   │   │   └── EditServerDialog.tsx
│   │   ├── hooks/
│   │   │   └── useServerActions.ts
│   │   └── index.ts
│   │
│   ├── terminals/
│   │   ├── components/
│   │   │   ├── TerminalWorkspace.tsx
│   │   │   ├── TerminalTab.tsx
│   │   │   ├── TerminalPane.tsx
│   │   │   ├── XTermWrapper.tsx
│   │   │   └── SplitLayout.tsx
│   │   ├── hooks/
│   │   │   ├── useTerminal.ts
│   │   │   └── useSplitter.ts
│   │   └── index.ts
│   │
│   ├── sftp/
│   │   ├── components/
│   │   │   ├── SftpBrowser.tsx
│   │   │   ├── FileList.tsx
│   │   │   ├── FileItem.tsx
│   │   │   └── TransferProgress.tsx
│   │   └── index.ts
│   │
│   ├── layout/
│   │   ├── components/
│   │   │   ├── AppShell.tsx
│   │   │   ├── Sidebar.tsx
│   │   │   ├── TopBar.tsx
│   │   │   ├── RightPanel.tsx
│   │   │   └── CommandPalette.tsx
│   │   └── index.ts
│   │
│   └── settings/
│       ├── components/
│       │   ├── SettingsDialog.tsx
│       │   ├── GeneralSettings.tsx
│       │   ├── TerminalSettings.tsx
│       │   └── SecuritySettings.tsx
│       └── index.ts
│
├── components/
│   ├── ui/                     # Reusable UI primitives
│   │   ├── Button.tsx
│   │   ├── Input.tsx
│   │   ├── Select.tsx
│   │   ├── Dialog.tsx
│   │   ├── Tabs.tsx
│   │   ├── Dropdown.tsx
│   │   ├── Badge.tsx
│   │   ├── Card.tsx
│   │   ├── Tooltip.tsx
│   │   └── Toast.tsx
│   ├── icons/
│   │   └── index.ts
│   └── feedback/
│       ├── LoadingSpinner.tsx
│       ├── ErrorBoundary.tsx
│       └── EmptyState.tsx
│
├── stores/
│   ├── serverStore.ts
│   ├── sessionStore.ts
│   ├── uiStore.ts
│   └── settingsStore.ts
│
├── lib/
│   ├── api/
│   │   └── tauri.ts            # Tauri invoke wrappers
│   ├── terminal/
│   │   └── xterm.ts            # xterm.js setup
│   ├── crypto/
│   │   └── secureStorage.ts    # Keychain wrappers
│   └── utils/
│       ├── format.ts
│       └── validation.ts
│
├── styles/
│   ├── tokens.css              # CSS variables (design tokens)
│   ├── globals.css
│   └── xterm.css               # Terminal-specific styles
│
└── types/
    ├── commands.ts             # Auto-generated from Rust
    ├── models.ts               # Additional TS types
    └── index.ts
```

### 5.2 Key Component Specifications

```typescript
// src/features/terminals/components/XTermWrapper.tsx
import { useEffect, useRef } from 'react';
import { Terminal } from 'xterm';
import { WebglAddon } from 'xterm-addon-webgl';
import { FitAddon } from 'xterm-addon-fit';
import { SearchAddon } from 'xterm-addon-search';
import { useTerminal } from '../hooks/useTerminal';

interface XTermWrapperProps {
  sessionId: string;
  onData?: (data: string) => void;
  onResize?: (cols: number, rows: number) => void;
  className?: string;
}

export const XTermWrapper: React.FC<XTermWrapperProps> = ({
  sessionId,
  onData,
  onResize,
  className,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    // Initialize xterm
    const terminal = new Terminal({
      rendererType: 'webgl',
      fontSize: 14,
      fontFamily: 'JetBrains Mono, Fira Code, monospace',
      cursorBlink: true,
      cursorStyle: 'bar',
      scrollback: 10000,
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        selection: '#264f78',
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
      },
    });

    // Load addons
    const webglAddon = new WebglAddon();
    terminal.loadAddon(webglAddon);

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    fitAddonRef.current = fitAddon;

    const searchAddon = new SearchAddon();
    terminal.loadAddon(searchAddon);

    // Mount terminal
    terminal.open(containerRef.current);
    fitAddon.fit();

    terminalRef.current = terminal;

    // Handle input
    terminal.onData((data) => {
      onData?.(data);
    });

    // Handle resize
    terminal.onResize(({ cols, rows }) => {
      onResize?.(cols, rows);
    });

    // Cleanup
    return () => {
      terminal.dispose();
      webglAddon.dispose();
    };
  }, []);

  // Handle incoming data
  useEffect(() => {
    const terminal = terminalRef.current;
    if (!terminal) return;

    // Subscribe to session output
    const { session } = useSessionStore.getState();
    const currentSession = session?.sessions.find(s => s.id === sessionId);

    if (currentSession?.terminalOutput) {
      terminal.write(currentSession.terminalOutput);
    }
  }, [sessionId]);

  return (
    <div
      ref={containerRef}
      className={`terminal-container ${className || ''}`}
      style={{ width: '100%', height: '100%' }}
    />
  );
};
```

---

## 6. CI/CD Pipeline Configuration

### 6.1 GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI/CD Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  NODE_VERSION: '20'
  RUST_VERSION: '1.75'

jobs:
  # ==================== RUST CHECKS ====================
  rust-check:
    name: Rust Lint & Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache Cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: 'core -> target'

      - name: Check Formatting
        run: cargo fmt -- --check
        working-directory: ./core

      - name: Run Clippy
        run: cargo clippy --all-features -- -D warnings
        working-directory: ./core

      - name: Run Tests
        run: cargo test --all-features
        working-directory: ./core

      - name: Build Core
        run: cargo build --release --all-features
        working-directory: ./core

  # ==================== FRONTEND CHECKS ====================
  frontend-check:
    name: Frontend Lint & Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: 'npm'

      - name: Install Dependencies
        run: npm ci

      - name: Type Check
        run: npm run typecheck

      - name: Lint
        run: npm run lint

      - name: Test
        run: npm run test:unit -- --coverage

      - name: Build
        run: npm run build

  # ==================== TAURI BUILD ====================
  tauri-build:
    name: Tauri Build (${{ matrix.platform }})
    needs: [rust-check, frontend-check]
    strategy:
      fail-fast: false
      matrix:
        platform: [ubuntu-latest, windows-latest, macos-latest]
        include:
          - platform: ubuntu-latest
            args: ''
          - platform: windows-latest
            args: ''
          - platform: macos-latest
            args: '--target universal-apple-darwin'
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

      - name: Install Linux Dependencies
        if: matrix.platform == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Install Frontend Dependencies
        run: npm ci

      - name: Build Tauri
        uses: tauri-apps/tauri-action@v0
        with:
          args: ${{ matrix.args }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  # ==================== VISUAL REGRESSION ====================
  chromatic:
    name: Visual Regression (Chromatic)
    needs: frontend-check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}

      - name: Install Dependencies
        run: npm ci

      - name: Publish to Chromatic
        uses: chromaui/action@latest
        with:
          projectToken: ${{ secrets.CHROMATIC_PROJECT_TOKEN }}

  # ==================== SECURITY SCAN ====================
  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run Cargo Audit
        run: |
          cargo install cargo-audit
          cargo audit
        working-directory: ./core

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}

      - name: Run NPM Audit
        run: npm audit --audit-level=high
```

### 6.2 Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    name: Release ${{ matrix.platform }}
    strategy:
      fail-fast: false
      matrix:
        platform: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Linux Dependencies
        if: matrix.platform == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev

      - name: Build and Release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'EasySSH ${{ github.ref_name }}'
          releaseBody: 'See the assets to download this version.'
          releaseDraft: true
          prerelease: false
```

---

## 7. Team Coordination Plan

### 7.1 Role Assignments

| Role | Responsibilities | Current Lead |
|------|------------------|--------------|
| **Tech Lead** (You) | Architecture decisions, integration, code review, CI/CD | @tech-lead |
| **Rust Core Dev** | Core library features, SSH, crypto, database | @backend-dev |
| **Frontend Dev** | React components, Tauri integration, xterm.js | @frontend-dev |
| **UI/UX Designer** | Design system, Figma, design tokens | @uiux-designer |
| **QA Engineer** | Testing, automation, quality gates | @qa-engineer |

### 7.2 Work Streams

```
Phase 1: Foundation (Weeks 1-2)
├── @frontend-dev: Tauri project setup, Vite+React scaffolding
├── @backend-dev: FFI bindings, command exports
├── @tech-lead: API specification, store architecture
└── @uiux-designer: Design tokens, component specs

Phase 2: Core Features (Weeks 3-6)
├── @frontend-dev: Server management UI, forms, validation
├── @backend-dev: SFTP operations, terminal streaming
├── @tech-lead: Integration testing, performance optimization
└── @qa-engineer: E2E test suite, Playwright setup

Phase 3: Terminal Integration (Weeks 7-9)
├── @frontend-dev: xterm.js integration, split panes
├── @backend-dev: Session multiplexing, output streaming
├── @tech-lead: Layout system, state synchronization
└── @uiux-designer: Terminal chrome, interaction polish

Phase 4: Polish & Pro Features (Weeks 10-12)
├── @frontend-dev: Team views, RBAC UI, admin console
├── @backend-dev: Audit logging, SSO integration
├── @tech-lead: Feature flags, release management
└── @qa-engineer: Security testing, load testing
```

### 7.3 Code Review Checklist

```markdown
## PR Review Checklist

### Rust Code
- [ ] Error handling uses proper `thiserror` types
- [ ] No `unwrap()` or `expect()` in production code
- [ ] Async code doesn't hold locks across await points
- [ ] All public APIs have doc comments
- [ ] Tests added for new functionality
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` has been run

### TypeScript/React Code
- [ ] Props interfaces are explicitly defined
- [ ] No `any` types (use `unknown` with type guards)
- [ ] Components use `React.memo` where appropriate
- [ ] Custom hooks extracted from component logic
- [ ] Error boundaries for error handling
- [ ] Accessibility attributes (aria-label, role)
- [ ] `tsc --noEmit` passes
- [ ] ESLint passes
- [ ] Tests added (unit + integration)

### Integration
- [ ] Tauri commands properly typed
- [ ] Frontend uses generated types from Rust
- [ ] Error messages propagate correctly
- [ ] Loading states handled
- [ ] Race conditions prevented

### UI/UX
- [ ] Design tokens used (no hardcoded colors/sizes)
- [ ] Responsive layout
- [ ] Dark mode support
- [ ] Loading states visible
- [ ] Error states informative
- [ ] Keyboard navigation works
```

### 7.4 Communication Channels

| Channel | Purpose | Updates |
|---------|---------|---------|
| #easyssh-general | Daily standups, general discussion | Daily |
| #easyssh-rust | Core library development | As needed |
| #easyssh-frontend | React, Tauri, UI components | As needed |
| #easyssh-design | Figma, design tokens, UX | Weekly |
| #easyssh-releases | Release planning, changelog | Per sprint |

---

## 8. Implementation Roadmap

### 8.1 Immediate Actions (Week 1)

1. **Create Tauri Project Structure**
   ```bash
   npm create tauri-app@latest easyssh-tauri
   cd easyssh-tauri
   npm install
   ```

2. **Add Dependencies**
   ```bash
   npm install zustand @xterm/xterm xterm-addon-webgl xterm-addon-fit xterm-addon-search
   npm install -D typescript @types/node @types/react @types/react-dom
   npm install -D tailwindcss postcss autoprefixer
   npm install -D @tauri-apps/cli @tauri-apps/api
   ```

3. **Setup Type Generation**
   - Add `specta` to core library
   - Create build script for TypeScript types
   - Document type generation process

4. **Initialize Stores**
   - Create store files (server, session, ui)
   - Add persistence configuration
   - Write unit tests for store logic

### 8.2 Short-term Goals (Weeks 2-4)

- [ ] Server CRUD UI complete
- [ ] Group management working
- [ ] Terminal connection established
- [ ] Basic xterm.js integration
- [ ] Settings persistence

### 8.3 Medium-term Goals (Weeks 5-8)

- [ ] Multi-tab terminal support
- [ ] Split pane layout
- [ ] SFTP file browser
- [ ] Server monitoring widgets
- [ ] Command palette

### 8.4 Long-term Goals (Weeks 9-12)

- [ ] Team collaboration features
- [ ] Audit logging UI
- [ ] RBAC implementation
- [ ] SSO integration
- [ ] Mobile companion app planning

---

## 9. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Tauri build issues on Windows | Medium | High | Early CI setup, cross-platform testing |
| xterm.js performance at scale | Medium | Medium | WebGL addon, output throttling |
| SSH connection multiplexing bugs | Low | High | Extensive testing, fallback mechanisms |
| Design system adoption resistance | Low | Medium | Clear documentation, code review enforcement |
| Team bandwidth constraints | Medium | High | Prioritize features, phased releases |

---

## 10. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Build time | < 5 minutes | CI pipeline duration |
| Test coverage | > 80% | Codecov report |
| Terminal latency | < 16ms | Input to render time |
| Bundle size | < 30MB | Release artifact size |
| Startup time | < 3 seconds | Time to interactive |
| Memory usage | < 200MB | 10 active sessions |

---

## Appendix A: Design Tokens Reference

```css
/* styles/tokens.css */
:root {
  /* Colors */
  --color-bg-primary: #1e1e1e;
  --color-bg-secondary: #252526;
  --color-bg-tertiary: #2d2d30;
  --color-bg-hover: #2a2d2e;
  --color-bg-active: #37373d;

  --color-text-primary: #d4d4d4;
  --color-text-secondary: #a0a0a0;
  --color-text-muted: #6e6e6e;
  --color-text-link: #3794ff;

  --color-border: #454545;
  --color-border-hover: #4f4f4f;

  --color-accent: #007acc;
  --color-accent-hover: #0098ff;
  --color-success: #89d185;
  --color-warning: #cca700;
  --color-error: #f48771;

  /* Spacing */
  --space-xs: 4px;
  --space-sm: 8px;
  --space-md: 16px;
  --space-lg: 24px;
  --space-xl: 32px;

  /* Typography */
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
  --font-size-xs: 11px;
  --font-size-sm: 12px;
  --font-size-md: 14px;
  --font-size-lg: 16px;
  --font-size-xl: 20px;

  /* Layout */
  --sidebar-width: 280px;
  --sidebar-collapsed-width: 48px;
  --topbar-height: 40px;
  --panel-width: 300px;

  /* Shadows */
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 8px rgba(0, 0, 0, 0.3);
  --shadow-lg: 0 8px 16px rgba(0, 0, 0, 0.4);

  /* Transitions */
  --transition-fast: 150ms ease;
  --transition-normal: 250ms ease;
  --transition-slow: 350ms ease;
}
```

---

*End of Integration Architecture Plan*
