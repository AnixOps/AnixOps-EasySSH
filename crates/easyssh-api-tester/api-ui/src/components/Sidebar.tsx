import React, { useState } from 'react';
import {
  Play,
  Plus,
  Save,
  Folder,
  ChevronDown,
  ChevronRight,
  MoreVertical,
  Search,
  Settings,
  Trash2,
  Copy,
  Edit,
  FolderPlus,
  FileJson,
  Download,
  Upload,
} from 'lucide-react';
import { useApiTesterStore, Collection, CollectionFolder, ApiRequest, Environment } from '../stores/apiTesterStore';
// TODO: Replace with native API calls
// import * as tauriCommands from '../utils/tauriCommands';

// Stubs for tauriCommands until native API is implemented
const tauriCommands = {
  exportPostmanCollection: async (_collection: Collection): Promise<string> => {
    console.log('exportPostmanCollection not implemented');
    return '';
  },
  importPostmanCollection: async (_data: string): Promise<Collection> => {
    console.log('importPostmanCollection not implemented');
    throw new Error('Not implemented');
  },
  exportCurlCommand: async (_request: ApiRequest): Promise<string> => {
    console.log('exportCurlCommand not implemented');
    return '';
  },
};

interface SidebarProps {
  onSelectRequest: (request: ApiRequest) => void;
  onNewRequest: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ onSelectRequest, onNewRequest }) => {
  const [activeTab, setActiveTab] = useState<'collections' | 'environments' | 'history'>('collections');
  const [expandedCollections, setExpandedCollections] = useState<Set<string>>(new Set());
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const [showNewCollectionModal, setShowNewCollectionModal] = useState(false);
  const [newCollectionName, setNewCollectionName] = useState('');
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; item: any; type: string } | null>(null);

  const {
    collections,
    environments,
    history,
    addCollection,
    deleteCollection,
    addEnvironment,
    deleteEnvironment,
    setActiveEnvironment,
    activeEnvironmentId,
    deleteHistoryEntry,
  } = useApiTesterStore();

  const toggleCollection = (id: string) => {
    const newExpanded = new Set(expandedCollections);
    if (newExpanded.has(id)) {
      newExpanded.delete(id);
    } else {
      newExpanded.add(id);
    }
    setExpandedCollections(newExpanded);
  };

  const toggleFolder = (id: string) => {
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(id)) {
      newExpanded.delete(id);
    } else {
      newExpanded.add(id);
    }
    setExpandedFolders(newExpanded);
  };

  const handleCreateCollection = () => {
    if (newCollectionName.trim()) {
      const newCollection: Collection = {
        id: crypto.randomUUID(),
        name: newCollectionName,
        requests: [],
        folders: [],
        variables: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      addCollection(newCollection);
      setNewCollectionName('');
      setShowNewCollectionModal(false);
    }
  };

  const handleCreateEnvironment = () => {
    const newEnv: Environment = {
      id: crypto.randomUUID(),
      name: 'New Environment',
      variables: [],
      isDefault: environments.length === 0,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
    addEnvironment(newEnv);
  };

  const handleExportCollection = async (collection: Collection) => {
    try {
      const json = await tauriCommands.exportPostmanCollection(collection);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${collection.name}.postman_collection.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Export failed:', error);
    }
  };

  const handleImportCollection = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const text = await file.text();
      const collection = await tauriCommands.importPostmanCollection(text);
      addCollection(collection);
    } catch (error) {
      console.error('Import failed:', error);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, item: any, type: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, item, type });
  };

  const closeContextMenu = () => setContextMenu(null);

  const filteredCollections = collections.filter(c =>
    c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    c.requests.some(r => r.name.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  const filteredHistory = history.filter(h =>
    h.request.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    h.request.url.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const getMethodColor = (method: string) => {
    switch (method.toUpperCase()) {
      case 'GET': return 'text-green-600';
      case 'POST': return 'text-blue-600';
      case 'PUT': return 'text-yellow-600';
      case 'DELETE': return 'text-red-600';
      case 'PATCH': return 'text-purple-600';
      default: return 'text-gray-600';
    }
  };

  return (
    <div className="h-full flex flex-col bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800">
      {/* Tabs */}
      <div className="flex border-b border-gray-200 dark:border-gray-800">
        <button
          onClick={() => setActiveTab('collections')}
          className={`flex-1 py-2 px-3 text-sm font-medium ${
            activeTab === 'collections'
              ? 'text-blue-600 border-b-2 border-blue-600'
              : 'text-gray-600 hover:text-gray-800'
          }`}
        >
          Collections
        </button>
        <button
          onClick={() => setActiveTab('environments')}
          className={`flex-1 py-2 px-3 text-sm font-medium ${
            activeTab === 'environments'
              ? 'text-blue-600 border-b-2 border-blue-600'
              : 'text-gray-600 hover:text-gray-800'
          }`}
        >
          Envs
        </button>
        <button
          onClick={() => setActiveTab('history')}
          className={`flex-1 py-2 px-3 text-sm font-medium ${
            activeTab === 'history'
              ? 'text-blue-600 border-b-2 border-blue-600'
              : 'text-gray-600 hover:text-gray-800'
          }`}
        >
          History
        </button>
      </div>

      {/* Search and Actions */}
      <div className="p-2 space-y-2">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input
            type="text"
            placeholder="Search..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 text-sm border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
        {activeTab === 'collections' && (
          <div className="flex gap-1">
            <button
              onClick={onNewRequest}
              className="flex-1 flex items-center justify-center gap-1 px-2 py-1.5 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
            >
              <Plus className="w-4 h-4" />
              New
            </button>
            <button
              onClick={() => setShowNewCollectionModal(true)}
              className="flex items-center justify-center px-2 py-1.5 text-sm border rounded-md hover:bg-gray-100"
            >
              <FolderPlus className="w-4 h-4" />
            </button>
            <label className="flex items-center justify-center px-2 py-1.5 text-sm border rounded-md hover:bg-gray-100 cursor-pointer">
              <Upload className="w-4 h-4" />
              <input
                type="file"
                accept=".json"
                onChange={handleImportCollection}
                className="hidden"
              />
            </label>
          </div>
        )}
        {activeTab === 'environments' && (
          <button
            onClick={handleCreateEnvironment}
            className="w-full flex items-center justify-center gap-1 px-2 py-1.5 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
          >
            <Plus className="w-4 h-4" />
            New Environment
          </button>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'collections' && (
          <div className="space-y-1 px-2">
            {filteredCollections.map((collection) => (
              <div key={collection.id}>
                <div
                  className="flex items-center gap-1 py-1 px-2 hover:bg-gray-100 rounded cursor-pointer"
                  onClick={() => toggleCollection(collection.id)}
                  onContextMenu={(e) => handleContextMenu(e, collection, 'collection')}
                >
                  {expandedCollections.has(collection.id) ? (
                    <ChevronDown className="w-4 h-4" />
                  ) : (
                    <ChevronRight className="w-4 h-4" />
                  )}
                  <Folder className="w-4 h-4 text-yellow-500" />
                  <span className="text-sm truncate">{collection.name}</span>
                </div>
                {expandedCollections.has(collection.id) && (
                  <div className="ml-4 space-y-1">
                    {collection.requests.map((request) => (
                      <div
                        key={request.id}
                        className="flex items-center gap-1 py-1 px-2 hover:bg-gray-100 rounded cursor-pointer"
                        onClick={() => onSelectRequest(request)}
                        onContextMenu={(e) => handleContextMenu(e, request, 'request')}
                      >
                        <span className={`text-xs font-semibold w-12 ${getMethodColor(request.method)}`}>
                          {request.method}
                        </span>
                        <span className="text-sm truncate">{request.name}</span>
                      </div>
                    ))}
                    {collection.folders.map((folder) => (
                      <FolderTree
                        key={folder.id}
                        folder={folder}
                        onSelectRequest={onSelectRequest}
                        onContextMenu={handleContextMenu}
                        expandedFolders={expandedFolders}
                        toggleFolder={toggleFolder}
                        getMethodColor={getMethodColor}
                      />
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {activeTab === 'environments' && (
          <div className="space-y-1 px-2">
            {environments.map((env) => (
              <div
                key={env.id}
                className={`flex items-center gap-2 py-1 px-2 rounded cursor-pointer ${
                  activeEnvironmentId === env.id ? 'bg-blue-100 text-blue-700' : 'hover:bg-gray-100'
                }`}
                onClick={() => setActiveEnvironment(env.id)}
                onContextMenu={(e) => handleContextMenu(e, env, 'environment')}
              >
                <Settings className="w-4 h-4" />
                <span className="text-sm truncate">{env.name}</span>
                {env.isDefault && (
                  <span className="text-xs bg-gray-200 px-1.5 py-0.5 rounded">Default</span>
                )}
              </div>
            ))}
          </div>
        )}

        {activeTab === 'history' && (
          <div className="space-y-1 px-2">
            {filteredHistory.map((entry) => (
              <div
                key={entry.id}
                className="flex items-center gap-2 py-1 px-2 hover:bg-gray-100 rounded cursor-pointer"
                onClick={() => onSelectRequest(entry.request)}
                onContextMenu={(e) => handleContextMenu(e, entry, 'history')}
              >
                <span className={`text-xs font-semibold w-12 ${getMethodColor(entry.request.method)}`}>
                  {entry.request.method}
                </span>
                <div className="flex-1 min-w-0">
                  <div className="text-sm truncate">{entry.request.name}</div>
                  <div className="text-xs text-gray-500 truncate">{entry.request.url}</div>
                </div>
                <span
                  className={`text-xs ${
                    entry.response.status >= 200 && entry.response.status < 300
                      ? 'text-green-600'
                      : entry.response.status >= 400
                      ? 'text-red-600'
                      : 'text-yellow-600'
                  }`}
                >
                  {entry.response.status}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* New Collection Modal */}
      {showNewCollectionModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-4 w-80">
            <h3 className="text-lg font-semibold mb-3">New Collection</h3>
            <input
              type="text"
              placeholder="Collection Name"
              value={newCollectionName}
              onChange={(e) => setNewCollectionName(e.target.value)}
              className="w-full px-3 py-2 border rounded-md mb-3"
              autoFocus
              onKeyDown={(e) => e.key === 'Enter' && handleCreateCollection()}
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowNewCollectionModal(false)}
                className="px-3 py-1.5 text-sm border rounded-md hover:bg-gray-100"
              >
                Cancel
              </button>
              <button
                onClick={handleCreateCollection}
                className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700"
              >
                Create
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Context Menu */}
      {contextMenu && (
        <>
          <div className="fixed inset-0" onClick={closeContextMenu} />
          <div
            className="fixed bg-white border rounded-md shadow-lg py-1 z-50 min-w-[160px]"
            style={{ left: contextMenu.x, top: contextMenu.y }}
          >
            {contextMenu.type === 'collection' && (
              <>
                <button
                  onClick={() => {
                    handleExportCollection(contextMenu.item);
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100"
                >
                  <Download className="w-4 h-4" />
                  Export
                </button>
                <button
                  onClick={() => {
                    deleteCollection(contextMenu.item.id);
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm text-red-600 hover:bg-gray-100"
                >
                  <Trash2 className="w-4 h-4" />
                  Delete
                </button>
              </>
            )}
            {contextMenu.type === 'request' && (
              <>
                <button
                  onClick={() => {
                    onSelectRequest(contextMenu.item);
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100"
                >
                  <Edit className="w-4 h-4" />
                  Edit
                </button>
                <button
                  onClick={() => {
                    // Duplicate request
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100"
                >
                  <Copy className="w-4 h-4" />
                  Duplicate
                </button>
                <button
                  onClick={() => {
                    tauriCommands.exportCurlCommand(contextMenu.item).then(cmd => {
                      navigator.clipboard.writeText(cmd);
                    });
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100"
                >
                  <FileJson className="w-4 h-4" />
                  Copy as cURL
                </button>
              </>
            )}
            {contextMenu.type === 'history' && (
              <>
                <button
                  onClick={() => {
                    onSelectRequest(contextMenu.item.request);
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm hover:bg-gray-100"
                >
                  <Play className="w-4 h-4" />
                  Replay
                </button>
                <button
                  onClick={() => {
                    deleteHistoryEntry(contextMenu.item.id);
                    closeContextMenu();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm text-red-600 hover:bg-gray-100"
                >
                  <Trash2 className="w-4 h-4" />
                  Delete
                </button>
              </>
            )}
            {contextMenu.type === 'environment' && (
              <button
                onClick={() => {
                  deleteEnvironment(contextMenu.item.id);
                  closeContextMenu();
                }}
                className="w-full flex items-center gap-2 px-3 py-2 text-sm text-red-600 hover:bg-gray-100"
              >
                <Trash2 className="w-4 h-4" />
                Delete
              </button>
            )}
          </div>
        </>
      )}
    </div>
  );
};

interface FolderTreeProps {
  folder: CollectionFolder;
  onSelectRequest: (request: ApiRequest) => void;
  onContextMenu: (e: React.MouseEvent, item: any, type: string) => void;
  expandedFolders: Set<string>;
  toggleFolder: (id: string) => void;
  getMethodColor: (method: string) => string;
}

const FolderTree: React.FC<FolderTreeProps> = ({
  folder,
  onSelectRequest,
  onContextMenu,
  expandedFolders,
  toggleFolder,
  getMethodColor,
}) => {
  const isExpanded = expandedFolders.has(folder.id);

  return (
    <div>
      <div
        className="flex items-center gap-1 py-1 px-2 hover:bg-gray-100 rounded cursor-pointer"
        onClick={() => toggleFolder(folder.id)}
        onContextMenu={(e) => onContextMenu(e, folder, 'folder')}
      >
        {isExpanded ? (
          <ChevronDown className="w-4 h-4" />
        ) : (
          <ChevronRight className="w-4 h-4" />
        )}
        <Folder className="w-4 h-4 text-yellow-400" />
        <span className="text-sm truncate">{folder.name}</span>
      </div>
      {isExpanded && (
        <div className="ml-4 space-y-1">
          {folder.requests.map((request) => (
            <div
              key={request.id}
              className="flex items-center gap-1 py-1 px-2 hover:bg-gray-100 rounded cursor-pointer"
              onClick={() => onSelectRequest(request)}
              onContextMenu={(e) => onContextMenu(e, request, 'request')}
            >
              <span className={`text-xs font-semibold w-12 ${getMethodColor(request.method)}`}>
                {request.method}
              </span>
              <span className="text-sm truncate">{request.name}</span>
            </div>
          ))}
          {folder.folders.map((subFolder) => (
            <FolderTree
              key={subFolder.id}
              folder={subFolder}
              onSelectRequest={onSelectRequest}
              onContextMenu={onContextMenu}
              expandedFolders={expandedFolders}
              toggleFolder={toggleFolder}
              getMethodColor={getMethodColor}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export default Sidebar;
