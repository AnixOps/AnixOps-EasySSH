import React, { useState } from 'react';
import { v4 as uuidv4 } from 'uuid';
import { useServerStore } from '../stores/serverStore';
import { Button, Input, Modal, RadioButton } from './design-system';
import type { NewServer } from '../types';

interface AddServerModalProps {
  isOpen: boolean;
  onClose: () => void;
}

type Step = 1 | 2 | 3;

export const AddServerModal: React.FC<AddServerModalProps> = ({ isOpen, onClose }) => {
  const { groups, addServer } = useServerStore();
  const [step, setStep] = useState<Step>(1);
  const [formData, setFormData] = useState({
    name: '',
    host: '',
    port: 22,
    username: '',
    authType: 'agent' as 'agent' | 'key' | 'password',
    identityFile: '',
    groupId: '' as string | undefined,
  });

  const handleSubmit = async () => {
    const server: NewServer = {
      id: uuidv4(),
      name: formData.name,
      host: formData.host,
      port: formData.port,
      username: formData.username,
      auth_type: formData.authType,
      identity_file: formData.identityFile || undefined,
      group_id: formData.groupId || undefined,
      status: 'unknown',
    };
    await addServer(server);
    handleClose();
  };

  const handleClose = () => {
    setStep(1);
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

  if (!isOpen) return null;

  return (
    <Modal title="添加服务器" onClose={handleClose}>
        {step === 1 && (
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
            <div className="flex justify-end gap-2 mt-6">
              <Button variant="ghost" onClick={handleClose}>
                取消
              </Button>
              <Button
                variant="primary"
                onClick={() => setStep(2)}
                disabled={!formData.name || !formData.host || !formData.username}
              >
                下一步
              </Button>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="space-y-4">
            <div className="space-y-2">
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
            <div className="flex justify-between mt-6">
              <Button variant="ghost" onClick={() => setStep(1)}>
                上一步
              </Button>
              <Button variant="primary" onClick={() => setStep(3)}>
                下一步
              </Button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className="space-y-4">
            <p className="text-xs uppercase tracking-[0.2em] text-slate-500 mb-2">选择分组</p>
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
            <div className="flex justify-between mt-6">
              <Button variant="ghost" onClick={() => setStep(2)}>
                上一步
              </Button>
              <Button variant="primary" onClick={handleSubmit}>
                完成
              </Button>
            </div>
          </div>
        )}
    </Modal>
  );
};
