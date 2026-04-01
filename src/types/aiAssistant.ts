/**
 * AI Assistant Types
 * @module types/aiAssistant
 */

// =============================================================================
// AI Provider Types
// =============================================================================

/**
 * Supported AI providers
 */
export type AIProvider = 'claude' | 'openai' | 'gemini' | 'local' | 'custom';

/**
 * Model information
 */
export interface AIModel {
  /** Model ID */
  id: string;
  /** Display name */
  name: string;
  /** Provider */
  provider: AIProvider;
  /** Max tokens */
  maxTokens: number;
  /** Supports vision */
  supportsVision: boolean;
  /** Supports streaming */
  supportsStreaming: boolean;
  /** Description */
  description?: string;
}

/**
 * Provider configuration
 */
export interface AIProviderConfig {
  /** Provider ID */
  provider: AIProvider;
  /** API key */
  apiKey?: string;
  /** API endpoint (for local/custom) */
  endpoint?: string;
  /** Default model */
  defaultModel: string;
  /** Is enabled */
  enabled: boolean;
  /** Custom headers */
  headers?: Record<string, string>;
}

// =============================================================================
// Chat Types
// =============================================================================

/**
 * Message role
 */
export type MessageRole = 'user' | 'assistant' | 'system';

/**
 * Content part type for multimodal messages
 */
export type ContentPartType = 'text' | 'image' | 'file';

/**
 * Content part
 */
export interface ContentPart {
  type: ContentPartType;
  content: string;
  mimeType?: string;
  filename?: string;
}

/**
 * Chat message
 */
export interface ChatMessage {
  /** Unique message ID */
  id: string;
  /** Conversation ID */
  conversationId: string;
  /** Message role */
  role: MessageRole;
  /** Message content (text or multimodal) */
  content: string | ContentPart[];
  /** Timestamp */
  timestamp: number;
  /** Model used (for assistant messages) */
  model?: string;
  /** Provider used */
  provider?: AIProvider;
  /** Is streaming */
  isStreaming?: boolean;
  /** Error message */
  error?: string;
  /** Token usage */
  tokens?: {
    input: number;
    output: number;
  };
  /** Attachments */
  attachments?: Attachment[];
}

/**
 * File attachment
 */
export interface Attachment {
  /** Attachment ID */
  id: string;
  /** File name */
  name: string;
  /** File size in bytes */
  size: number;
  /** MIME type */
  mimeType: string;
  /** File content (base64 or URL) */
  content: string;
  /** Preview URL (for images) */
  previewUrl?: string;
}

/**
 * Conversation/Session
 */
export interface Conversation {
  /** Unique conversation ID */
  id: string;
  /** Conversation title */
  title: string;
  /** Creation timestamp */
  createdAt: number;
  /** Last update timestamp */
  updatedAt: number;
  /** Associated messages */
  messageIds: string[];
  /** Current model */
  model: string;
  /** Provider */
  provider: AIProvider;
  /** System prompt */
  systemPrompt?: string;
  /** Is pinned */
  isPinned: boolean;
  /** Tags */
  tags: string[];
  /** Token usage summary */
  tokenUsage: {
    input: number;
    output: number;
  };
}

// =============================================================================
// Quick Command Types
// =============================================================================

/**
 * Quick command/preset prompt
 */
export interface QuickCommand {
  /** Command ID */
  id: string;
  /** Display name */
  name: string;
  /** Command description */
  description?: string;
  /** Prompt template */
  prompt: string;
  /** Icon name */
  icon?: string;
  /** Category */
  category: string;
  /** Is builtin */
  isBuiltin: boolean;
  /** Variables in template */
  variables?: string[];
}

// =============================================================================
// Voice Types
// =============================================================================

/**
 * Voice synthesis provider
 */
export type VoiceProvider = 'browser' | 'elevenlabs' | 'azure' | 'local';

/**
 * Voice settings
 */
export interface VoiceSettings {
  /** Voice provider */
  provider: VoiceProvider;
  /** Voice ID/name */
  voice: string;
  /** Speed */
  speed: number;
  /** Pitch */
  pitch: number;
  /** Volume */
  volume: number;
}

/**
 * Speech recognition settings
 */
export interface SpeechRecognitionSettings {
  /** Is continuous listening */
  continuous: boolean;
  /** Language */
  language: string;
  /** Interim results */
  interimResults: boolean;
}

// =============================================================================
// Settings Types
// =============================================================================

/**
 * AI Assistant settings
 */
export interface AIAssistantSettings {
  /** Active provider */
  activeProvider: AIProvider;
  /** Active model */
  activeModel: string;
  /** Temperature */
  temperature: number;
  /** Max tokens per request */
  maxTokens: number;
  /** Providers configuration */
  providers: AIProviderConfig[];
  /** Quick commands */
  quickCommands: QuickCommand[];
  /** Voice settings */
  voice: VoiceSettings;
  /** Speech recognition */
  speech: SpeechRecognitionSettings;
  /** Auto speak responses */
  autoSpeak: boolean;
  /** Show token count */
  showTokens: boolean;
  /** Default export format */
  defaultExportFormat: 'markdown' | 'pdf' | 'txt';
  /** Keep history for days */
  historyRetentionDays: number;
}

// =============================================================================
// Streaming Types
// =============================================================================

/**
 * Stream chunk
 */
export interface StreamChunk {
  /** Chunk ID */
  id: string;
  /** Delta content */
  delta: string;
  /** Is finished */
  done: boolean;
  /** Usage stats (on final chunk) */
  usage?: {
    input: number;
    output: number;
  };
}

// =============================================================================
// Export Types
// =============================================================================

/**
 * Export format
 */
export type ExportFormat = 'markdown' | 'pdf' | 'txt' | 'json';

/**
 * Export options
 */
export interface ExportOptions {
  /** Format */
  format: ExportFormat;
  /** Include metadata */
  includeMetadata: boolean;
  /** Include timestamps */
  includeTimestamps: boolean;
  /** Include token usage */
  includeTokens: boolean;
}
