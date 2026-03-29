import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface CommandHistoryItem {
  id: string;
  command: string;
  timestamp: number;
  sessionId: string;
}

interface CommandHistoryState {
  // 按会话存储命令历史
  histories: Record<string, CommandHistoryItem[]>;
  // 全局命令历史
  globalHistory: CommandHistoryItem[];
  // 当前会话ID
  currentSessionId: string | null;

  // 添加命令到历史
  addToHistory: (sessionId: string, command: string) => void;

  // 获取会话命令历史
  getSessionHistory: (sessionId: string) => CommandHistoryItem[];

  // 获取全局命令历史
  getGlobalHistory: () => CommandHistoryItem[];

  // 清空会话历史
  clearSessionHistory: (sessionId: string) => void;

  // 清空所有历史
  clearAllHistory: () => void;

  // 设置当前会话
  setCurrentSession: (sessionId: string | null) => void;
}

export const useCommandHistoryStore = create<CommandHistoryState>()(
  persist(
    (set, get) => ({
      histories: {},
      globalHistory: [],
      currentSessionId: null,

      addToHistory: (sessionId: string, command: string) => {
        const trimmedCommand = command.trim();
        if (!trimmedCommand) return;

        const newItem: CommandHistoryItem = {
          id: crypto.randomUUID(),
          command: trimmedCommand,
          timestamp: Date.now(),
          sessionId,
        };

        set(state => {
          // 添加到会话历史
          const sessionHistory = state.histories[sessionId] || [];
          // 避免重复连续命令
          const lastCommand = sessionHistory[sessionHistory.length - 1];
          if (lastCommand && lastCommand.command === trimmedCommand) {
            return state;
          }

          const newSessionHistory = [...sessionHistory, newItem].slice(-1000); // 保留最近1000条

          // 添加到全局历史
          const newGlobalHistory = [...state.globalHistory, newItem].slice(-1000);

          return {
            histories: {
              ...state.histories,
              [sessionId]: newSessionHistory,
            },
            globalHistory: newGlobalHistory,
          };
        });
      },

      getSessionHistory: (sessionId: string) => {
        return get().histories[sessionId] || [];
      },

      getGlobalHistory: () => {
        return get().globalHistory;
      },

      clearSessionHistory: (sessionId: string) => {
        set(state => {
          const { [sessionId]: _, ...remaining } = state.histories;
          return {
            histories: remaining,
          };
        });
      },

      clearAllHistory: () => {
        set({
          histories: {},
          globalHistory: [],
        });
      },

      setCurrentSession: (sessionId: string | null) => {
        set({ currentSessionId: sessionId });
      },
    }),
    {
      name: 'easyssh-command-history',
    }
  )
);

// 快捷访问函数
export const addCommandToHistory = (sessionId: string, command: string) => {
  useCommandHistoryStore.getState().addToHistory(sessionId, command);
};

export const getCommandHistory = (sessionId: string): string[] => {
  return useCommandHistoryStore
    .getState()
    .getSessionHistory(sessionId)
    .map(item => item.command);
};

export const getGlobalCommandHistory = (): string[] => {
  return useCommandHistoryStore
    .getState()
    .getGlobalHistory()
    .map(item => item.command);
};
