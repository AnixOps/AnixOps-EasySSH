/**
 * Server & Session State Management using Zustand
 * @module stores/serverStore
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Server, ServerGroup, Session, SessionState } from '../types/index.js';
import { generateId } from '../utils/index.js';

// =============================================================================
// Types
// =============================================================================

/**
 * Server Store State
 */
interface ServerState {
  // Servers
  servers: Server[];
  addServer: (server: Omit<Server, 'id' | 'createdAt' | 'updatedAt'>) => Server;
  updateServer: (id: string, updates: Partial<Server>) => void;
  removeServer: (id: string) => void;
  getServerById: (id: string) => Server | undefined;
  getServersByGroup: (groupId: string | null) => Server[];
  searchServers: (query: string) => Server[];

  // Groups
  groups: ServerGroup[];
  addGroup: (group: Omit<ServerGroup, 'id' | 'createdAt'>) => ServerGroup;
  updateGroup: (id: string, updates: Partial<ServerGroup>) => void;
  removeGroup: (id: string) => void;
  moveGroup: (id: string, parentId: string | null) => void;
  reorderGroups: (orderedIds: string[]) => void;

  // Sessions
  sessions: Session[];
  createSession: (serverId: string) => string;
  updateSessionState: (sessionId: string, state: SessionState, error?: string) => void;
  closeSession: (sessionId: string) => void;
  getSessionById: (sessionId: string) => Session | undefined;
  getSessionsByServer: (serverId: string) => Session[];
  getActiveSessions: () => Session[];

  // Selection
  selectedServerId: string | null;
  selectedSessionId: string | null;
  setSelectedServer: (id: string | null) => void;
  setSelectedSession: (id: string | null) => void;

  // Import/Export
  exportData: () => { servers: Server[]; groups: ServerGroup[] };
  importData: (data: { servers: Server[]; groups: ServerGroup[] }) => void;
}

// =============================================================================
// Helper Functions
// =============================================================================

/*
 * Build group hierarchy from flat list - helper for future use
 * Uncomment when needed:
function buildGroupTree(
  groups: ServerGroup[],
  parentId: string | null = null
): ServerGroup[] {
  return groups
    .filter((g) => g.parentId === parentId)
    .sort((a, b) => a.order - b.order);
}
*/

// =============================================================================
// Store Creation
// =============================================================================

export const useServerStore = create<ServerState>()(
  persist(
    (set, get) => ({
      // Servers
      servers: [],
      addServer: (serverData) => {
        const server: Server = {
          ...serverData,
          id: generateId(),
          createdAt: Date.now(),
          updatedAt: Date.now(),
        };
        set((state) => ({ servers: [...state.servers, server] }));
        return server;
      },
      updateServer: (id, updates) => {
        set((state) => ({
          servers: state.servers.map((s) =>
            s.id === id ? { ...s, ...updates, updatedAt: Date.now() } : s
          ),
        }));
      },
      removeServer: (id) => {
        set((state) => ({
          servers: state.servers.filter((s) => s.id !== id),
          sessions: state.sessions.filter((s) => s.serverId !== id),
          selectedServerId:
            state.selectedServerId === id ? null : state.selectedServerId,
        }));
      },
      getServerById: (id) => get().servers.find((s) => s.id === id),
      getServersByGroup: (groupId) =>
        get().servers.filter((s) => s.groupId === groupId),
      searchServers: (query) => {
        const lowerQuery = query.toLowerCase();
        return get().servers.filter(
          (s) =>
            s.name.toLowerCase().includes(lowerQuery) ||
            s.host.toLowerCase().includes(lowerQuery) ||
            s.username.toLowerCase().includes(lowerQuery) ||
            s.tags.some((t) => t.toLowerCase().includes(lowerQuery))
        );
      },

      // Groups
      groups: [],
      addGroup: (groupData) => {
        const maxOrder = Math.max(
          0,
          ...get().groups
            .filter((g) => g.parentId === groupData.parentId)
            .map((g) => g.order)
        );
        const group: ServerGroup = {
          ...groupData,
          id: generateId(),
          order: maxOrder + 1,
          createdAt: Date.now(),
        };
        set((state) => ({ groups: [...state.groups, group] }));
        return group;
      },
      updateGroup: (id, updates) => {
        set((state) => ({
          groups: state.groups.map((g) =>
            g.id === id ? { ...g, ...updates } : g
          ),
        }));
      },
      removeGroup: (id) => {
        // Move children to parent
        const group = get().groups.find((g) => g.id === id);
        if (group) {
          set((state) => ({
            groups: state.groups.filter((g) => g.id !== id),
            servers: state.servers.map((s) =>
              s.groupId === id ? { ...s, groupId: group.parentId } : s
            ),
          }));
        }
      },
      moveGroup: (id, parentId) => {
        set((state) => ({
          groups: state.groups.map((g) =>
            g.id === id ? { ...g, parentId } : g
          ),
        }));
      },
      reorderGroups: (orderedIds) => {
        set((state) => ({
          groups: state.groups.map((g) => ({
            ...g,
            order: orderedIds.indexOf(g.id),
          })),
        }));
      },

      // Sessions
      sessions: [],
      createSession: (serverId) => {
        const server = get().getServerById(serverId);
        if (!server) throw new Error(`Server ${serverId} not found`);

        const existingSessions = get().getSessionsByServer(serverId);
        const index = existingSessions.length + 1;
        const displayName =
          index === 1 ? server.name : `${server.name} (${index})`;

        const session: Session = {
          id: generateId(),
          serverId,
          displayName,
          state: 'connecting',
          startedAt: Date.now(),
          lastActivityAt: Date.now(),
          index,
        };

        set((state) => ({
          sessions: [...state.sessions, session],
          selectedSessionId: session.id,
        }));

        return session.id;
      },
      updateSessionState: (sessionId, newState, error) => {
        set((state) => ({
          sessions: state.sessions.map((s) =>
            s.id === sessionId
              ? { ...s, state: newState, error, lastActivityAt: Date.now() }
              : s
          ),
        }));
      },
      closeSession: (sessionId) => {
        set((state) => ({
          sessions: state.sessions.filter((s) => s.id !== sessionId),
          selectedSessionId:
            state.selectedSessionId === sessionId
              ? null
              : state.selectedSessionId,
        }));
      },
      getSessionById: (sessionId) =>
        get().sessions.find((s) => s.id === sessionId),
      getSessionsByServer: (serverId) =>
        get().sessions.filter((s) => s.serverId === serverId),
      getActiveSessions: () =>
        get().sessions.filter((s) => s.state === 'connected'),

      // Selection
      selectedServerId: null,
      selectedSessionId: null,
      setSelectedServer: (id) => set({ selectedServerId: id }),
      setSelectedSession: (id) => set({ selectedSessionId: id }),

      // Import/Export
      exportData: () => ({
        servers: get().servers,
        groups: get().groups,
      }),
      importData: (data) => {
        set({
          servers: data.servers,
          groups: data.groups,
        });
      },
    }),
    {
      name: 'easyssh-servers',
      partialize: (state) => ({
        servers: state.servers,
        groups: state.groups,
        selectedServerId: state.selectedServerId,
      }),
    }
  )
);

// =============================================================================
// Selector Hooks
// =============================================================================

/**
 * Get servers with optional filtering
 */
export const useServers = (groupId?: string | null) =>
  useServerStore((state) =>
    groupId !== undefined
      ? state.getServersByGroup(groupId)
      : state.servers
  );

/**
 * Get groups
 */
export const useGroups = () => useServerStore((state) => state.groups);

/**
 * Get group tree structure
 */
export const useGroupTree = () =>
  useServerStore((state) => {
    const buildTree = (parentId: string | null = null): ServerGroup[] => {
      return state.groups
        .filter((g) => g.parentId === parentId)
        .sort((a, b) => a.order - b.order)
        .map((g) => ({ ...g }));
    };
    return buildTree();
  });

/**
 * Get active sessions
 */
export const useSessions = () =>
  useServerStore((state) => ({
    sessions: state.sessions,
    create: state.createSession,
    close: state.closeSession,
    getById: state.getSessionById,
    active: state.getActiveSessions(),
  }));

/**
 * Get selection state
 */
export const useSelection = () =>
  useServerStore((state) => ({
    selectedServer: state.selectedServerId,
    selectedSession: state.selectedSessionId,
    setServer: state.setSelectedServer,
    setSession: state.setSelectedSession,
    getServer: () =>
      state.selectedServerId
        ? state.getServerById(state.selectedServerId)
        : undefined,
    getSession: () =>
      state.selectedSessionId
        ? state.getSessionById(state.selectedSessionId)
        : undefined,
  }));
