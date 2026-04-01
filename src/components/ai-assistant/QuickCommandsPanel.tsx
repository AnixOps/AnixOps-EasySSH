/**
 * Quick Commands Panel Component
 * @module components/ai-assistant/QuickCommandsPanel
 */

import React, { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  X,
  Search,
  Code,
  Terminal,
  Globe,
  FileText,
  Wand2,
  Bug,
  TestTube,
  Shield,
  AlertCircle,
  Plus,
  Edit2,
  Trash2,
  Sparkles,
} from 'lucide-react';

// Store
import { useAIAssistantStore, useAIQuickCommands } from '../../stores/aiAssistantStore';

// Types
import type { QuickCommand } from '../../types/aiAssistant';

// =============================================================================
// Types
// =============================================================================

interface QuickCommandsPanelProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (command: QuickCommand) => void;
}

// =============================================================================
// Component
// =============================================================================

export const QuickCommandsPanel: React.FC<QuickCommandsPanelProps> = ({
  isOpen,
  onClose,
  onSelect,
}) => {
  const commands = useAIQuickCommands();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [editingCommand, setEditingCommand] = useState<QuickCommand | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);

  const { deleteQuickCommand, addQuickCommand, updateQuickCommand } = useAIAssistantStore();

  // Categories
  const categories = useMemo(() => {
    const cats = new Set(commands.map((c) => c.category));
    return Array.from(cats);
  }, [commands]);

  // Filter commands
  const filteredCommands = useMemo(() => {
    let filtered = commands;

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = commands.filter(
        (c) =>
          c.name.toLowerCase().includes(query) ||
          c.description?.toLowerCase().includes(query) ||
          c.category.toLowerCase().includes(query)
      );
    }

    if (selectedCategory) {
      filtered = filtered.filter((c) => c.category === selectedCategory);
    }

    return filtered;
  }, [commands, searchQuery, selectedCategory]);

  // Group by category
  const groupedCommands = useMemo(() => {
    const groups: Record<string, QuickCommand[]> = {};
    filteredCommands.forEach((cmd) => {
      if (!groups[cmd.category]) {
        groups[cmd.category] = [];
      }
      groups[cmd.category].push(cmd);
    });
    return Object.entries(groups);
  }, [filteredCommands]);

  // Category icons
  const getCategoryIcon = (category: string) => {
    const iconMap: Record<string, typeof Code> = {
      '代码': Code,
      'SSH': Terminal,
      '通用': Globe,
      '文档': FileText,
      '调试': Bug,
      '测试': TestTube,
      '安全': Shield,
    };
    return iconMap[category] || Sparkles;
  };

  // Handle delete
  const handleDelete = (id: string) => {
    if (confirm('确定要删除这个快捷指令吗？')) {
      deleteQuickCommand(id);
    }
  };

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-apple-4"
        onClick={(e) => {
          if (e.target === e.currentTarget) onClose();
        }}
      >
        <motion.div
          initial={{ scale: 0.95, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0.95, opacity: 0 }}
          className="w-full max-w-3xl max-h-[80vh] bg-apple-bg-primary rounded-apple-xl shadow-apple-xl border border-apple-border overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-apple-6 py-apple-4 border-b border-apple-border">
            <div>
              <h2 className="text-apple-lg font-semibold text-apple-text-primary">快捷指令</h2>
              <p className="text-apple-sm text-apple-text-secondary">选择或创建预设提示词模板</p>
            </div>
            <div className="flex items-center gap-apple-2">
              <button
                onClick={() => setShowAddForm(true)}
                className="flex items-center gap-apple-2 px-apple-3 py-apple-2 bg-apple-accent-blue text-white rounded-apple-lg text-apple-sm font-medium hover:bg-apple-accent-blue/90 transition-colors"
              >
                <Plus className="w-4 h-4" />
                新建
              </button>
              <button
                onClick={onClose}
                className="p-apple-2 rounded-apple-lg hover:bg-apple-bg-tertiary transition-colors"
              >
                <X className="w-5 h-5 text-apple-text-secondary" />
              </button>
            </div>
          </div>

          {/* Search and filters */}
          <div className="px-apple-6 py-apple-4 border-b border-apple-border space-y-apple-3">
            <div className="relative">
              <Search className="absolute left-apple-3 top-1/2 -translate-y-1/2 w-4 h-4 text-apple-text-tertiary" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="搜索快捷指令..."
                className="w-full pl-apple-9 pr-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
                autoFocus
              />
            </div>

            {/* Category filters */}
            <div className="flex items-center gap-apple-2 flex-wrap">
              <button
                onClick={() => setSelectedCategory(null)}
                className={`px-apple-3 py-apple-1.5 rounded-apple-md text-apple-xs font-medium transition-colors ${
                  selectedCategory === null
                    ? 'bg-apple-accent-blue text-white'
                    : 'bg-apple-bg-tertiary text-apple-text-secondary hover:bg-apple-bg-secondary'
                }`}
              >
                全部
              </button>
              {categories.map((category) => {
                const Icon = getCategoryIcon(category);
                return (
                  <button
                    key={category}
                    onClick={() => setSelectedCategory(category === selectedCategory ? null : category)}
                    className={`flex items-center gap-apple-1.5 px-apple-3 py-apple-1.5 rounded-apple-md text-apple-xs font-medium transition-colors ${
                      selectedCategory === category
                        ? 'bg-apple-accent-blue text-white'
                        : 'bg-apple-bg-tertiary text-apple-text-secondary hover:bg-apple-bg-secondary'
                    }`}
                  >
                    <Icon className="w-3.5 h-3.5" />
                    {category}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Commands list */}
          <div className="overflow-y-auto max-h-[50vh] p-apple-6">
            {groupedCommands.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-apple-8 text-center">
                <Sparkles className="w-10 h-10 text-apple-text-tertiary mb-apple-3" />
                <p className="text-apple-sm text-apple-text-secondary">
                  {searchQuery ? '没有找到匹配的指令' : '暂无快捷指令'}
                </p>
              </div>
            ) : (
              <div className="space-y-apple-6">
                {groupedCommands.map(([category, cmds]) => {
                  const Icon = getCategoryIcon(category);
                  return (
                    <div key={category}>
                      <h3 className="flex items-center gap-apple-2 text-apple-xs font-semibold text-apple-text-tertiary uppercase tracking-wider mb-apple-3">
                        <Icon className="w-4 h-4" />
                        {category}
                      </h3>
                      <div className="grid grid-cols-1 sm:grid-cols-2 gap-apple-3">
                        {cmds.map((cmd) => (
                          <CommandCard
                            key={cmd.id}
                            command={cmd}
                            onSelect={() => {
                              onSelect(cmd);
                              onClose();
                            }}
                            onEdit={() => setEditingCommand(cmd)}
                            onDelete={() => handleDelete(cmd.id)}
                          />
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </motion.div>

        {/* Add/Edit Form Modal */}
        {(showAddForm || editingCommand) && (
          <CommandFormModal
            command={editingCommand}
            onClose={() => {
              setShowAddForm(false);
              setEditingCommand(null);
            }}
            onSave={(data) => {
              if (editingCommand) {
                updateQuickCommand(editingCommand.id, data);
              } else {
                addQuickCommand(data);
              }
              setShowAddForm(false);
              setEditingCommand(null);
            }}
          />
        )}
      </motion.div>
    </AnimatePresence>
  );
};

// =============================================================================
// Command Card Component
// =============================================================================

interface CommandCardProps {
  command: QuickCommand;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

const CommandCard: React.FC<CommandCardProps> = ({ command, onSelect, onEdit, onDelete }) => {
  const [showActions, setShowActions] = useState(false);

  // Get icon component
  const getIcon = (iconName?: string) => {
    const iconMap: Record<string, typeof Code> = {
      'Code': Code,
      'Terminal': Terminal,
      'Wand2': Wand2,
      'Bug': Bug,
      'TestTube': TestTube,
      'FileText': FileText,
      'Shield': Shield,
      'AlertCircle': AlertCircle,
    };
    return iconMap[iconName || ''] || Sparkles;
  };

  const Icon = getIcon(command.icon);

  return (
    <motion.div
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onHoverStart={() => setShowActions(true)}
      onHoverEnd={() => setShowActions(false)}
      onClick={onSelect}
      className="group relative p-apple-4 bg-apple-bg-secondary rounded-apple-lg border border-apple-border hover:border-apple-accent-purple/50 cursor-pointer transition-all"
    >
      <div className="flex items-start gap-apple-3">
        <div className="w-10 h-10 rounded-apple-lg bg-apple-accent-purple/10 flex items-center justify-center flex-shrink-0">
          <Icon className="w-5 h-5 text-apple-accent-purple" />
        </div>
        <div className="flex-1 min-w-0">
          <h4 className="text-apple-sm font-semibold text-apple-text-primary truncate">
            {command.name}
          </h4>
          {command.description && (
            <p className="text-apple-xs text-apple-text-secondary mt-apple-1 line-clamp-2">
              {command.description}
            </p>
          )}

          {/* Variables indicator */}
          {command.variables && command.variables.length > 0 && (
            <div className="flex items-center gap-apple-1 mt-apple-2 flex-wrap">
              {command.variables.map((v) => (
                <span
                  key={v}
                  className="px-apple-1.5 py-apple-0.5 bg-apple-bg-tertiary rounded-apple-sm text-apple-xs text-apple-text-tertiary"
                >
                  {'{'}{v}{'}'}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Actions */}
        {!command.isBuiltin && (
          <div
            className={`flex items-center gap-apple-1 transition-opacity ${
              showActions ? 'opacity-100' : 'opacity-0'
            }`}
          >
            <button
              onClick={(e) => {
                e.stopPropagation();
                onEdit();
              }}
              className="p-apple-1.5 rounded-apple-sm hover:bg-apple-bg-tertiary"
            >
              <Edit2 className="w-4 h-4 text-apple-text-tertiary" />
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
              className="p-apple-1.5 rounded-apple-sm hover:bg-apple-accent-red/10"
            >
              <Trash2 className="w-4 h-4 text-apple-accent-red" />
            </button>
          </div>
        )}
      </div>

      {/* Builtin badge */}
      {command.isBuiltin && (
        <span className="absolute top-apple-2 right-apple-2 px-apple-1.5 py-apple-0.5 bg-apple-accent-blue/10 text-apple-accent-blue text-apple-xs rounded-apple-sm">
          内置
        </span>
      )}
    </motion.div>
  );
};

// =============================================================================
// Command Form Modal
// =============================================================================

interface CommandFormModalProps {
  command: QuickCommand | null;
  onClose: () => void;
  onSave: (data: { name: string; description?: string; prompt: string; category: string; icon?: string }) => void;
}

const CommandFormModal: React.FC<CommandFormModalProps> = ({ command, onClose, onSave }) => {
  const [name, setName] = useState(command?.name || '');
  const [description, setDescription] = useState(command?.description || '');
  const [prompt, setPrompt] = useState(command?.prompt || '');
  const [category, setCategory] = useState(command?.category || '通用');
  const [icon, setIcon] = useState(command?.icon || 'Sparkles');
  const [newCategory, setNewCategory] = useState('');
  const [showNewCategory, setShowNewCategory] = useState(false);

  const categories = ['代码', 'SSH', '通用', '文档', '调试', '测试', '安全'];
  const icons = ['Sparkles', 'Code', 'Terminal', 'Globe', 'FileText', 'Wand2', 'Bug', 'TestTube', 'Shield', 'AlertCircle'];

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !prompt.trim()) return;

    const finalCategory = showNewCategory && newCategory.trim() ? newCategory.trim() : category;

    onSave({
      name: name.trim(),
      description: description.trim() || undefined,
      prompt: prompt.trim(),
      category: finalCategory,
      icon,
    });
  };

  // Extract variables from prompt
  const variables = useMemo(() => {
    const matches = prompt.match(/\{\{(\w+)\}\}/g);
    return matches ? matches.map((m) => m.replace(/[{}]/g, '')) : [];
  }, [prompt]);

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm p-apple-4">
      <motion.div
        initial={{ scale: 0.95, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.95, opacity: 0 }}
        className="w-full max-w-lg bg-apple-bg-primary rounded-apple-xl shadow-apple-xl border border-apple-border overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="px-apple-6 py-apple-4 border-b border-apple-border">
          <h3 className="text-apple-md font-semibold text-apple-text-primary">
            {command ? '编辑快捷指令' : '新建快捷指令'}
          </h3>
        </div>

        <form onSubmit={handleSubmit} className="p-apple-6 space-y-apple-4">
          {/* Name */}
          <div>
            <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
              名称 *
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：解释代码"
              required
              className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
              描述
            </label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="简要描述这个指令的用途"
              className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
            />
          </div>

          {/* Category */}
          <div>
            <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
              分类
            </label>
            {!showNewCategory ? (
              <div className="flex items-center gap-apple-2">
                <select
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                  className="flex-1 px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary focus:border-apple-accent-blue focus:outline-none"
                >
                  {categories.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  onClick={() => setShowNewCategory(true)}
                  className="px-apple-3 py-apple-2 text-apple-xs text-apple-accent-blue hover:bg-apple-accent-blue/10 rounded-apple-lg transition-colors"
                >
                  + 新分类
                </button>
              </div>
            ) : (
              <div className="flex items-center gap-apple-2">
                <input
                  type="text"
                  value={newCategory}
                  onChange={(e) => setNewCategory(e.target.value)}
                  placeholder="输入新分类名称"
                  className="flex-1 px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
                  autoFocus
                />
                <button
                  type="button"
                  onClick={() => {
                    setShowNewCategory(false);
                    setNewCategory('');
                  }}
                  className="px-apple-3 py-apple-2 text-apple-xs text-apple-text-secondary hover:bg-apple-bg-tertiary rounded-apple-lg transition-colors"
                >
                  取消
                </button>
              </div>
            )}
          </div>

          {/* Icon */}
          <div>
            <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
              图标
            </label>
            <div className="flex items-center gap-apple-2 flex-wrap">
              {icons.map((iconName) => (
                <button
                  key={iconName}
                  type="button"
                  onClick={() => setIcon(iconName)}
                  className={`p-apple-2 rounded-apple-lg transition-colors ${
                    icon === iconName
                      ? 'bg-apple-accent-blue text-white'
                      : 'bg-apple-bg-tertiary text-apple-text-secondary hover:bg-apple-bg-secondary'
                  }`}
                >
                  {iconName}
                </button>
              ))}
            </div>
          </div>

          {/* Prompt */}
          <div>
            <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
              提示词模板 *
            </label>
            <textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="输入提示词模板，使用 {{variable}} 作为变量占位符"
              required
              rows={4}
              className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none resize-none"
            />
            {variables.length > 0 && (
              <p className="text-apple-xs text-apple-text-tertiary mt-apple-1">
                检测到的变量: {variables.map((v) => `{${v}}`).join(', ')}
              </p>
            )}
          </div>

          {/* Actions */}
          <div className="flex items-center justify-end gap-apple-3 pt-apple-4 border-t border-apple-border">
            <button
              type="button"
              onClick={onClose}
              className="px-apple-4 py-apple-2 text-apple-sm font-medium text-apple-text-secondary hover:text-apple-text-primary transition-colors"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={!name.trim() || !prompt.trim()}
              className="px-apple-4 py-apple-2 bg-apple-accent-blue text-white rounded-apple-lg text-apple-sm font-medium hover:bg-apple-accent-blue/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {command ? '保存' : '创建'}
            </button>
          </div>
        </form>
      </motion.div>
    </div>
  );
};
