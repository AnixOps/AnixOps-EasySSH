/**
 * Chat Input Component
 * @module components/ai-assistant/ChatInput
 */

import React, { useRef, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Send,
  Paperclip,
  Mic,
  MicOff,
  X,
  FileText,
  Image,
  FileCode,
  Loader2,
} from 'lucide-react';

// =============================================================================
// Types
// =============================================================================

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onVoiceInput: () => void;
  isVoiceListening: boolean;
  isGenerating: boolean;
  attachments: File[];
  onAttachmentsChange: (attachments: File[]) => void;
}

// =============================================================================
// Component
// =============================================================================

export const ChatInput: React.FC<ChatInputProps> = ({
  value,
  onChange,
  onSend,
  onVoiceInput,
  isVoiceListening,
  isGenerating,
  attachments,
  onAttachmentsChange,
}) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<Record<string, number>>({});

  // Auto-resize textarea
  const handleInput = useCallback(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
    }
  }, []);

  // Handle send
  const handleSend = useCallback(() => {
    if (isGenerating) return;
    if (!value.trim() && attachments.length === 0) return;
    onSend();
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  }, [value, attachments, isGenerating, onSend]);

  // Handle keydown
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  // File upload handling
  const handleFileSelect = useCallback(
    (files: FileList | null) => {
      if (!files) return;

      const newFiles = Array.from(files);
      const totalFiles = [...attachments, ...newFiles];

      // Limit to 5 files
      if (totalFiles.length > 5) {
        alert('最多只能上传5个文件');
        return;
      }

      // Simulate upload progress
      newFiles.forEach((file) => {
        let progress = 0;
        const interval = setInterval(() => {
          progress += 10;
          setUploadProgress((prev) => ({ ...prev, [file.name]: progress }));
          if (progress >= 100) {
            clearInterval(interval);
            setTimeout(() => {
              setUploadProgress((prev) => {
                const { [file.name]: _, ...rest } = prev;
                return rest;
              });
            }, 500);
          }
        }, 50);
      });

      onAttachmentsChange(totalFiles.slice(0, 5));
    },
    [attachments, onAttachmentsChange]
  );

  // Remove attachment
  const removeAttachment = useCallback(
    (index: number) => {
      onAttachmentsChange(attachments.filter((_, i) => i !== index));
    },
    [attachments, onAttachmentsChange]
  );

  // Drag and drop
  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragging(false);
      handleFileSelect(e.dataTransfer.files);
    },
    [handleFileSelect]
  );

  // Get file icon
  const getFileIcon = (file: File) => {
    if (file.type.startsWith('image/')) return Image;
    if (file.type.includes('code') || file.name.match(/\.(js|ts|jsx|tsx|py|rs|go|java|cpp|c|h|hpp|php|rb|swift|kt)$/)) {
      return FileCode;
    }
    return FileText;
  };

  // Format file size
  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div
      className="border-t border-apple-border bg-apple-bg-secondary/50"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Drag overlay */}
      <AnimatePresence>
        {isDragging && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 bg-apple-accent-blue/10 border-2 border-dashed border-apple-accent-blue rounded-apple-lg flex items-center justify-center z-50 pointer-events-none"
          >
            <div className="text-center">
              <Paperclip className="w-10 h-10 text-apple-accent-blue mx-auto mb-apple-2" />
              <p className="text-apple-md font-medium text-apple-accent-blue">释放以上传文件</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Attachments */}
      <AnimatePresence>
        {attachments.length > 0 && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="px-apple-4 pt-apple-3"
          >
            <div className="flex flex-wrap gap-apple-2">
              {attachments.map((file, index) => {
                const Icon = getFileIcon(file);
                const progress = uploadProgress[file.name];

                return (
                  <motion.div
                    key={`${file.name}-${index}`}
                    initial={{ scale: 0.9, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    exit={{ scale: 0.9, opacity: 0 }}
                    className="relative flex items-center gap-apple-2 px-apple-3 py-apple-2 bg-apple-bg-tertiary rounded-apple-lg border border-apple-border group"
                  >
                    <Icon className="w-4 h-4 text-apple-text-secondary" />
                    <div className="min-w-0">
                      <p className="text-apple-xs text-apple-text-primary truncate max-w-[120px]">
                        {file.name}
                      </p>
                      <p className="text-apple-xs text-apple-text-tertiary">
                        {formatFileSize(file.size)}
                      </p>
                    </div>

                    {/* Upload progress */}
                    {progress !== undefined && progress < 100 && (
                      <div className="absolute inset-x-apple-0 bottom-0 h-0.5 bg-apple-border rounded-apple-b">
                        <div
                          className="h-full bg-apple-accent-blue rounded-apple-b transition-all"
                          style={{ width: `${progress}%` }}
                        />
                      </div>
                    )}

                    {/* Remove button */}
                    <button
                      onClick={() => removeAttachment(index)}
                      className="ml-apple-1 p-apple-1 rounded-apple-sm hover:bg-apple-bg-secondary opacity-0 group-hover:opacity-100 transition-opacity"
                    >
                      <X className="w-3 h-3 text-apple-text-tertiary" />
                    </button>
                  </motion.div>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Input area */}
      <div className="p-apple-3">
        <div className="flex items-end gap-apple-2 bg-apple-bg-primary rounded-apple-xl border border-apple-border focus-within:border-apple-accent-blue focus-within:ring-1 focus-within:ring-apple-accent-blue/20 transition-all">
          {/* Attach button */}
          <button
            onClick={() => fileInputRef.current?.click()}
            className="p-apple-3 rounded-apple-xl hover:bg-apple-bg-tertiary transition-colors flex-shrink-0"
            title="添加附件"
            disabled={attachments.length >= 5}
          >
            <Paperclip className="w-5 h-5 text-apple-text-secondary" />
          </button>
          <input
            ref={fileInputRef}
            type="file"
            multiple
            className="hidden"
            onChange={(e) => handleFileSelect(e.target.files)}
            accept=".txt,.md,.json,.js,.ts,.jsx,.tsx,.py,.rs,.go,.java,.cpp,.c,.h,.hpp,.php,.rb,.swift,.kt,.html,.css,.scss,.less,.xml,.yaml,.yml,.toml,.ini,.conf,.sh,.bash,.zsh,.fish,.sql,.log,.csv,.pdf,.doc,.docx,.png,.jpg,.jpeg,.gif,.webp,.svg,.bmp"
          />

          {/* Textarea */}
          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onInput={handleInput}
            onKeyDown={handleKeyDown}
            placeholder={isVoiceListening ? '正在聆听...' : '输入消息，Shift+Enter 换行...'}
            className="flex-1 min-h-[44px] max-h-[200px] py-apple-3 px-apple-2 bg-transparent border-none outline-none resize-none text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary"
            rows={1}
            disabled={isGenerating}
          />

          {/* Voice button */}
          <button
            onClick={onVoiceInput}
            className={`p-apple-3 rounded-apple-xl transition-colors flex-shrink-0 ${
              isVoiceListening
                ? 'bg-apple-accent-red/10 text-apple-accent-red animate-pulse'
                : 'hover:bg-apple-bg-tertiary text-apple-text-secondary'
            }`}
            title={isVoiceListening ? '停止录音' : '语音输入'}
          >
            {isVoiceListening ? (
              <MicOff className="w-5 h-5" />
            ) : (
              <Mic className="w-5 h-5" />
            )}
          </button>

          {/* Send button */}
          <button
            onClick={handleSend}
            disabled={isGenerating || (!value.trim() && attachments.length === 0)}
            className={`p-apple-3 rounded-apple-xl transition-all flex-shrink-0 ${
              isGenerating || (!value.trim() && attachments.length === 0)
                ? 'opacity-50 cursor-not-allowed'
                : 'bg-apple-accent-blue text-white hover:bg-apple-accent-blue/90'
            }`}
            title={isGenerating ? '生成中...' : '发送'}
          >
            {isGenerating ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              <Send className="w-5 h-5" />
            )}
          </button>
        </div>

        {/* Input hints */}
        <div className="flex items-center justify-between mt-apple-2 px-apple-1">
          <p className="text-apple-xs text-apple-text-tertiary">
            {isVoiceListening ? (
              <span className="flex items-center gap-apple-1">
                <span className="w-1.5 h-1.5 bg-apple-accent-red rounded-full animate-pulse" />
                正在聆听，请说话...
              </span>
            ) : (
              <span>Shift + Enter 换行 · 支持拖拽上传</span>
            )}
          </p>
          <p className="text-apple-xs text-apple-text-tertiary">
            {value.length} / 4000
          </p>
        </div>
      </div>
    </div>
  );
};
