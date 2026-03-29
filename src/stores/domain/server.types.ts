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

export interface ServerUIState {
  isLoading: boolean;
  error: string | null;
  searchQuery: string;
  selectedServerId: string | null;
}

export const initialServerUIState: ServerUIState = {
  isLoading: false,
  error: null,
  searchQuery: '',
  selectedServerId: null,
};
