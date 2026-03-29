import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface Team {
  id: string;
  name: string;
  memberCount: number;
  role: 'owner' | 'admin' | 'member' | 'viewer';
}

export interface AuditLog {
  id: string;
  action: string;
  user: string;
  target: string;
  timestamp: string;
}

interface TeamState {
  teams: Team[];
  auditLogs: AuditLog[];
  activeTab: 'team' | 'audit' | 'sso' | 'members';
  setActiveTab: (tab: 'team' | 'audit' | 'sso' | 'members') => void;
}

export const useTeamStore = create<TeamState>()(
  persist(
    (set) => ({
      teams: [],
      auditLogs: [],
      activeTab: 'team',
      setActiveTab: (tab) => set({ activeTab: tab }),
    }),
    {
      name: 'easyssh-team-state',
    }
  )
);
