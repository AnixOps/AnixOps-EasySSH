import type { AuthType, ServerStatus } from './common.types';

export interface Server {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_type: AuthType;
  identity_file?: string;
  group_id?: string;
  status: ServerStatus;
  created_at: string;
  updated_at: string;
}

export interface Group {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

// DTOs for creating/updating
export interface CreateServerDTO {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_type: string;
  identity_file?: string;
  group_id?: string;
  status: string;
}

export interface UpdateServerDTO extends CreateServerDTO {}

export interface CreateGroupDTO {
  id: string;
  name: string;
}

export interface UpdateGroupDTO extends CreateGroupDTO {}

// Backward compatibility aliases
export type NewServer = CreateServerDTO;
export type UpdateServer = UpdateServerDTO;
export type NewGroup = CreateGroupDTO;
export type UpdateGroup = UpdateGroupDTO;
