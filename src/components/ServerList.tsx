import React, { useState } from 'react';
import { useServerStore } from '../stores/serverStore';
import { useSessionStore } from '../stores/sessionStore';
import { useUiStore } from '../stores/uiStore';
import { Button, StatusDot } from './design-system';
import { EditServerModal } from './EditServerModal';
import { EditGroupModal } from './EditGroupModal';
import type { Server, Group } from '../types';

interface ServerItemProps {
  server: Server;
  onConnect: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

export const ServerItem: React.FC<ServerItemProps> = ({
  server,
  onConnect,
  onEdit,
  onDelete,
}) => {
  const [showMenu, setShowMenu] = useState(false);

  return (
    <div
      className="flex items-center justify-between px-4 py-2 hover:bg-slate-700/50 cursor-pointer group"
      onClick={() => setShowMenu(!showMenu)}
    >
      <div className="flex items-center gap-3 flex-1">
        <StatusDot variant={server.status} />
        <div className="flex flex-col">
          <span className="text-sm font-medium text-white">{server.name}</span>
          <span className="text-xs text-slate-400">
            {server.username}@{server.host}:{server.port}
          </span>
        </div>
      </div>

      {showMenu && (
        <div className="flex items-center gap-2 bg-slate-800 rounded-md py-1">
          <Button size="sm" variant="primary" onClick={(e) => { e.stopPropagation(); onConnect(); }}>
            连接
          </Button>
          <Button size="sm" variant="secondary" onClick={(e) => { e.stopPropagation(); onEdit(); }}>
            编辑
          </Button>
          <Button size="sm" variant="danger" onClick={(e) => { e.stopPropagation(); onDelete(); }}>
            删除
          </Button>
        </div>
      )}
    </div>
  );
};

export const ServerList: React.FC = () => {
  const { servers, groups, deleteServer, deleteGroup, connectServerNative } = useServerStore();
  const { searchQuery, productMode } = useUiStore();
  const { sshConnect } = useSessionStore();
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set(['ungrouped']));
  const [editingServer, setEditingServer] = useState<Server | null>(null);
  const [editingGroup, setEditingGroup] = useState<Group | null>(null);

  const filteredServers = servers.filter(
    (s) =>
      s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.host.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.username.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const serversByGroup = filteredServers.reduce((acc, server) => {
    const groupId = server.group_id || 'ungrouped';
    if (!acc[groupId]) {
      acc[groupId] = [];
    }
    acc[groupId].push(server);
    return acc;
  }, {} as Record<string, Server[]>);

  const toggleGroup = (groupId: string) => {
    const newExpanded = new Set(expandedGroups);
    if (newExpanded.has(groupId)) {
      newExpanded.delete(groupId);
    } else {
      newExpanded.add(groupId);
    }
    setExpandedGroups(newExpanded);
  };

  const handleConnect = async (server: Server) => {
    if (productMode === 'standard') {
      await sshConnect(server.id);
      return;
    }

    await connectServerNative(server.id);
  };

  const handleDelete = async (id: string) => {
    if (confirm('确定要删除这个服务器吗？')) {
      await deleteServer(id);
    }
  };

  const handleDeleteGroup = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirm('确定要删除这个分组吗？分组内的服务器将变为未分组状态。')) {
      await deleteGroup(id);
    }
  };

  const handleEditGroup = (group: Group, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingGroup(group);
  };

  const renderGroups = () => {
    const items: React.ReactNode[] = [];

    groups.forEach((group) => {
      const groupServers = serversByGroup[group.id] || [];
      if (groupServers.length === 0) return;

      items.push(
        <div key={group.id} className="mb-2 group">
          <div
            className="flex items-center gap-2 px-4 py-2 cursor-pointer hover:bg-slate-700/50"
            onClick={() => toggleGroup(group.id)}
          >
            <span className="text-slate-400">
              {expandedGroups.has(group.id) ? '▼' : '▶'}
            </span>
            <span className="text-sm font-semibold text-slate-300">
              {group.name} ({groupServers.length})
            </span>
            <button
              onClick={(e) => handleEditGroup(group, e)}
              className="ml-auto opacity-0 group-hover:opacity-100 text-slate-500 hover:text-cyan-400 transition-opacity text-xs px-2 py-0.5 rounded hover:bg-slate-700"
              title="编辑分组"
            >
              编辑
            </button>
            <button
              onClick={(e) => handleDeleteGroup(group.id, e)}
              className="opacity-0 group-hover:opacity-100 text-slate-500 hover:text-red-400 transition-opacity text-xs px-2 py-0.5 rounded hover:bg-slate-700"
              title="删除分组"
            >
              ✕
            </button>
          </div>
          {expandedGroups.has(group.id) && (
            <div className="ml-4">
              {groupServers.map((server) => (
                <ServerItem
                  key={server.id}
                  server={server}
                  onConnect={() => handleConnect(server)}
                  onEdit={() => setEditingServer(server)}
                  onDelete={() => handleDelete(server.id)}
                />
              ))}
            </div>
          )}
        </div>
      );
    });

    const ungroupedServers = serversByGroup['ungrouped'] || [];
    if (ungroupedServers.length > 0) {
      items.push(
        <div key="ungrouped" className="mb-2">
          <div
            className="flex items-center gap-2 px-4 py-2 cursor-pointer hover:bg-slate-700/50"
            onClick={() => toggleGroup('ungrouped')}
          >
            <span className="text-slate-400">
              {expandedGroups.has('ungrouped') ? '▼' : '▶'}
            </span>
            <span className="text-sm font-semibold text-slate-300">
              未分组 ({ungroupedServers.length})
            </span>
          </div>
          {expandedGroups.has('ungrouped') && (
            <div className="ml-4">
              {ungroupedServers.map((server) => (
                <ServerItem
                  key={server.id}
                  server={server}
                  onConnect={() => handleConnect(server)}
                  onEdit={() => setEditingServer(server)}
                  onDelete={() => handleDelete(server.id)}
                />
              ))}
            </div>
          )}
        </div>
      );
    }

    return items;
  };

  return (
    <div className="flex-1 overflow-y-auto">
      {servers.length === 0 ? (
        <div className="flex items-center justify-center h-full text-slate-500">
          <p>暂无服务器，点击下方按钮添加</p>
        </div>
      ) : (
        renderGroups()
      )}
      <EditServerModal
        isOpen={editingServer !== null}
        onClose={() => setEditingServer(null)}
        server={editingServer}
      />
      <EditGroupModal
        isOpen={editingGroup !== null}
        onClose={() => setEditingGroup(null)}
        group={editingGroup}
      />
    </div>
  );
};
