/**
 * Conversation Sidebar Component
 * @module components/ai-assistant/ConversationSidebar
 */

import React, { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Plus,
  MessageSquare,
  Search,
  MoreVertical,
  Pin,
  Trash2,
  Edit2,
  Check,
  X,
  Clock,
} from 'lucide-react';

// Store
import {
  useAIAssistantStore,
  useAIConversations,
  useActiveConversationId,
} from '../../stores/aiAssistantStore';

// Types
import type { Conversation } from '../../types/aiAssistant';

// =============================================================================
// Component
// =============================================================================

export const ConversationSidebar: React.FC = () => {
  const conversations = useAIConversations();
  const activeId = useActiveConversationId();
  const [searchQuery, setSearchQuery] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTitle, setEditTitle] = useState('');

  const {
    createConversation,
    setActiveConversation,
    deleteConversation,
    pinConversation,
    unpinConversation,
    renameConversation,
  } = useAIAssistantStore();

  // Filter and sort conversations
  const filteredConversations = useMemo(() => {
    let filtered = conversations;

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = conversations.filter(
        (c) =>
          c.title.toLowerCase().includes(query) ||
          c.tags.some((t) => t.toLowerCase().includes(query))
      );
    }

    // Sort: pinned first, then by updatedAt
    return [...filtered].sort((a, b) => {
      if (a.isPinned && !b.isPinned) return -1;
      if (!a.isPinned && b.isPinned) return 1;
      return b.updatedAt - a.updatedAt;
    });
  }, [conversations, searchQuery]);

  // Group conversations by date
  const groupedConversations = useMemo(() => {
    const groups: Record<string, Conversation[]> = {
      '置顶': [],
      '今天': [],
      '昨天': [],
      '过去7天': [],
      '更早': [],
    };

    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const yesterday = new Date(today.getTime() - 24 * 60 * 60 * 1000);
    const weekAgo = new Date(today.getTime() - 7 * 24 * 60 * 60 * 1000);

    filteredConversations.forEach((conv) => {
      if (conv.isPinned) {
        groups['置顶'].push(conv);
      } else {
        const convDate = new Date(conv.updatedAt);
        if (convDate >= today) {
          groups['今天'].push(conv);
        } else if (convDate >= yesterday) {
          groups['昨天'].push(conv);
        } else if (convDate >= weekAgo) {
          groups['过去7天'].push(conv);
        } else {
          groups['更早'].push(conv);
        }
      }
    });

    return Object.entries(groups).filter(([_, items]) => items.length > 0);
  }, [filteredConversations]);

  // Handle rename
  const handleStartEdit = (conv: Conversation) => {
    setEditingId(conv.id);
    setEditTitle(conv.title);
  };

  const handleSaveEdit = () => {
    if (editingId && editTitle.trim()) {
      renameConversation(editingId, editTitle.trim());
    }
    setEditingId(null);
    setEditTitle('');
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setEditTitle('');
  };

  // Format time
  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const isToday = date.toDateString() === now.toDateString();

    if (isToday) {
      return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
    }
    return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' });
  };

  return (
    <div className="flex flex-col h-full bg-apple-bg-secondary">
      {/* Header */}
      <div className="p-apple-4 border-b border-apple-border">
        <div className="flex items-center justify-between mb-apple-3">
          <h2 className="text-apple-sm font-semibold text-apple-text-primary">对话历史</h2>
          <button
            onClick={() => {
              const id = createConversation();
              setActiveConversation(id);
            }}
            className="p-apple-1.5 rounded-apple-md hover:bg-apple-bg-tertiary transition-colors"
            title="新建对话"
          >
            <Plus className="w-4 h-4 text-apple-text-secondary" />
          </button>
        </div>

        {/* Search */}
        <div className="relative">
          <Search className="absolute left-apple-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text-tertiary" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索对话..."
            className="w-full pl-apple-9 pr-apple-3 py-apple-2 bg-apple-bg-primary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
          />
        </div>
      </div>

      {/* Conversation list */}
      <div className="flex-1 overflow-y-auto p-apple-2">
        {filteredConversations.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-apple-8 text-center">
            <MessageSquare className="w-10 h-10 text-apple-text-tertiary mb-apple-3" />
            <p className="text-apple-sm text-apple-text-secondary">
              {searchQuery ? '没有找到匹配的对话' : '还没有对话'}
            </p>
            {!searchQuery && (
              <button
                onClick={() => {
                  const id = createConversation();
                  setActiveConversation(id);
                }}
                className="mt-apple-3 text-apple-sm text-apple-accent-blue hover:underline"
              >
                开始新对话
              </button>
            )}
          </div>
        ) : (
          <div className="space-y-apple-4">
            {groupedConversations.map(([group, items]) => (
              <div key={group}>
                <h3 className="px-apple-2 mb-apple-2 text-apple-xs font-medium text-apple-text-tertiary uppercase tracking-wider">
                  {group}
                </h3>
                <div className="space-y-apple-1">
                  {items.map((conv) => (
                    <ConversationItem
                      key={conv.id}
                      conversation={conv}
                      isActive={conv.id === activeId}
                      isEditing={conv.id === editingId}
                      editTitle={editTitle}
                      onEditChange={setEditTitle}
                      onStartEdit={() => handleStartEdit(conv)}
                      onSaveEdit={handleSaveEdit}
                      onCancelEdit={handleCancelEdit}
                      onSelect={() => setActiveConversation(conv.id)}
                      onDelete={() => deleteConversation(conv.id)}
                      onTogglePin={() =>
                        conv.isPinned ? unpinConversation(conv.id) : pinConversation(conv.id)
                      }
                      formatTime={formatTime}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer stats */}
      <div className="p-apple-3 border-t border-apple-border">
        <p className="text-apple-xs text-apple-text-tertiary text-center">
          {conversations.length} 个对话 · {conversations.filter((c) => c.isPinned).length} 置顶
        </p>
      </div>
    </div>
  );
};

// =============================================================================
// Conversation Item Component
// =============================================================================

interface ConversationItemProps {
  conversation: Conversation;
  isActive: boolean;
  isEditing: boolean;
  editTitle: string;
  onEditChange: (value: string) => void;
  onStartEdit: () => void;
  onSaveEdit: () => void;
  onCancelEdit: () => void;
  onSelect: () => void;
  onDelete: () => void;
  onTogglePin: () => void;
  formatTime: (timestamp: number) => string;
}

const ConversationItem: React.FC<ConversationItemProps> = ({
  conversation,
  isActive,
  isEditing,
  editTitle,
  onEditChange,
  onStartEdit,
  onSaveEdit,
  onCancelEdit,
  onSelect,
  onDelete,
  onTogglePin,
  formatTime,
}) => {
  const [showMenu, setShowMenu] = useState(false);

  return (
    <div
      className={`group relative flex items-center gap-apple-2 px-apple-3 py-apple-2.5 rounded-apple-lg cursor-pointer transition-all ${
        isActive
          ? 'bg-apple-accent-blue/10 border border-apple-accent-blue/20'
          : 'hover:bg-apple-bg-tertiary border border-transparent'
      }`}
      onClick={isEditing ? undefined : onSelect}
    >
      {/* Icon */}
      <div className="flex-shrink-0">
        {conversation.isPinned ? (
          <Pin className="w-4 h-4 text-apple-accent-purple" />
        ) : (
          <MessageSquare className="w-4 h-4 text-apple-text-tertiary" />
        )}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {isEditing ? (
          <div className="flex items-center gap-apple-1">
            <input
              type="text"
              value={editTitle}
              onChange={(e) => onEditChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') onSaveEdit();
                if (e.key === 'Escape') onCancelEdit();
              }}
              onClick={(e) => e.stopPropagation()}
              autoFocus
              className="flex-1 min-w-0 px-apple-2 py-apple-1 text-apple-sm text-apple-text-primary bg-apple-bg-primary border border-apple-accent-blue rounded-apple-md focus:outline-none"
            />
            <button
              onClick={(e) => {
                e.stopPropagation();
                onSaveEdit();
              }}
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-primary"
            >
              <Check className="w-3.5 h-3.5 text-apple-accent-green" />
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onCancelEdit();
              }}
              className="p-apple-1 rounded-apple-sm hover:bg-apple-bg-primary"
            >
              <X className="w-3.5 h-3.5 text-apple-accent-red" />
            </button>
          </div>
        ) : (
          <>
            <p className={`text-apple-sm truncate ${isActive ? 'font-medium text-apple-text-primary' : 'text-apple-text-primary'}`}>
              {conversation.title}
            </p>
            <div className="flex items-center gap-apple-2 mt-apple-0.5">
              <span className="text-apple-xs text-apple-text-tertiary flex items-center gap-apple-1">
                <Clock className="w-3 h-3" />
                {formatTime(conversation.updatedAt)}
              </span>
              {conversation.messageIds.length > 0 && (
                <span className="text-apple-xs text-apple-text-tertiary">
                  {conversation.messageIds.length} 条消息
                </span>
              )}
            </div>
          </>
        )}
      </div>

      {/* Actions menu */}
      {!isEditing && (
        <div className="relative" onClick={(e) => e.stopPropagation()}>
          <button
            onClick={() => setShowMenu(!showMenu)}
            className="p-apple-1 rounded-apple-sm opacity-0 group-hover:opacity-100 hover:bg-apple-bg-primary transition-all"
          >
            <MoreVertical className="w-4 h-4 text-apple-text-tertiary" />
          </button>

          <AnimatePresence>
            {showMenu && (
              <>
                <div
                  className="fixed inset-0 z-40"
                  onClick={() => setShowMenu(false)}
                />
                <motion.div
                  initial={{ opacity: 0, scale: 0.95, y: -5 }}
                  animate={{ opacity: 1, scale: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.95, y: -5 }}
                  className="absolute right-0 top-full mt-apple-1 py-apple-1 bg-apple-bg-secondary rounded-apple-lg shadow-apple-lg border border-apple-border z-50 min-w-[140px]"
                >
                  <button
                    onClick={() => {
                      onTogglePin();
                      setShowMenu(false);
                    }}
                    className="w-full px-apple-3 py-apple-1.5 text-apple-sm text-apple-text-primary hover:bg-apple-bg-tertiary flex items-center gap-apple-2"
                  >
                    <Pin className="w-4 h-4" />
                    {conversation.isPinned ? '取消置顶' : '置顶对话'}
                  </button>
                  <button
                    onClick={() => {
                      onStartEdit();
                      setShowMenu(false);
                    }}
                    className="w-full px-apple-3 py-apple-1.5 text-apple-sm text-apple-text-primary hover:bg-apple-bg-tertiary flex items-center gap-apple-2"
                  >
                    <Edit2 className="w-4 h-4" />
                    重命名
                  </button>
                  <button
                    onClick={() => {
                      onDelete();
                      setShowMenu(false);
                    }}
                    className="w-full px-apple-3 py-apple-1.5 text-apple-sm text-apple-accent-red hover:bg-apple-accent-red/10 flex items-center gap-apple-2"
                  >
                    <Trash2 className="w-4 h-4" />
                    删除
                  </button>
                </motion.div>
              </>
            )}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
};
