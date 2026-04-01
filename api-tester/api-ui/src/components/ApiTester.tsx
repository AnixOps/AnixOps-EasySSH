import React, { useState } from 'react';
import { Sidebar } from './Sidebar';
import { RequestBuilder } from './RequestBuilder';
import { ResponseViewer } from './ResponseViewer';
import { WebSocketClient } from './WebSocketClient';
import { useApiTesterStore, ApiRequest } from '../stores/apiTesterStore';
import { saveRequest } from '../utils/tauriCommands';

export const ApiTester: React.FC = () => {
  const [activeView, setActiveView] = useState<'http' | 'websocket' | 'grpc'>('http');
  const [showSaveModal, setShowSaveModal] = useState(false);

  const {
    currentRequest,
    setCurrentRequest,
    newRequest,
    collections,
    sidebarVisible,
    sidebarWidth,
  } = useApiTesterStore();

  const handleSelectRequest = (request: ApiRequest) => {
    setCurrentRequest(request);
  };

  const handleNewRequest = () => {
    newRequest();
  };

  const handleSave = async () => {
    if (collections.length === 0) {
      // Create a default collection
      setShowSaveModal(true);
      return;
    }
    setShowSaveModal(true);
  };

  const handleSaveToCollection = async (collectionId: string, folderId?: string) => {
    try {
      await saveRequest(currentRequest, collectionId, folderId);
      setShowSaveModal(false);
    } catch (error) {
      console.error('Failed to save request:', error);
    }
  };

  return (
    <div className="h-screen flex flex-col bg-white dark:bg-gray-900">
      {/* Top Navigation */}
      <div className="flex items-center justify-between px-4 py-2 border-b">
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold">API Tester</h1>
          <div className="flex bg-gray-100 rounded-md p-1">
            <button
              onClick={() => setActiveView('http')}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                activeView === 'http' ? 'bg-white shadow-sm' : 'text-gray-600 hover:text-gray-800'
              }`}
            >
              HTTP
            </button>
            <button
              onClick={() => setActiveView('websocket')}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                activeView === 'websocket' ? 'bg-white shadow-sm' : 'text-gray-600 hover:text-gray-800'
              }`}
            >
              WebSocket
            </button>
            <button
              onClick={() => setActiveView('grpc')}
              className={`px-3 py-1 text-sm rounded-md transition-colors ${
                activeView === 'grpc' ? 'bg-white shadow-sm' : 'text-gray-600 hover:text-gray-800'
              }`}
            >
              gRPC
            </button>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-500">EasySSH API Tester</span>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        {sidebarVisible && (
          <div
            className="flex-shrink-0 border-r"
            style={{ width: sidebarWidth }}
          >
            <Sidebar
              onSelectRequest={handleSelectRequest}
              onNewRequest={handleNewRequest}
            />
          </div>
        )}

        {/* Content Area */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {activeView === 'http' && (
            <>
              {/* Request Builder - Top Half */}
              <div className="flex-1 min-h-0 border-b">
                <RequestBuilder onSend={() => {}} onSave={handleSave} />
              </div>

              {/* Response Viewer - Bottom Half */}
              <div className="flex-1 min-h-0">
                <ResponseViewer />
              </div>
            </>
          )}

          {activeView === 'websocket' && (
            <div className="flex-1 h-full">
              <WebSocketClient />
            </div>
          )}

          {activeView === 'grpc' && (
            <div className="flex-1 h-full flex items-center justify-center text-gray-500">
              <div className="text-center">
                <p className="text-lg mb-2">gRPC Client</p>
                <p className="text-sm">gRPC support is coming soon</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Save Modal */}
      {showSaveModal && (
        <SaveRequestModal
          onClose={() => setShowSaveModal(false)}
          onSave={handleSaveToCollection}
          requestName={currentRequest.name}
        />
      )}
    </div>
  );
};

interface SaveRequestModalProps {
  onClose: () => void;
  onSave: (collectionId: string, folderId?: string) => void;
  requestName: string;
}

const SaveRequestModal: React.FC<SaveRequestModalProps> = ({
  onClose,
  onSave,
  requestName,
}) => {
  const [selectedCollection, setSelectedCollection] = useState<string>('');
  const [selectedFolder, setSelectedFolder] = useState<string>('');
  const [newCollectionName, setNewCollectionName] = useState('');
  const { collections, addCollection } = useApiTesterStore();

  const handleSave = () => {
    if (selectedCollection === 'new') {
      if (newCollectionName.trim()) {
        const newCollection = {
          id: crypto.randomUUID(),
          name: newCollectionName,
          requests: [],
          folders: [],
          variables: [],
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        };
        addCollection(newCollection);
        onSave(newCollection.id, selectedFolder || undefined);
      }
    } else if (selectedCollection) {
      onSave(selectedCollection, selectedFolder || undefined);
    }
  };

  const selectedCol = collections.find(c => c.id === selectedCollection);

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg p-6 w-96 max-w-full">
        <h3 className="text-lg font-semibold mb-4">Save Request</h3>
        <p className="text-sm text-gray-600 mb-4">
          Saving: <span className="font-medium">{requestName}</span>
        </p>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Collection</label>
            <select
              value={selectedCollection}
              onChange={(e) => {
                setSelectedCollection(e.target.value);
                setSelectedFolder('');
              }}
              className="w-full px-3 py-2 border rounded-md"
            >
              <option value="">Select a collection...</option>
              <option value="new">+ Create New Collection</option>
              {collections.map((collection) => (
                <option key={collection.id} value={collection.id}>
                  {collection.name}
                </option>
              ))}
            </select>
          </div>

          {selectedCollection === 'new' && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                New Collection Name
              </label>
              <input
                type="text"
                value={newCollectionName}
                onChange={(e) => setNewCollectionName(e.target.value)}
                placeholder="My Collection"
                className="w-full px-3 py-2 border rounded-md"
                autoFocus
              />
            </div>
          )}

          {selectedCollection && selectedCollection !== 'new' && selectedCol && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Folder (optional)
              </label>
              <select
                value={selectedFolder}
                onChange={(e) => setSelectedFolder(e.target.value)}
                className="w-full px-3 py-2 border rounded-md"
              >
                <option value="">Root</option>
                {selectedCol.folders.map((folder) => (
                  <option key={folder.id} value={folder.id}>
                    {folder.name}
                  </option>
                ))}
              </select>
            </div>
          )}
        </div>

        <div className="flex justify-end gap-2 mt-6">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm border rounded-md hover:bg-gray-100"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={!selectedCollection || (selectedCollection === 'new' && !newCollectionName.trim())}
            className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
};

export default ApiTester;
