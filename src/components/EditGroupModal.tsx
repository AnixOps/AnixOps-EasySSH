import React, { useState, useEffect } from 'react';
import { useServerStore } from '../stores/serverStore';
import { Button, Input, Modal } from './design-system';
import type { Group } from '../types';

interface EditGroupModalProps {
  isOpen: boolean;
  onClose: () => void;
  group: Group | null;
}

export const EditGroupModal: React.FC<EditGroupModalProps> = ({ isOpen, onClose, group }) => {
  const { updateGroup } = useServerStore();
  const [name, setName] = useState('');

  useEffect(() => {
    if (group) {
      setName(group.name);
    }
  }, [group]);

  const handleSubmit = async () => {
    if (!group || !name.trim()) return;
    await updateGroup({ id: group.id, name: name.trim() });
    handleClose();
  };

  const handleClose = () => {
    setName('');
    onClose();
  };

  if (!isOpen || !group) return null;

  return (
    <Modal title="编辑分组" onClose={handleClose}>
      <div>
        <label className="block text-xs uppercase tracking-[0.2em] text-slate-500 mb-1">分组名称</label>
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Production"
          autoFocus
          onKeyDown={(e) => e.key === 'Enter' && handleSubmit()}
        />
      </div>
      <div className="flex justify-end gap-2 mt-6">
        <Button variant="ghost" onClick={handleClose}>
          取消
        </Button>
        <Button variant="primary" onClick={handleSubmit} disabled={!name.trim()}>
          保存
        </Button>
      </div>
    </Modal>
  );
};
