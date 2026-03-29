import React, { useState } from 'react';
import { useServerStore } from '../stores/serverStore';
import { Button, Input, Modal, RadioButton } from './design-system';
import type { Server, UpdateServer } from '../types';

interface EditServerModalProps {
  isOpen: boolean;
  onClose: () => void;
  server: Server | null;
}

export const EditServerModal: React.FC<EditServerModalProps> = ({ isOpen, onClose, server }) => {
  const { groups, updateServer } = useServerStore();
  const [formData, setFormData] = useState({
    name: '',
    host: '',
    port: 22,
    username: '',
    authType: 'agent' as 'agent' | 'key' | 'password',
    identityFile: '',
    groupId: '' as string | undefined,
  });

  // Update form when server changes
  React.useEffect(() => {
    if (server) {
      setFormData({
        name: server.name,
        host: server.host,
        port: server.port,
        username: server.username,
        authType: server.auth_type as 'agent' | 'key' | 'password',
        identityFile: server.identity_file || '',
        groupId: server.group_id,
      });
    }
  }, [server]);

  const handleSubmit = async () => {
    if (!server) return;
    const updateData: UpdateServer = {
      id: server.id,
      name: formData.name,
      host: formData.host,
      port: formData.port,
      username: formData.username,
      auth_type: formData.authType,
      identity_file: formData.identityFile || undefined,
      group_id: formData.groupId,
      status: server.status,
    };
    await updateServer(updateData);
    handleClose();
  };

  const handleClose = () => {
    setFormData({
      name: '',
      host: '',
      port: 22,
      username: '',
      authType: 'agent',
      identityFile: '',
      groupId: undefined,
    });
    onClose();
  };

  if (!isOpen || !server) return null;

  return (
    <Modal title="编辑服务器" onClose={handleClose}>
      <div className="space-y-4">
        <div>
          <label className="block text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">名称</label>
          <Input
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            placeholder="Web Server 1"
          />
        </div>
        <div>
          <label className="block text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">主机</label>
          <Input
            value={formData.host}
            onChange={(e) => setFormData({ ...formData, host: e.target.value })}
            placeholder="192.168.1.10"
          />
        </div>
        <div>
          <label className="block text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">端口</label>
          <Input
            type="number"
            value={formData.port}
            onChange={(e) => setFormData({ ...formData, port: parseInt(e.target.value) || 22 })}
          />
        </div>
        <div>
          <label className="block text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">用户名</label>
          <Input
            value={formData.username}
            onChange={(e) => setFormData({ ...formData, username: e.target.value })}
            placeholder="admin"
          />
        </div>

        <div className="space-y-2">
          <label className="block text-xs uppercase tracking-[0.2em] text-slate-500">认证方式</label>
          <RadioButton
            name="authType"
            label="SSH Agent (推荐)"
            description="使用系统SSH Agent"
            checked={formData.authType === 'agent'}
            onChange={() => setFormData({ ...formData, authType: 'agent' })}
          />

          <RadioButton
            name="authType"
            label="SSH密钥"
            description="使用私钥文件认证"
            checked={formData.authType === 'key'}
            onChange={() => setFormData({ ...formData, authType: 'key' })}
          />

          {formData.authType === 'key' && (
            <div className="ml-8 mt-2">
              <Input
                value={formData.identityFile}
                onChange={(e) => setFormData({ ...formData, identityFile: e.target.value })}
                placeholder="~/.ssh/id_rsa"
              />
            </div>
          )}

          <RadioButton
            name="authType"
            label="密码"
            description="使用密码认证（不推荐）"
            checked={formData.authType === 'password'}
            onChange={() => setFormData({ ...formData, authType: 'password' })}
          />
        </div>

        <div>
          <label className="block text-xs uppercase tracking-[0.2em] text-slate-500 mb-2">选择分组</label>
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {groups.map((group) => (
              <RadioButton
                key={group.id}
                name="group"
                label={group.name}
                checked={formData.groupId === group.id}
                onChange={() => setFormData({ ...formData, groupId: group.id })}
              />
            ))}
            <RadioButton
              name="group"
              label="不分组"
              checked={!formData.groupId}
              onChange={() => setFormData({ ...formData, groupId: undefined })}
            />
          </div>
        </div>

        <div className="flex justify-end gap-2 mt-6">
          <Button variant="ghost" onClick={handleClose}>
            取消
          </Button>
          <Button
            variant="primary"
            onClick={handleSubmit}
            disabled={!formData.name || !formData.host || !formData.username}
          >
            保存
          </Button>
        </div>
      </div>
    </Modal>
  );
};
