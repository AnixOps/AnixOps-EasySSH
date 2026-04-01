/**
 * AI Assistant Store
 * @module stores/aiAssistantStore
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';
import type {
  AIProvider,
  AIModel,
  AIProviderConfig,
  ChatMessage,
  Conversation,
  QuickCommand,
  AIAssistantSettings,
} from '../types/aiAssistant';

// =============================================================================
// Default Models
// =============================================================================

const DEFAULT_MODELS: AIModel[] = [
  // Claude Models
  { id: 'claude-3-opus-20240229', name: 'Claude 3 Opus', provider: 'claude', maxTokens: 4096, supportsVision: true, supportsStreaming: true, description: 'Most powerful Claude model' },
  { id: 'claude-3-sonnet-20240229', name: 'Claude 3.5 Sonnet', provider: 'claude', maxTokens: 4096, supportsVision: true, supportsStreaming: true, description: 'Best balance of intelligence and speed' },
  { id: 'claude-3-haiku-20240307', name: 'Claude 3 Haiku', provider: 'claude', maxTokens: 4096, supportsVision: true, supportsStreaming: true, description: 'Fastest Claude model' },

  // OpenAI Models
  { id: 'gpt-4-turbo-preview', name: 'GPT-4 Turbo', provider: 'openai', maxTokens: 4096, supportsVision: true, supportsStreaming: true, description: 'Latest GPT-4 model' },
  { id: 'gpt-4o', name: 'GPT-4o', provider: 'openai', maxTokens: 4096, supportsVision: true, supportsStreaming: true, description: 'Omni-modal GPT-4' },
  { id: 'gpt-3.5-turbo', name: 'GPT-3.5 Turbo', provider: 'openai', maxTokens: 4096, supportsVision: false, supportsStreaming: true, description: 'Fast and cost-effective' },

  // Gemini Models
  { id: 'gemini-1.5-pro-latest', name: 'Gemini 1.5 Pro', provider: 'gemini', maxTokens: 8192, supportsVision: true, supportsStreaming: true, description: 'Advanced reasoning' },
  { id: 'gemini-1.5-flash-latest', name: 'Gemini 1.5 Flash', provider: 'gemini', maxTokens: 8192, supportsVision: true, supportsStreaming: true, description: 'Fast and efficient' },

  // Local Models
  { id: 'llama2', name: 'Llama 2', provider: 'local', maxTokens: 4096, supportsVision: false, supportsStreaming: true, description: 'Local Llama 2' },
  { id: 'mistral', name: 'Mistral', provider: 'local', maxTokens: 4096, supportsVision: false, supportsStreaming: true, description: 'Local Mistral' },
  { id: 'codellama', name: 'CodeLlama', provider: 'local', maxTokens: 4096, supportsVision: false, supportsStreaming: true, description: 'Code-specialized' },
];

const DEFAULT_QUICK_COMMANDS: QuickCommand[] = [
  { id: 'explain-code', name: '解释代码', description: '解释选中的代码', prompt: '请解释以下代码的工作原理：\n\n```\n{{code}}\n```', icon: 'Code', category: '代码', isBuiltin: true, variables: ['code'] },
  { id: 'refactor-code', name: '重构代码', description: '优化代码结构', prompt: '请重构以下代码，使其更清晰、更高效：\n\n```\n{{code}}\n```', icon: 'Wand2', category: '代码', isBuiltin: true, variables: ['code'] },
  { id: 'debug-code', name: '调试代码', description: '找出代码问题', prompt: '请帮我找出以下代码的问题：\n\n```\n{{code}}\n```\n\n错误信息：{{error}}', icon: 'Bug', category: '代码', isBuiltin: true, variables: ['code', 'error'] },
  { id: 'write-tests', name: '写单元测试', description: '生成测试代码', prompt: '请为以下代码编写单元测试：\n\n```\n{{code}}\n```\n\n使用 {{framework}} 测试框架。', icon: 'TestTube', category: '代码', isBuiltin: true, variables: ['code', 'framework'] },
  { id: 'generate-docs', name: '生成文档', description: '生成代码文档', prompt: '请为以下代码生成详细的文档注释：\n\n```\n{{code}}\n```', icon: 'FileText', category: '代码', isBuiltin: true, variables: ['code'] },
  { id: 'shell-command', name: 'Shell命令', description: '解释Shell命令', prompt: '请解释以下Shell命令的作用：\n\n```bash\n{{command}}\n```', icon: 'Terminal', category: 'SSH', isBuiltin: true, variables: ['command'] },
  { id: 'troubleshoot', name: '故障排查', description: '诊断服务器问题', prompt: '服务器出现了以下问题，请帮我排查：\n\n症状：{{symptom}}\n\n日志：\n```\n{{logs}}\n```', icon: 'AlertCircle', category: 'SSH', isBuiltin: true, variables: ['symptom', 'logs'] },
  { id: 'config-review', name: '配置审查', description: '审查SSH配置', prompt: '请审查以下SSH配置的安全性：\n\n```\n{{config}}\n```', icon: 'Shield', category: 'SSH', isBuiltin: true, variables: ['config'] },
  { id: 'summarize', name: '总结文本', description: '总结长文本', prompt: '请总结以下内容的要点：\n\n{{text}}', icon: 'Text', category: '通用', isBuiltin: true, variables: ['text'] },
  { id: 'translate', name: '翻译', description: '翻译文本', prompt: '请将以下内容翻译成 {{targetLanguage}}：\n\n{{text}}', icon: 'Languages', category: '通用', isBuiltin: true, variables: ['text', 'targetLanguage'] },
];

const DEFAULT_SETTINGS: AIAssistantSettings = {
  activeProvider: 'claude',
  activeModel: 'claude-3-sonnet-20240229',
  temperature: 0.7,
  maxTokens: 4096,
  providers: [
    { provider: 'claude', apiKey: '', defaultModel: 'claude-3-sonnet-20240229', enabled: false },
    { provider: 'openai', apiKey: '', defaultModel: 'gpt-4o', enabled: false },
    { provider: 'gemini', apiKey: '', defaultModel: 'gemini-1.5-pro-latest', enabled: false },
    { provider: 'local', endpoint: 'http://localhost:11434', defaultModel: 'llama2', enabled: true },
  ],
  quickCommands: DEFAULT_QUICK_COMMANDS,
  voice: {
    provider: 'browser',
    voice: 'default',
    speed: 1,
    pitch: 1,
    volume: 1,
  },
  speech: {
    continuous: false,
    language: 'zh-CN',
    interimResults: true,
  },
  autoSpeak: false,
  showTokens: true,
  defaultExportFormat: 'markdown',
  historyRetentionDays: 30,
};

// =============================================================================
// State & Actions
// =============================================================================

interface AIAssistantState {
  // Settings
  settings: AIAssistantSettings;

  // Conversations
  conversations: Conversation[];
  activeConversationId: string | null;

  // Messages
  messages: Record<string, ChatMessage>;

  // UI State
  isSidebarOpen: boolean;
  isSettingsOpen: boolean;
  isVoiceListening: boolean;
  isSpeaking: boolean;
  isGenerating: boolean;
  streamingContent: string;

  // Available models
  availableModels: AIModel[];
}

interface AIAssistantActions {
  // Settings
  updateSettings: (settings: Partial<AIAssistantSettings>) => void;
  updateProviderConfig: (provider: AIProvider, config: Partial<AIProviderConfig>) => void;
  setActiveProvider: (provider: AIProvider) => void;
  setActiveModel: (model: string) => void;

  // Conversations
  createConversation: (title?: string) => string;
  deleteConversation: (id: string) => void;
  setActiveConversation: (id: string | null) => void;
  updateConversation: (id: string, updates: Partial<Conversation>) => void;
  pinConversation: (id: string) => void;
  unpinConversation: (id: string) => void;
  renameConversation: (id: string, title: string) => void;
  clearAllConversations: () => void;

  // Messages
  addMessage: (conversationId: string, message: Omit<ChatMessage, 'id' | 'timestamp'>) => string;
  updateMessage: (id: string, updates: Partial<ChatMessage>) => void;
  deleteMessage: (id: string) => void;
  clearConversationMessages: (conversationId: string) => void;
  getConversationMessages: (conversationId: string) => ChatMessage[];
  appendStreamingContent: (content: string) => void;
  clearStreamingContent: () => void;

  // Quick Commands
  addQuickCommand: (command: Omit<QuickCommand, 'id' | 'isBuiltin'>) => void;
  updateQuickCommand: (id: string, updates: Partial<QuickCommand>) => void;
  deleteQuickCommand: (id: string) => void;

  // UI
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  openSettings: () => void;
  closeSettings: () => void;
  setVoiceListening: (listening: boolean) => void;
  setSpeaking: (speaking: boolean) => void;
  setGenerating: (generating: boolean) => void;

  // Export
  exportConversation: (conversationId: string, format: 'markdown' | 'pdf' | 'txt') => string;

  // Import
  importConversation: (data: string, format: 'json') => string | null;

  // Speech
  startVoiceListening: () => void;
  stopVoiceListening: () => void;

  // Voice synthesis
  speakText: (text: string) => void;
  stopSpeaking: () => void;
}

// =============================================================================
// Store
// =============================================================================

const generateId = () => `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

export const useAIAssistantStore = create<AIAssistantState & AIAssistantActions>()(
  immer(
    persist(
      (set, get) => ({
        // Initial State
        settings: DEFAULT_SETTINGS,
        conversations: [],
        activeConversationId: null,
        messages: {},
        isSidebarOpen: true,
        isSettingsOpen: false,
        isVoiceListening: false,
        isSpeaking: false,
        isGenerating: false,
        streamingContent: '',
        availableModels: DEFAULT_MODELS,

        // Settings Actions
        updateSettings: (newSettings) =>
          set((state) => {
            Object.assign(state.settings, newSettings);
          }),

        updateProviderConfig: (provider, config) =>
          set((state) => {
            const idx = state.settings.providers.findIndex((p: AIProviderConfig) => p.provider === provider);
            if (idx >= 0) {
              Object.assign(state.settings.providers[idx], config);
            }
          }),

        setActiveProvider: (provider) =>
          set((state) => {
            state.settings.activeProvider = provider;
            const providerConfig = state.settings.providers.find((p: AIProviderConfig) => p.provider === provider);
            if (providerConfig) {
              state.settings.activeModel = providerConfig.defaultModel;
            }
          }),

        setActiveModel: (model) =>
          set((state) => {
            state.settings.activeModel = model;
          }),

        // Conversation Actions
        createConversation: (title) => {
          const id = generateId();
          const newConversation: Conversation = {
            id,
            title: title || '新对话',
            createdAt: Date.now(),
            updatedAt: Date.now(),
            messageIds: [],
            model: get().settings.activeModel,
            provider: get().settings.activeProvider,
            isPinned: false,
            tags: [],
            tokenUsage: { input: 0, output: 0 },
          };
          set((state) => {
            state.conversations.unshift(newConversation);
            state.activeConversationId = id;
          });
          return id;
        },

        deleteConversation: (id) =>
          set((state) => {
            const idx = state.conversations.findIndex((c: Conversation) => c.id === id);
            if (idx >= 0) {
              // Delete associated messages
              state.conversations[idx].messageIds.forEach((msgId: string) => {
                delete state.messages[msgId];
              });
              state.conversations.splice(idx, 1);
              if (state.activeConversationId === id) {
                state.activeConversationId = state.conversations[0]?.id || null;
              }
            }
          }),

        setActiveConversation: (id) =>
          set((state) => {
            state.activeConversationId = id;
          }),

        updateConversation: (id, updates) =>
          set((state) => {
            const conv = state.conversations.find((c: Conversation) => c.id === id);
            if (conv) {
              Object.assign(conv, updates, { updatedAt: Date.now() });
            }
          }),

        pinConversation: (id) =>
          set((state) => {
            const conv = state.conversations.find((c: Conversation) => c.id === id);
            if (conv) {
              conv.isPinned = true;
            }
          }),

        unpinConversation: (id) =>
          set((state) => {
            const conv = state.conversations.find((c: Conversation) => c.id === id);
            if (conv) {
              conv.isPinned = false;
            }
          }),

        renameConversation: (id, title) =>
          set((state) => {
            const conv = state.conversations.find((c: Conversation) => c.id === id);
            if (conv) {
              conv.title = title;
              conv.updatedAt = Date.now();
            }
          }),

        clearAllConversations: () =>
          set((state) => {
            state.conversations = [];
            state.messages = {};
            state.activeConversationId = null;
          }),

        // Message Actions
        addMessage: (conversationId, message) => {
          const id = generateId();
          const newMessage: ChatMessage = {
            ...message,
            id,
            timestamp: Date.now(),
            conversationId,
          };
          set((state) => {
            state.messages[id] = newMessage;
            const conv = state.conversations.find((c: Conversation) => c.id === conversationId);
            if (conv) {
              conv.messageIds.push(id);
              conv.updatedAt = Date.now();
            }
          });
          return id;
        },

        updateMessage: (id, updates) =>
          set((state) => {
            const msg = state.messages[id];
            if (msg) {
              Object.assign(msg, updates);
            }
          }),

        deleteMessage: (id) =>
          set((state) => {
            const msg = state.messages[id];
            if (msg) {
              const conv = state.conversations.find((c: Conversation) => c.id === msg.conversationId);
              if (conv) {
                conv.messageIds = conv.messageIds.filter((mid: string) => mid !== id);
              }
              delete state.messages[id];
            }
          }),

        clearConversationMessages: (conversationId) =>
          set((state) => {
            const conv = state.conversations.find((c: Conversation) => c.id === conversationId);
            if (conv) {
              conv.messageIds.forEach((msgId: string) => {
                delete state.messages[msgId];
              });
              conv.messageIds = [];
              conv.tokenUsage = { input: 0, output: 0 };
            }
          }),

        getConversationMessages: (conversationId) => {
          const { conversations, messages } = get();
          const conv = conversations.find((c: Conversation) => c.id === conversationId);
          if (!conv) return [];
          return conv.messageIds.map((id: string) => messages[id]).filter(Boolean);
        },

        appendStreamingContent: (content) =>
          set((state) => {
            state.streamingContent += content;
          }),

        clearStreamingContent: () =>
          set((state) => {
            state.streamingContent = '';
          }),

        // Quick Commands
        addQuickCommand: (command) =>
          set((state) => {
            state.settings.quickCommands.push({
              ...command,
              id: generateId(),
              isBuiltin: false,
            });
          }),

        updateQuickCommand: (id, updates) =>
          set((state) => {
            const cmd = state.settings.quickCommands.find((c: QuickCommand) => c.id === id);
            if (cmd && !cmd.isBuiltin) {
              Object.assign(cmd, updates);
            }
          }),

        deleteQuickCommand: (id) =>
          set((state) => {
            state.settings.quickCommands = state.settings.quickCommands.filter(
              (c: QuickCommand) => c.id !== id || c.isBuiltin
            );
          }),

        // UI Actions
        toggleSidebar: () =>
          set((state) => {
            state.isSidebarOpen = !state.isSidebarOpen;
          }),

        setSidebarOpen: (open) =>
          set((state) => {
            state.isSidebarOpen = open;
          }),

        openSettings: () =>
          set((state) => {
            state.isSettingsOpen = true;
          }),

        closeSettings: () =>
          set((state) => {
            state.isSettingsOpen = false;
          }),

        setVoiceListening: (listening) =>
          set((state) => {
            state.isVoiceListening = listening;
          }),

        setSpeaking: (speaking) =>
          set((state) => {
            state.isSpeaking = speaking;
          }),

        setGenerating: (generating) =>
          set((state) => {
            state.isGenerating = generating;
          }),

        // Export
        exportConversation: (conversationId, format) => {
          const { conversations, messages } = get();
          const conv = conversations.find((c: Conversation) => c.id === conversationId);
          if (!conv) return '';

          const msgs = conv.messageIds.map((id: string) => messages[id]).filter(Boolean) as ChatMessage[];

          if (format === 'markdown') {
            let md = `# ${conv.title}\n\n`;
            md += `> Created: ${new Date(conv.createdAt).toLocaleString()}\n\n`;
            md += `---\n\n`;
            msgs.forEach((msg: ChatMessage) => {
              const role = msg.role === 'user' ? '**User**' : '**Assistant**';
              md += `### ${role} (${new Date(msg.timestamp).toLocaleTimeString()})\n\n`;
              if (typeof msg.content === 'string') {
                md += `${msg.content}\n\n`;
              }
              md += `---\n\n`;
            });
            return md;
          } else if (format === 'txt') {
            let txt = `${conv.title}\n\n`;
            msgs.forEach((msg: ChatMessage) => {
              const role = msg.role === 'user' ? 'User' : 'Assistant';
              txt += `[${role}] ${new Date(msg.timestamp).toLocaleTimeString()}\n`;
              if (typeof msg.content === 'string') {
                txt += `${msg.content}\n\n`;
              }
            });
            return txt;
          }
          return '';
        },

        // Import
        importConversation: (data) => {
          try {
            const parsed = JSON.parse(data);
            if (parsed.messages && Array.isArray(parsed.messages)) {
              const convId = generateId();
              const newConv: Conversation = {
                id: convId,
                title: parsed.title || '导入的对话',
                createdAt: Date.now(),
                updatedAt: Date.now(),
                messageIds: [],
                model: get().settings.activeModel,
                provider: get().settings.activeProvider,
                isPinned: false,
                tags: ['imported'],
                tokenUsage: { input: 0, output: 0 },
              };

              set((state) => {
                state.conversations.unshift(newConv);
                parsed.messages.forEach((msg: Partial<ChatMessage>) => {
                  const id = generateId();
                  state.messages[id] = {
                    ...msg,
                    id,
                    conversationId: convId,
                    timestamp: msg.timestamp || Date.now(),
                  } as ChatMessage;
                  newConv.messageIds.push(id);
                });
              });
              return convId;
            }
          } catch (e) {
            console.error('Failed to import conversation:', e);
          }
          return null;
        },

        // Speech (placeholders for actual implementation)
        startVoiceListening: () => {
          set((state) => {
            state.isVoiceListening = true;
          });
          // Actual implementation would use Web Speech API or Tauri plugin
        },

        stopVoiceListening: () => {
          set((state) => {
            state.isVoiceListening = false;
          });
        },

        speakText: (text) => {
          set((state) => {
            state.isSpeaking = true;
          });
          // Actual implementation would use Web Speech API or Tauri plugin
          const utterance = new SpeechSynthesisUtterance(text);
          utterance.onend = () => {
            get().stopSpeaking();
          };
          window.speechSynthesis.speak(utterance);
        },

        stopSpeaking: () => {
          window.speechSynthesis.cancel();
          set((state) => {
            state.isSpeaking = false;
          });
        },
      }),
      {
        name: 'ai-assistant-storage',
        partialize: (state) => ({
          settings: state.settings,
          conversations: state.conversations,
          messages: state.messages,
          activeConversationId: state.activeConversationId,
        }),
      }
    )
  )
);

// =============================================================================
// Selectors
// =============================================================================

export const useAISettings = () => useAIAssistantStore((state) => state.settings);
export const useAIConversations = () => useAIAssistantStore((state) => state.conversations);
export const useActiveConversationId = () => useAIAssistantStore((state) => state.activeConversationId);
export const useActiveConversation = () =>
  useAIAssistantStore((state) => {
    return state.conversations.find((c) => c.id === state.activeConversationId) || null;
  });
export const useConversationMessages = (conversationId: string) =>
  useAIAssistantStore((state) => {
    const conv = state.conversations.find((c) => c.id === conversationId);
    if (!conv) return [];
    return conv.messageIds.map((id) => state.messages[id]).filter(Boolean);
  });
export const useActiveConversationMessages = () =>
  useAIAssistantStore((state) => {
    const conv = state.conversations.find((c) => c.id === state.activeConversationId);
    if (!conv) return [];
    return conv.messageIds.map((id) => state.messages[id]).filter(Boolean);
  });
export const useAIQuickCommands = () => useAIAssistantStore((state) => state.settings.quickCommands);
export const useAvailableModels = () => useAIAssistantStore((state) => state.availableModels);
export const useAIUIState = () =>
  useAIAssistantStore((state) => ({
    isSidebarOpen: state.isSidebarOpen,
    isSettingsOpen: state.isSettingsOpen,
    isVoiceListening: state.isVoiceListening,
    isSpeaking: state.isSpeaking,
    isGenerating: state.isGenerating,
    streamingContent: state.streamingContent,
  }));
