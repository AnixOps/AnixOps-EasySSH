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

export interface TeamMember {
  id: string;
  userId: string;
  teamId: string;
  role: Team['role'];
  joinedAt: string;
}
