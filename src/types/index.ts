export interface Server {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_type: 'agent' | 'key' | 'password';
  identity_file?: string;
  group_id?: string;
  status: 'online' | 'offline' | 'warning' | 'unknown';
  created_at: string;
  updated_at: string;
}

export interface Group {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface NewServer {
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

export interface UpdateServer {
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

export interface NewGroup {
  id: string;
  name: string;
}

export interface UpdateGroup {
  id: string;
  name: string;
}
