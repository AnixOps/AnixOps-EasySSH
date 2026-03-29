export interface TerminalSession {
  id: string;
  serverId: string;
  serverName: string;
  sessionId: string;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}

export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  theme: 'dark' | 'light';
}
