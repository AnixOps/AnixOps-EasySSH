/**
 * Chat Message List Component
 * @module components/ai-assistant/ChatMessageList
 */

import React from 'react';
import { motion } from 'framer-motion';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import remarkGfm from 'remark-gfm';
import {
  Bot,
  User,
  Copy,
  Check,
  Volume2,
  RotateCcw,
  ThumbsUp,
  ThumbsDown,
} from 'lucide-react';

import type { ChatMessage } from '../../types/aiAssistant';

// =============================================================================
// Types
// =============================================================================

interface ChatMessageListProps {
  messages: ChatMessage[];
  streamingContent: string;
  isGenerating: boolean;
  onCopyMessage: (message: ChatMessage) => void;
  copiedId: string | null;
}

// =============================================================================
// Components
// =============================================================================

export const ChatMessageList: React.FC<ChatMessageListProps> = ({
  messages,
  streamingContent,
  isGenerating,
  onCopyMessage,
  copiedId,
}) => {
  if (messages.length === 0 && !streamingContent) {
    return <EmptyState />;
  }

  return (
    <div className="flex flex-col gap-apple-4 p-apple-4">
      {messages.map((message, index) => (
        <MessageItem
          key={message.id}
          message={message}
          isLast={index === messages.length - 1}
          onCopy={() => onCopyMessage(message)}
          isCopied={copiedId === message.id}
        />
      ))}

      {/* Streaming content */}
      {isGenerating && streamingContent && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="flex gap-apple-3"
        >
          <div className="w-8 h-8 rounded-apple-lg bg-apple-accent-purple/10 flex items-center justify-center flex-shrink-0">
            <Bot className="w-5 h-5 text-apple-accent-purple" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="prose prose-sm max-w-none dark:prose-invert">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={markdownComponents}
              >
                {streamingContent}
              </ReactMarkdown>
            </div>
            <div className="mt-apple-2 flex items-center gap-apple-1">
              <span className="w-2 h-2 bg-apple-accent-purple rounded-full animate-pulse" />
              <span className="text-apple-xs text-apple-text-tertiary">思考中...</span>
            </div>
          </div>
        </motion.div>
      )}

      {/* Generating indicator */}
      {isGenerating && !streamingContent && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="flex gap-apple-3"
        >
          <div className="w-8 h-8 rounded-apple-lg bg-apple-accent-purple/10 flex items-center justify-center flex-shrink-0">
            <Bot className="w-5 h-5 text-apple-accent-purple" />
          </div>
          <div className="flex items-center gap-apple-2">
            <div className="flex gap-apple-1">
              <span className="w-2 h-2 bg-apple-text-tertiary rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
              <span className="w-2 h-2 bg-apple-text-tertiary rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
              <span className="w-2 h-2 bg-apple-text-tertiary rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
            </div>
          </div>
        </motion.div>
      )}
    </div>
  );
};

// =============================================================================
// Message Item Component
// =============================================================================

interface MessageItemProps {
  message: ChatMessage;
  isLast: boolean;
  onCopy: () => void;
  isCopied: boolean;
}

const MessageItem: React.FC<MessageItemProps> = ({ message, isLast: _isLast, onCopy, isCopied }) => {
  const isUser = message.role === 'user';
  const content = typeof message.content === 'string' ? message.content : '';

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={`flex gap-apple-3 ${isUser ? 'flex-row-reverse' : ''}`}
    >
      {/* Avatar */}
      <div
        className={`w-8 h-8 rounded-apple-lg flex items-center justify-center flex-shrink-0 ${
          isUser
            ? 'bg-apple-accent-blue/10'
            : 'bg-apple-accent-purple/10'
        }`}
      >
        {isUser ? (
          <User className="w-5 h-5 text-apple-accent-blue" />
        ) : (
          <Bot className="w-5 h-5 text-apple-accent-purple" />
        )}
      </div>

      {/* Content */}
      <div className={`flex-1 min-w-0 ${isUser ? 'max-w-[85%]' : 'max-w-none'}`}>
        {/* Message bubble */}
        <div
          className={`rounded-apple-lg px-apple-4 py-apple-3 ${
            isUser
              ? 'bg-apple-accent-blue text-white'
              : 'bg-apple-bg-secondary border border-apple-border'
          }`}
        >
          {isUser ? (
            <p className="text-apple-sm whitespace-pre-wrap">{content}</p>
          ) : (
            <div className="prose prose-sm max-w-none dark:prose-invert">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={markdownComponents}
              >
                {content}
              </ReactMarkdown>
            </div>
          )}
        </div>

        {/* Message actions */}
        {!isUser && (
          <div className="flex items-center gap-apple-1 mt-apple-1.5 opacity-0 hover:opacity-100 transition-opacity">
            <button
              onClick={onCopy}
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-tertiary transition-colors"
              title={isCopied ? '已复制' : '复制'}
            >
              {isCopied ? (
                <Check className="w-4 h-4 text-apple-accent-green" />
              ) : (
                <Copy className="w-4 h-4 text-apple-text-tertiary" />
              )}
            </button>
            <button
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-tertiary transition-colors"
              title="朗读"
            >
              <Volume2 className="w-4 h-4 text-apple-text-tertiary" />
            </button>
            <button
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-tertiary transition-colors"
              title="重新生成"
            >
              <RotateCcw className="w-4 h-4 text-apple-text-tertiary" />
            </button>
            <button
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-tertiary transition-colors"
              title="有用"
            >
              <ThumbsUp className="w-4 h-4 text-apple-text-tertiary" />
            </button>
            <button
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-tertiary transition-colors"
              title="无用"
            >
              <ThumbsDown className="w-4 h-4 text-apple-text-tertiary" />
            </button>
          </div>
        )}

        {/* Timestamp */}
        <div className={`mt-apple-1 text-apple-xs text-apple-text-tertiary ${isUser ? 'text-right' : ''}`}>
          {new Date(message.timestamp).toLocaleTimeString('zh-CN', {
            hour: '2-digit',
            minute: '2-digit',
          })}
          {message.model && (
            <span className="ml-apple-2">
              · {message.model}
            </span>
          )}
          {message.tokens && message.tokens.output > 0 && (
            <span className="ml-apple-2">
              · {message.tokens.output} tokens
            </span>
          )}
        </div>
      </div>
    </motion.div>
  );
};

// =============================================================================
// Empty State
// =============================================================================

const EmptyState: React.FC = () => {
  const suggestions = [
    { icon: '🔧', text: '解释SSH配置问题' },
    { icon: '🐛', text: '调试服务器连接失败' },
    { icon: '📝', text: '生成Ansible部署脚本' },
    { icon: '🔐', text: '检查SSH密钥安全性' },
  ];

  return (
    <div className="flex flex-col items-center justify-center h-full p-apple-8">
      <div className="w-16 h-16 rounded-apple-xl bg-gradient-to-br from-apple-accent-purple to-apple-accent-blue flex items-center justify-center mb-apple-4">
        <Bot className="w-8 h-8 text-white" />
      </div>
      <h3 className="text-apple-lg font-semibold text-apple-text-primary mb-apple-2">
        AI 智能助手
      </h3>
      <p className="text-apple-sm text-apple-text-secondary text-center max-w-md mb-apple-6">
        我可以帮你解答SSH相关问题、排查服务器故障、生成配置脚本，以及处理各种运维任务。
      </p>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-apple-3 w-full max-w-lg">
        {suggestions.map((suggestion, index) => (
          <button
            key={index}
            className="flex items-center gap-apple-3 p-apple-3 rounded-apple-lg bg-apple-bg-secondary border border-apple-border hover:border-apple-accent-purple/50 hover:bg-apple-bg-tertiary transition-all text-left"
          >
            <span className="text-apple-xl">{suggestion.icon}</span>
            <span className="text-apple-sm text-apple-text-primary">{suggestion.text}</span>
          </button>
        ))}
      </div>
    </div>
  );
};

// =============================================================================
// Markdown Components
// =============================================================================

const markdownComponents: Record<string, React.FC<any>> = {
  code({ node, inline, className, children, ...props }: any) {
    const match = /language-(\w+)/.exec(className || '');
    const language = match ? match[1] : 'text';

    if (inline) {
      return (
        <code
          className="px-apple-1.5 py-apple-0.5 bg-apple-bg-tertiary rounded-apple-sm text-apple-accent-purple text-apple-sm font-mono"
          {...props}
        >
          {children}
        </code>
      );
    }

    return (
      <div className="my-apple-3 rounded-apple-lg overflow-hidden border border-apple-border">
        {/* Code header */}
        <div className="flex items-center justify-between px-apple-3 py-apple-2 bg-apple-bg-tertiary border-b border-apple-border">
          <span className="text-apple-xs text-apple-text-secondary uppercase">{language}</span>
          <button
            onClick={() => navigator.clipboard.writeText(String(children).replace(/\n$/, ''))}
            className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-secondary transition-colors"
            title="复制代码"
          >
            <Copy className="w-4 h-4 text-apple-text-tertiary" />
          </button>
        </div>
        {/* Code content */}
        <SyntaxHighlighter
          style={vscDarkPlus}
          language={language}
          PreTag="div"
          customStyle={{
            margin: 0,
            borderRadius: '0 0 8px 8px',
            fontSize: '13px',
            lineHeight: '1.5',
          }}
          {...props}
        >
          {String(children).replace(/\n$/, '')}
        </SyntaxHighlighter>
      </div>
    );
  },
  p({ children }: any) {
    return <p className="text-apple-sm text-apple-text-primary mb-apple-3 last:mb-0">{children}</p>;
  },
  ul({ children }: any) {
    return <ul className="list-disc list-inside mb-apple-3 text-apple-sm text-apple-text-primary">{children}</ul>;
  },
  ol({ children }: any) {
    return <ol className="list-decimal list-inside mb-apple-3 text-apple-sm text-apple-text-primary">{children}</ol>;
  },
  li({ children }: any) {
    return <li className="mb-apple-1">{children}</li>;
  },
  h1({ children }: any) {
    return <h1 className="text-apple-lg font-bold text-apple-text-primary mb-apple-3">{children}</h1>;
  },
  h2({ children }: any) {
    return <h2 className="text-apple-md font-bold text-apple-text-primary mb-apple-2">{children}</h2>;
  },
  h3({ children }: any) {
    return <h3 className="text-apple-sm font-bold text-apple-text-primary mb-apple-2">{children}</h3>;
  },
  blockquote({ children }: any) {
    return (
      <blockquote className="border-l-4 border-apple-accent-purple pl-apple-3 py-apple-1 my-apple-3 bg-apple-bg-tertiary/50 rounded-apple-r">
        {children}
      </blockquote>
    );
  },
  a({ href, children }: any) {
    return (
      <a
        href={href}
        target="_blank"
        rel="noopener noreferrer"
        className="text-apple-accent-blue hover:underline"
      >
        {children}
      </a>
    );
  },
  table({ children }: any) {
    return (
      <div className="overflow-x-auto my-apple-3">
        <table className="min-w-full border-collapse border border-apple-border rounded-apple-lg">
          {children}
        </table>
      </div>
    );
  },
  thead({ children }: any) {
    return <thead className="bg-apple-bg-tertiary">{children}</thead>;
  },
  th({ children }: any) {
    return (
      <th className="px-apple-3 py-apple-2 text-left text-apple-xs font-semibold text-apple-text-secondary border-b border-apple-border">
        {children}
      </th>
    );
  },
  td({ children }: any) {
    return (
      <td className="px-apple-3 py-apple-2 text-apple-sm text-apple-text-primary border-b border-apple-border">
        {children}
      </td>
    );
  },
  hr() {
    return <hr className="my-apple-4 border-apple-border" />;
  },
};
