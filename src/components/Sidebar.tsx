import React, { useState } from 'react';
import { useServerStore } from '../stores/serverStore';
import { useUiStore } from '../stores/uiStore';
import { ServerList } from './ServerList';
import { AddServerModal } from './AddServerModal';
import { AddGroupModal } from './AddGroupModal';
import { Button, Input } from './design-system';

export const Sidebar: React.FC = () => {
  const { searchQuery, setSearchQuery } = useUiStore();
  const { servers, groups } = useServerStore();
  const [showAddServer, setShowAddServer] = useState(false);
  const [showAddGroup, setShowAddGroup] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  return (
    <aside className="flex h-full w-80 flex-col bg-slate-950/95">
      <div className="flex items-center justify-between border-b border-slate-800 px-4 py-4">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Navigation</p>
          <h2 className="text-base font-semibold text-slate-50">Servers</h2>
        </div>
        <Button
          variant="ghost"
          onClick={() => setShowSettings(!showSettings)}
          title="Settings"
        >
          ⚙️
        </Button>
      </div>

      <div className="border-b border-slate-800 px-4 py-3">
        <Input
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="搜索服务器..."
        />
      </div>

      {showSettings && (
        <div className="border-b border-slate-800 px-4 py-3">
          <p className="mb-2 text-xs uppercase tracking-[0.2em] text-slate-500">Settings</p>
          <div className="space-y-2">
            <button
              onClick={() => setShowAddGroup(true)}
              className="w-full rounded-xl px-3 py-2 text-left text-sm text-slate-300 transition hover:bg-slate-900 hover:text-white"
            >
              + 添加分组
            </button>
            <button
              onClick={() => {
                localStorage.clear();
                window.location.reload();
              }}
              className="w-full rounded-xl px-3 py-2 text-left text-sm text-slate-300 transition hover:bg-slate-900 hover:text-white"
            >
              清除本地数据
            </button>
          </div>
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-hidden">
        <ServerList />
      </div>

      <div className="border-t border-slate-800 px-4 py-3">
        <div className="grid grid-cols-2 gap-2">
          <Button variant="primary" onClick={() => setShowAddServer(true)}>
            + 添加服务器
          </Button>
          <Button variant="secondary" onClick={() => setShowAddGroup(true)}>
            + 添加分组
          </Button>
        </div>
        <p className="mt-3 text-xs text-slate-500">
          {groups.length} 分组 · {servers.length} 服务器
        </p>
      </div>

      <AddServerModal isOpen={showAddServer} onClose={() => setShowAddServer(false)} />
      <AddGroupModal isOpen={showAddGroup} onClose={() => setShowAddGroup(false)} />
    </aside>
  );
};
