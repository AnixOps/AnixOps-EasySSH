import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Group, NewGroup, NewServer, Server, UpdateGroup, UpdateServer } from '../types';

interface ServerState {
  servers: Server[];
  groups: Group[];
  isLoading: boolean;
  error: string | null;
  fetchServers: () => Promise<void>;
  fetchGroups: () => Promise<void>;
  addServer: (server: NewServer) => Promise<void>;
  updateServer: (server: UpdateServer) => Promise<void>;
  deleteServer: (id: string) => Promise<void>;
  connectServerNative: (id: string) => Promise<void>;
  addGroup: (group: NewGroup) => Promise<void>;
  updateGroup: (group: UpdateGroup) => Promise<void>;
  deleteGroup: (id: string) => Promise<void>;
}

export const useServerStore = create<ServerState>((set, get) => ({
  servers: [],
  groups: [],
  isLoading: false,
  error: null,

  fetchServers: async () => {
    set({ isLoading: true, error: null });
    try {
      const servers = await invoke<Server[]>('get_servers');
      set({ servers, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  fetchGroups: async () => {
    set({ error: null });
    try {
      const groups = await invoke<Group[]>('get_groups');
      set({ groups });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addServer: async (server: NewServer) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('add_server', { server });
      await get().fetchServers();
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  updateServer: async (server: UpdateServer) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('update_server', { server });
      await get().fetchServers();
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  deleteServer: async (id: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('delete_server', { id });
      await get().fetchServers();
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  connectServerNative: async (id: string) => {
    try {
      await invoke('connect_server', { id });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addGroup: async (group: NewGroup) => {
    try {
      await invoke('add_group', { group });
      await get().fetchGroups();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  updateGroup: async (group: UpdateGroup) => {
    try {
      await invoke('update_group', { group });
      await get().fetchGroups();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteGroup: async (id: string) => {
    try {
      await invoke('delete_group', { id });
      await get().fetchGroups();
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
