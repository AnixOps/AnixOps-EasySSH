import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { useServerStore } from './serverStore';

export interface TerminalSession {
  id: string;
  serverId: string;
  serverName: string;
  sessionId: string;
}

interface SessionState {
  terminalSessions: TerminalSession[];
  activeSessionId: string | null;
  sshConnect: (serverId: string, password?: string) => Promise<string>;
  sshDisconnect: (terminalSessionId: string) => Promise<void>;
  sshDisconnectBySessionId: (sessionId: string) => Promise<void>;
  setActiveSession: (sessionId: string | null) => void;
  clearAllSessions: () => Promise<void>;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  terminalSessions: [],
  activeSessionId: null,

  sshConnect: async (serverId: string, password?: string) => {
    try {
      const sshSessionId = await invoke<string>('ssh_connect', { id: serverId, password });
      const server = useServerStore.getState().servers.find((item) => item.id === serverId);
      const terminalSessionId = crypto.randomUUID();

      set((state) => ({
        terminalSessions: [
          ...state.terminalSessions,
          {
            id: terminalSessionId,
            serverId,
            serverName: server?.name || 'Unknown',
            sessionId: sshSessionId,
          },
        ],
        activeSessionId: terminalSessionId,
      }));

      return terminalSessionId;
    } catch (e) {
      throw e;
    }
  },

  sshDisconnect: async (terminalSessionId: string) => {
    const session = get().terminalSessions.find((item) => item.id === terminalSessionId);
    if (!session) return;

    try {
      await invoke('ssh_disconnect', { sessionId: session.sessionId });
    } catch (e) {
      // Silently ignore disconnect errors during cleanup
    }

    set((state) => {
      const remaining = state.terminalSessions.filter((item) => item.id !== terminalSessionId);
      return {
        terminalSessions: remaining,
        activeSessionId: state.activeSessionId === terminalSessionId ? remaining[0]?.id || null : state.activeSessionId,
      };
    });
  },

  sshDisconnectBySessionId: async (sessionId: string) => {
    const session = get().terminalSessions.find((item) => item.sessionId === sessionId);
    if (!session) return;
    await get().sshDisconnect(session.id);
  },

  setActiveSession: (sessionId: string | null) => set({ activeSessionId: sessionId }),

  clearAllSessions: async () => {
    const sessions = [...get().terminalSessions];
    for (const session of sessions) {
      try {
        await invoke('ssh_disconnect', { sessionId: session.sessionId });
      } catch (e) {
        // Silently ignore disconnect errors during bulk cleanup
      }
    }

    set({ terminalSessions: [], activeSessionId: null });
  },
}));
