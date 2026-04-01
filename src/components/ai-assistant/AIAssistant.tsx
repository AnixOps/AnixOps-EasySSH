/**
 * AI Assistant Main Component
 * @module components/ai-assistant/AIAssistant
 */

import React, { useEffect, useRef, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Plus,
  Settings,
  PanelLeft,
  X,
  Download,
  Sparkles,
  Command,
} from 'lucide-react';

// Components
import { ChatMessageList } from './ChatMessageList';
import { ChatInput } from './ChatInput';
import { ConversationSidebar } from './ConversationSidebar';
import { AISettingsPanel } from './AISettingsPanel';
import { QuickCommandsPanel } from './QuickCommandsPanel';

// Store
import {
  useAIAssistantStore,
  useActiveConversation,
  useActiveConversationMessages,
  useAISettings,
  useAIUIState,
} from '../../stores/aiAssistantStore';

// Types
import type { ChatMessage, QuickCommand, AIProvider } from '../../types/aiAssistant';

// =============================================================================
// AI Assistant Main Component
// =============================================================================

export interface AIAssistantProps {
  /** Is in fullscreen mode */
  fullscreen?: boolean;
  /** Initial conversation ID */
  initialConversationId?: string;
  /** On close callback */
  onClose?: () => void;
  /** Embed mode (no outer shell) */
  embed?: boolean;
}

export const AIAssistant: React.FC<AIAssistantProps> = ({
  fullscreen = false,
  initialConversationId,
  onClose,
  embed = false,
}) => {
  const {
    createConversation,
    setActiveConversation,
    addMessage,
    updateMessage,
    toggleSidebar,
    openSettings,
    setGenerating,
    appendStreamingContent,
    clearStreamingContent,
    speakText,
  } = useAIAssistantStore();

  const settings = useAISettings();
  const activeConversation = useActiveConversation();
  const messages = useActiveConversationMessages();
  const uiState = useAIUIState();

  const [showQuickCommands, setShowQuickCommands] = React.useState(false);
  const [inputValue, setInputValue] = React.useState('');
  const [attachments, setAttachments] = React.useState<File[]>([]);
  const [copiedId, setCopiedId] = React.useState<string | null>(null);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Initialize with conversation
  useEffect(() => {
    if (initialConversationId) {
      setActiveConversation(initialConversationId);
    } else if (!activeConversation) {
      const id = createConversation();
      setActiveConversation(id);
    }
  }, [initialConversationId]);

  // Scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, uiState.streamingContent]);

  // Handle send message
  const handleSend = useCallback(async () => {
    if (!inputValue.trim() && attachments.length === 0) return;
    if (!activeConversation) return;

    const content = inputValue.trim();
    setInputValue('');

    // Add user message
    addMessage(activeConversation.id, {
      role: 'user',
      content,
      conversationId: activeConversation.id,
    });

    // Simulate AI response
    setGenerating(true);
    clearStreamingContent();

    // Create assistant message placeholder
    const assistantMessageId = addMessage(activeConversation.id, {
      role: 'assistant',
      content: '',
      conversationId: activeConversation.id,
      isStreaming: true,
      provider: settings.activeProvider,
      model: settings.activeModel,
    });

    // Simulate streaming response
    const responses = [
      '我来帮你分析这个问题。',
      '\n\n',
      '首先，我们需要理解上下文...',
      '\n\n',
      '```python\n# 示例代码\ndef analyze():\n    return "analysis"\n```',
      '\n\n',
      '基于以上分析，我建议：',
      '\n',
      '1. 检查配置',
      '2. 验证连接',
      '3. 重试操作',
    ];

    let fullResponse = '';
    for (const chunk of responses) {
      await new Promise((r) => setTimeout(r, 100 + Math.random() * 200));
      fullResponse += chunk;
      appendStreamingContent(chunk);
    }

    // Finalize message
    updateMessage(assistantMessageId, {
      content: fullResponse,
      isStreaming: false,
      tokens: { input: content.length, output: fullResponse.length },
    });

    setGenerating(false);
    clearStreamingContent();

    // Auto speak if enabled
    if (settings.autoSpeak) {
      speakText(fullResponse);
    }
  }, [inputValue, attachments, activeConversation, settings, addMessage, updateMessage]);

  // Handle quick command
  const handleQuickCommand = useCallback(
    (command: QuickCommand) => {
      let prompt = command.prompt;
      // Replace variables with placeholders or show dialog
      prompt = prompt.replace(/\{\{(\w+)\}\}/g, '[$1]');
      setInputValue(prompt);
      setShowQuickCommands(false);
    },
    [setInputValue]
  );

  // Handle copy message
  const handleCopyMessage = useCallback((message: ChatMessage) => {
    const content = typeof message.content === 'string' ? message.content : '';
    navigator.clipboard.writeText(content);
    setCopiedId(message.id);
    setTimeout(() => setCopiedId(null), 2000);
  }, []);

  // Handle export
  const handleExport = useCallback(
    (format: 'markdown' | 'pdf' | 'txt') => {
      if (!activeConversation) return;
      const { exportConversation } = useAIAssistantStore.getState();
      const content = exportConversation(activeConversation.id, format);

      // Create download
      const blob = new Blob([content], { type: 'text/plain' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${activeConversation.title}.${format === 'markdown' ? 'md' : format}`;
      a.click();
      URL.revokeObjectURL(url);
    },
    [activeConversation]
  );

  // Handle voice input
  const handleVoiceInput = useCallback(() => {
    if (uiState.isVoiceListening) {
      useAIAssistantStore.getState().stopVoiceListening();
    } else {
      useAIAssistantStore.getState().startVoiceListening();
      // Simulated voice recognition
      setTimeout(() => {
        setInputValue('这是语音输入的示例文本...');
        useAIAssistantStore.getState().stopVoiceListening();
      }, 2000);
    }
  }, [uiState.isVoiceListening, setInputValue]);

  // Provider badge color
  const getProviderColor = (provider: AIProvider): string => {
    const colors: Record<AIProvider, string> = {
      claude: 'bg-orange-500',
      openai: 'bg-green-500',
      gemini: 'bg-blue-500',
      local: 'bg-purple-500',
      custom: 'bg-gray-500',
    };
    return colors[provider] || 'bg-gray-500';
  };

  // Provider display name
  const getProviderName = (provider: AIProvider): string => {
    const names: Record<AIProvider, string> = {
      claude: 'Claude',
      openai: 'GPT',
      gemini: 'Gemini',
      local: 'Local',
      custom: 'Custom',
    };
    return names[provider] || provider;
  };

  return (
    <div
      className={`flex flex-col bg-apple-bg-primary ${
        fullscreen ? 'fixed inset-0 z-50' : 'h-full'
      } ${embed ? '' : 'rounded-apple-xl border border-apple-border shadow-apple-lg'}`}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-apple-4 py-apple-3 border-b border-apple-border bg-apple-bg-secondary/50">
        <div className="flex items-center gap-apple-3">
          {!embed && (
            <button
              onClick={toggleSidebar}
              className="p-apple-2 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors"
              title="切换侧边栏"
            >
              <PanelLeft className="w-5 h-5 text-apple-text-secondary" />
            </button>
          )}

          <div className="flex items-center gap-apple-2">
            <div className="w-8 h-8 rounded-apple-lg bg-gradient-to-br from-apple-accent-purple to-apple-accent-blue flex items-center justify-center">
              <Sparkles className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 className="text-apple-sm font-semibold text-apple-text-primary">
                AI 助手
              </h2>
              {activeConversation && (
                <div className="flex items-center gap-apple-1.5">
                  <span className="text-apple-xs text-apple-text-secondary">
                    {activeConversation.title}
                  </span>
                  <span
                    className={`w-2 h-2 rounded-full ${getProviderColor(
                      settings.activeProvider
                    )}`}
                    title={getProviderName(settings.activeProvider)}
                  />
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-apple-2">
          {/* New Conversation */}
          <button
            onClick={() => {
              const id = createConversation();
              setActiveConversation(id);
            }}
            className="p-apple-2 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors"
            title="新建对话"
          >
            <Plus className="w-5 h-5 text-apple-text-secondary" />
          </button>

          {/* Quick Commands */}
          <button
            onClick={() => setShowQuickCommands(true)}
            className="p-apple-2 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors"
            title="快捷指令"
          >
            <Command className="w-5 h-5 text-apple-text-secondary" />
          </button>

          {/* Settings */}
          <button
            onClick={openSettings}
            className="p-apple-2 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors"
            title="设置"
          >
            <Settings className="w-5 h-5 text-apple-text-secondary" />
          </button>

          {/* Export */}
          {activeConversation && (
            <div className="relative group">
              <button className="p-apple-2 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors">
                <Download className="w-5 h-5 text-apple-text-secondary" />
              </button>
              <div className="absolute right-0 top-full mt-apple-1 py-apple-1 bg-apple-bg-secondary rounded-apple-md shadow-apple-lg border border-apple-border opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50 min-w-[120px]">
                <button
                  onClick={() => handleExport('markdown')}
                  className="w-full px-apple-3 py-apple-1.5 text-apple-sm text-apple-text-primary hover:bg-apple-bg-tertiary text-left"
                >
                  导出为 Markdown
                </button>
                <button
                  onClick={() => handleExport('txt')}
                  className="w-full px-apple-3 py-apple-1.5 text-apple-sm text-apple-text-primary hover:bg-apple-bg-tertiary text-left"
                >
                  导出为 TXT
                </button>
              </div>
            </div>
          )}

          {/* Close */}
          {onClose && (
            <button
              onClick={onClose}
              className="p-apple-2 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors"
            >
              <X className="w-5 h-5 text-apple-text-secondary" />
            </button>
          )}
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        <AnimatePresence initial={false}>
          {uiState.isSidebarOpen && !embed && (
            <motion.div
              initial={{ width: 0, opacity: 0 }}
              animate={{ width: 280, opacity: 1 }}
              exit={{ width: 0, opacity: 0 }}
              transition={{ duration: 0.2 }}
              className="border-r border-apple-border overflow-hidden"
            >
              <ConversationSidebar />
            </motion.div>
          )}
        </AnimatePresence>

        {/* Chat Area */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Messages */}
          <div className="flex-1 overflow-y-auto">
            <ChatMessageList
              messages={messages}
              streamingContent={uiState.streamingContent}
              isGenerating={uiState.isGenerating}
              onCopyMessage={handleCopyMessage}
              copiedId={copiedId}
            />
            <div ref={messagesEndRef} />
          </div>

          {/* Input Area */}
          <ChatInput
            value={inputValue}
            onChange={setInputValue}
            onSend={handleSend}
            onVoiceInput={handleVoiceInput}
            isVoiceListening={uiState.isVoiceListening}
            isGenerating={uiState.isGenerating}
            attachments={attachments}
            onAttachmentsChange={setAttachments}
          />
        </div>
      </div>

      {/* Settings Panel */}
      <AISettingsPanel />

      {/* Quick Commands Panel */}
      <QuickCommandsPanel
        isOpen={showQuickCommands}
        onClose={() => setShowQuickCommands(false)}
        onSelect={handleQuickCommand}
      />
    </div>
  );
};

export default AIAssistant;
