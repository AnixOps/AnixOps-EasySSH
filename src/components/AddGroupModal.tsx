import React, { useState } from 'react';
import { v4 as uuidv4 } from 'uuid';
import { useServerStore } from '../stores/serverStore';
import { Button, Input, Modal } from './design-system';

interface AddGroupModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AddGroupModal: React.FC<AddGroupModalProps> = ({ isOpen, onClose }) => {
  const { addGroup } = useServerStore();
  const [name, setName] = useState('');

  const handleSubmit = async () => {
    if (!name.trim()) return;
    await addGroup({ id: uuidv4(), name: name.trim() });
    handleClose();
  };

  const handleClose = () => {
    setName('');
    onClose();
  };

  if (!isOpen) return null;

  return (
    <Modal title="添加分组" onClose={handleClose}>
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
          添加
        </Button>
      </div>
    </Modal>
  );
};
