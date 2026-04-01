import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';

export interface KeyValue {
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
}

export type AuthType = 'none' | 'basic' | 'bearer' | 'apikey' | 'oauth2';

export interface Auth {
  type: AuthType;
  username?: string;
  password?: string;
  token?: string;
  apiKey?: string;
  apiValue?: string;
  apiKeyIn?: 'header' | 'query';
  accessToken?: string;
  refreshToken?: string;
}

export type BodyType = 'none' | 'text' | 'json' | 'xml' | 'form' | 'multipart';

export interface Body {
  type: BodyType;
  content?: string;
  formData?: KeyValue[];
  multipartData?: MultipartPart[];
}

export interface MultipartPart {
  name: string;
  type: 'text' | 'file';
  content?: string;
  filename?: string;
  data?: Uint8Array;
}

export interface ApiRequest {
  id: string;
  name: string;
  method: HttpMethod;
  url: string;
  headers: KeyValue[];
  queryParams: KeyValue[];
  auth: Auth;
  body: Body;
  preRequestScript?: string;
  testScript?: string;
  createdAt: string;
  updatedAt: string;
}

export interface ApiResponse {
  status: number;
  statusText: string;
  timestamp: string;
  headers: Record<string, string>;
  body: string;
  bodyBase64?: string;
  contentType?: string;
  sizeBytes: number;
  timeMs: number;
}

export interface TestResult {
  name: string;
  passed: boolean;
  errorMessage?: string;
  durationMs: number;
}

export interface Collection {
  id: string;
  name: string;
  description?: string;
  requests: ApiRequest[];
  folders: CollectionFolder[];
  variables: EnvironmentVariable[];
  auth?: Auth;
  createdAt: string;
  updatedAt: string;
}

export interface CollectionFolder {
  id: string;
  name: string;
  description?: string;
  requests: ApiRequest[];
  folders: CollectionFolder[];
}

export interface EnvironmentVariable {
  key: string;
  value: string;
  enabled: boolean;
  description?: string;
}

export interface Environment {
  id: string;
  name: string;
  variables: EnvironmentVariable[];
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface HistoryEntry {
  id: string;
  request: ApiRequest;
  response: ApiResponse;
  environmentId?: string;
  collectionId?: string;
  timestamp: string;
}

export type TabType = 'params' | 'headers' | 'body' | 'auth' | 'pre-script' | 'tests';
export type ResponseTabType = 'body' | 'headers' | 'cookies' | 'test-results';
export type WebSocketDirection = 'sent' | 'received';

export interface WebSocketMessage {
  timestamp: string;
  direction: WebSocketDirection;
  content: string;
  type: string;
}

interface ApiTesterState {
  // Current request
  currentRequest: ApiRequest;
  currentResponse: ApiResponse | null;
  testResults: TestResult[];
  isLoading: boolean;
  error: string | null;

  // UI state
  activeTab: TabType;
  activeResponseTab: ResponseTabType;
  sidebarVisible: boolean;
  sidebarWidth: number;

  // Collections
  collections: Collection[];
  activeCollectionId: string | null;

  // Environments
  environments: Environment[];
  activeEnvironmentId: string | null;

  // History
  history: HistoryEntry[];

  // WebSocket
  wsConnected: boolean;
  wsMessages: WebSocketMessage[];
  wsUrl: string;

  // Actions
  setCurrentRequest: (request: Partial<ApiRequest>) => void;
  setMethod: (method: HttpMethod) => void;
  setUrl: (url: string) => void;
  setHeaders: (headers: KeyValue[]) => void;
  setQueryParams: (params: KeyValue[]) => void;
  setBody: (body: Body) => void;
  setAuth: (auth: Auth) => void;
  setPreRequestScript: (script: string) => void;
  setTestScript: (script: string) => void;
  setActiveTab: (tab: TabType) => void;
  setActiveResponseTab: (tab: ResponseTabType) => void;
  setSidebarVisible: (visible: boolean) => void;
  setSidebarWidth: (width: number) => void;

  // Request actions
  resetRequest: () => void;
  newRequest: () => void;
  saveRequestToCollection: (collectionId: string, folderId?: string) => void;

  // Collection actions
  addCollection: (collection: Collection) => void;
  updateCollection: (collection: Collection) => void;
  deleteCollection: (id: string) => void;
  setActiveCollection: (id: string | null) => void;

  // Environment actions
  addEnvironment: (env: Environment) => void;
  updateEnvironment: (env: Environment) => void;
  deleteEnvironment: (id: string) => void;
  setActiveEnvironment: (id: string | null) => void;

  // History actions
  addToHistory: (entry: HistoryEntry) => void;
  clearHistory: () => void;
  deleteHistoryEntry: (id: string) => void;

  // WebSocket actions
  setWsConnected: (connected: boolean) => void;
  addWsMessage: (message: WebSocketMessage) => void;
  clearWsMessages: () => void;
  setWsUrl: (url: string) => void;
}

const createDefaultRequest = (): ApiRequest => ({
  id: crypto.randomUUID(),
  name: 'New Request',
  method: 'GET',
  url: '',
  headers: [
    { key: 'Accept', value: '*/*', enabled: true },
  ],
  queryParams: [],
  auth: { type: 'none' },
  body: { type: 'none' },
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
});

export const useApiTesterStore = create<ApiTesterState>()(
  persist(
    (set, get) => ({
      // Initial state
      currentRequest: createDefaultRequest(),
      currentResponse: null,
      testResults: [],
      isLoading: false,
      error: null,

      activeTab: 'params',
      activeResponseTab: 'body',
      sidebarVisible: true,
      sidebarWidth: 280,

      collections: [],
      activeCollectionId: null,

      environments: [],
      activeEnvironmentId: null,

      history: [],

      wsConnected: false,
      wsMessages: [],
      wsUrl: '',

      // Actions
      setCurrentRequest: (request) => set((state) => ({
        currentRequest: { ...state.currentRequest, ...request, updatedAt: new Date().toISOString() },
      })),

      setMethod: (method) => set((state) => ({
        currentRequest: { ...state.currentRequest, method, updatedAt: new Date().toISOString() },
      })),

      setUrl: (url) => set((state) => ({
        currentRequest: { ...state.currentRequest, url, updatedAt: new Date().toISOString() },
      })),

      setHeaders: (headers) => set((state) => ({
        currentRequest: { ...state.currentRequest, headers, updatedAt: new Date().toISOString() },
      })),

      setQueryParams: (queryParams) => set((state) => ({
        currentRequest: { ...state.currentRequest, queryParams, updatedAt: new Date().toISOString() },
      })),

      setBody: (body) => set((state) => ({
        currentRequest: { ...state.currentRequest, body, updatedAt: new Date().toISOString() },
      })),

      setAuth: (auth) => set((state) => ({
        currentRequest: { ...state.currentRequest, auth, updatedAt: new Date().toISOString() },
      })),

      setPreRequestScript: (preRequestScript) => set((state) => ({
        currentRequest: { ...state.currentRequest, preRequestScript, updatedAt: new Date().toISOString() },
      })),

      setTestScript: (testScript) => set((state) => ({
        currentRequest: { ...state.currentRequest, testScript, updatedAt: new Date().toISOString() },
      })),

      setActiveTab: (activeTab) => set({ activeTab }),
      setActiveResponseTab: (activeResponseTab) => set({ activeResponseTab }),
      setSidebarVisible: (sidebarVisible) => set({ sidebarVisible }),
      setSidebarWidth: (sidebarWidth) => set({ sidebarWidth }),

      resetRequest: () => set({ currentRequest: createDefaultRequest(), currentResponse: null, testResults: [], error: null }),

      newRequest: () => set({
        currentRequest: createDefaultRequest(),
        currentResponse: null,
        testResults: [],
        error: null,
        activeTab: 'params',
      }),

      saveRequestToCollection: (collectionId, folderId) => {
        const { currentRequest, collections } = get();
        const collection = collections.find(c => c.id === collectionId);
        if (!collection) return;

        const updatedRequest = { ...currentRequest, updatedAt: new Date().toISOString() };

        if (folderId) {
          // Add to folder logic
          const folder = collection.folders.find(f => f.id === folderId);
          if (folder) {
            folder.requests.push(updatedRequest);
          }
        } else {
          collection.requests.push(updatedRequest);
        }

        collection.updatedAt = new Date().toISOString();

        set({
          collections: [...collections],
        });
      },

      addCollection: (collection) => set((state) => ({
        collections: [...state.collections, collection],
      })),

      updateCollection: (collection) => set((state) => ({
        collections: state.collections.map(c => c.id === collection.id ? collection : c),
      })),

      deleteCollection: (id) => set((state) => ({
        collections: state.collections.filter(c => c.id !== id),
        activeCollectionId: state.activeCollectionId === id ? null : state.activeCollectionId,
      })),

      setActiveCollection: (activeCollectionId) => set({ activeCollectionId }),

      addEnvironment: (env) => set((state) => ({
        environments: [...state.environments, env],
      })),

      updateEnvironment: (env) => set((state) => ({
        environments: state.environments.map(e => e.id === env.id ? env : e),
      })),

      deleteEnvironment: (id) => set((state) => ({
        environments: state.environments.filter(e => e.id !== id),
        activeEnvironmentId: state.activeEnvironmentId === id ? null : state.activeEnvironmentId,
      })),

      setActiveEnvironment: (activeEnvironmentId) => set({ activeEnvironmentId }),

      addToHistory: (entry) => set((state) => ({
        history: [entry, ...state.history.slice(0, 999)],
      })),

      clearHistory: () => set({ history: [] }),

      deleteHistoryEntry: (id) => set((state) => ({
        history: state.history.filter(h => h.id !== id),
      })),

      setWsConnected: (wsConnected) => set({ wsConnected }),
      addWsMessage: (message) => set((state) => ({
        wsMessages: [...state.wsMessages, message],
      })),
      clearWsMessages: () => set({ wsMessages: [] }),
      setWsUrl: (wsUrl) => set({ wsUrl }),
    }),
    {
      name: 'api-tester-storage',
      partialize: (state) => ({
        collections: state.collections,
        environments: state.environments,
        history: state.history.slice(0, 100),
        sidebarWidth: state.sidebarWidth,
        sidebarVisible: state.sidebarVisible,
      }),
    }
  )
);
