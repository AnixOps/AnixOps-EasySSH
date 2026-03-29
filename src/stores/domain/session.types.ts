export interface SessionUIState {
  isConnecting: boolean;
  error: string | null;
  showTerminalPanel: boolean;
}

export const initialSessionUIState: SessionUIState = {
  isConnecting: false,
  error: null,
  showTerminalPanel: true,
};
