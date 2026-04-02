import React, { useState } from 'react';
import {
  Send,
  Save,
  Code,
  Play,
  Plus,
  Trash2,
  ChevronDown,
  Lock,
  Unlock,
} from 'lucide-react';
import { useApiTesterStore, HttpMethod, KeyValue, Auth, Body, ApiResponse, TestResult } from '../stores/apiTesterStore';
// TODO: Replace with native API calls
// import { executeRequest } from '../utils/tauriCommands';

// Stub for executeRequest until native API is implemented
const executeRequest = async (_request: any, _environmentId?: string): Promise<{ response: ApiResponse; testResults: TestResult[] }> => {
  console.log('executeRequest not implemented');
  return Promise.resolve({
    response: {
      id: 'stub',
      status: 200,
      statusText: 'OK',
      headers: {},
      body: '{}',
      size: 0,
      time: 0,
      timestamp: new Date(),
    },
    testResults: [],
  });
};

const methods: HttpMethod[] = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'];

const methodColors: Record<HttpMethod, string> = {
  GET: 'bg-green-100 text-green-700 border-green-300',
  POST: 'bg-blue-100 text-blue-700 border-blue-300',
  PUT: 'bg-yellow-100 text-yellow-700 border-yellow-300',
  DELETE: 'bg-red-100 text-red-700 border-red-300',
  PATCH: 'bg-purple-100 text-purple-700 border-purple-300',
  HEAD: 'bg-gray-100 text-gray-700 border-gray-300',
  OPTIONS: 'bg-gray-100 text-gray-700 border-gray-300',
};

interface RequestBuilderProps {
  onSend: () => void;
  onSave: () => void;
}

export const RequestBuilder: React.FC<RequestBuilderProps> = ({ onSend, onSave }) => {
  const [activeTab, setActiveTab] = useState<'params' | 'headers' | 'body' | 'auth' | 'tests'>('params');
  const [isMethodDropdownOpen, setIsMethodDropdownOpen] = useState(false);

  const {
    currentRequest,
    setMethod,
    setUrl,
    setQueryParams,
    setHeaders,
    setBody,
    setAuth,
    setTestScript,
    setCurrentRequest,
    isLoading,
  } = useApiTesterStore();

  const handleSend = async () => {
    const { setCurrentResponse, setTestResults } = useApiTesterStore.getState();
    setCurrentRequest({ ...currentRequest });

    try {
      const { response, testResults } = await executeRequest(currentRequest);
      setCurrentResponse(response);
      setTestResults(testResults);
      onSend();
    } catch (error) {
      console.error('Request failed:', error);
    }
  };

  const addKeyValue = (type: 'params' | 'headers') => {
    const newKv: KeyValue = { key: '', value: '', enabled: true };
    if (type === 'params') {
      setQueryParams([...currentRequest.queryParams, newKv]);
    } else {
      setHeaders([...currentRequest.headers, newKv]);
    }
  };

  const updateKeyValue = (type: 'params' | 'headers', index: number, field: keyof KeyValue, value: any) => {
    const items = type === 'params' ? [...currentRequest.queryParams] : [...currentRequest.headers];
    items[index] = { ...items[index], [field]: value };
    if (type === 'params') {
      setQueryParams(items);
    } else {
      setHeaders(items);
    }
  };

  const removeKeyValue = (type: 'params' | 'headers', index: number) => {
    const items = type === 'params' ? [...currentRequest.queryParams] : [...currentRequest.headers];
    items.splice(index, 1);
    if (type === 'params') {
      setQueryParams(items);
    } else {
      setHeaders(items);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* URL Bar */}
      <div className="flex items-center gap-2 p-4 border-b">
        {/* Method Dropdown */}
        <div className="relative">
          <button
            onClick={() => setIsMethodDropdownOpen(!isMethodDropdownOpen)}
            className={`flex items-center gap-2 px-3 py-2 rounded-md border font-semibold ${methodColors[currentRequest.method]}`}
          >
            {currentRequest.method}
            <ChevronDown className="w-4 h-4" />
          </button>
          {isMethodDropdownOpen && (
            <div className="absolute top-full left-0 mt-1 bg-white border rounded-md shadow-lg z-10 min-w-[100px]">
              {methods.map((method) => (
                <button
                  key={method}
                  onClick={() => {
                    setMethod(method);
                    setIsMethodDropdownOpen(false);
                  }}
                  className={`w-full px-3 py-2 text-left hover:bg-gray-100 first:rounded-t-md last:rounded-b-md ${methodColors[method]}`}
                >
                  {method}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* URL Input */}
        <input
          type="text"
          placeholder="Enter URL or paste cURL command"
          value={currentRequest.url}
          onChange={(e) => setUrl(e.target.value)}
          className="flex-1 px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />

        {/* Send Button */}
        <button
          onClick={handleSend}
          disabled={isLoading}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          <Send className="w-4 h-4" />
          {isLoading ? 'Sending...' : 'Send'}
        </button>

        {/* Save Button */}
        <button
          onClick={onSave}
          className="flex items-center gap-2 px-3 py-2 border rounded-md hover:bg-gray-100"
        >
          <Save className="w-4 h-4" />
          Save
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        {(['params', 'headers', 'body', 'auth', 'tests'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab
                ? 'border-blue-600 text-blue-600'
                : 'border-transparent text-gray-600 hover:text-gray-800'
            }`}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
            {tab === 'params' && currentRequest.queryParams.length > 0 && (
              <span className="ml-1 text-xs bg-gray-200 px-1.5 py-0.5 rounded-full">
                {currentRequest.queryParams.filter((p) => p.enabled).length}
              </span>
            )}
            {tab === 'headers' && currentRequest.headers.length > 0 && (
              <span className="ml-1 text-xs bg-gray-200 px-1.5 py-0.5 rounded-full">
                {currentRequest.headers.filter((h) => h.enabled).length}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-auto p-4">
        {/* Params Tab */}
        {activeTab === 'params' && (
          <div className="space-y-2">
            {currentRequest.queryParams.map((param, index) => (
              <div key={index} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={param.enabled}
                  onChange={(e) => updateKeyValue('params', index, 'enabled', e.target.checked)}
                  className="w-4 h-4"
                />
                <input
                  type="text"
                  placeholder="Key"
                  value={param.key}
                  onChange={(e) => updateKeyValue('params', index, 'key', e.target.value)}
                  className="flex-1 px-3 py-2 border rounded-md text-sm"
                />
                <input
                  type="text"
                  placeholder="Value"
                  value={param.value}
                  onChange={(e) => updateKeyValue('params', index, 'value', e.target.value)}
                  className="flex-1 px-3 py-2 border rounded-md text-sm"
                />
                <button
                  onClick={() => removeKeyValue('params', index)}
                  className="p-2 text-red-500 hover:bg-red-50 rounded-md"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            ))}
            <button
              onClick={() => addKeyValue('params')}
              className="flex items-center gap-2 px-4 py-2 text-sm text-blue-600 hover:bg-blue-50 rounded-md"
            >
              <Plus className="w-4 h-4" />
              Add Parameter
            </button>
          </div>
        )}

        {/* Headers Tab */}
        {activeTab === 'headers' && (
          <div className="space-y-2">
            {currentRequest.headers.map((header, index) => (
              <div key={index} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={header.enabled}
                  onChange={(e) => updateKeyValue('headers', index, 'enabled', e.target.checked)}
                  className="w-4 h-4"
                />
                <input
                  type="text"
                  placeholder="Header"
                  value={header.key}
                  onChange={(e) => updateKeyValue('headers', index, 'key', e.target.value)}
                  className="flex-1 px-3 py-2 border rounded-md text-sm"
                />
                <input
                  type="text"
                  placeholder="Value"
                  value={header.value}
                  onChange={(e) => updateKeyValue('headers', index, 'value', e.target.value)}
                  className="flex-1 px-3 py-2 border rounded-md text-sm"
                />
                <button
                  onClick={() => removeKeyValue('headers', index)}
                  className="p-2 text-red-500 hover:bg-red-50 rounded-md"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            ))}
            <button
              onClick={() => addKeyValue('headers')}
              className="flex items-center gap-2 px-4 py-2 text-sm text-blue-600 hover:bg-blue-50 rounded-md"
            >
              <Plus className="w-4 h-4" />
              Add Header
            </button>
          </div>
        )}

        {/* Body Tab */}
        {activeTab === 'body' && (
          <div className="space-y-4">
            <div className="flex gap-2">
              {(['none', 'text', 'json', 'xml', 'form'] as const).map((type) => (
                <button
                  key={type}
                  onClick={() => setBody({ type, content: currentRequest.body.content })}
                  className={`px-3 py-1.5 text-sm rounded-md ${
                    currentRequest.body.type === type
                      ? 'bg-blue-100 text-blue-700'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {type.charAt(0).toUpperCase() + type.slice(1)}
                </button>
              ))}
            </div>

            {currentRequest.body.type !== 'none' && currentRequest.body.type !== 'form' && (
              <textarea
                value={currentRequest.body.content || ''}
                onChange={(e) => setBody({ type: currentRequest.body.type, content: e.target.value })}
                placeholder={`Enter ${currentRequest.body.type.toUpperCase()} body...`}
                className="w-full h-64 px-3 py-2 border rounded-md font-mono text-sm resize-none"
                spellCheck={false}
              />
            )}

            {currentRequest.body.type === 'form' && (
              <div className="space-y-2">
                {(currentRequest.body.formData || []).map((field, index) => (
                  <div key={index} className="flex items-center gap-2">
                    <input
                      type="text"
                      placeholder="Key"
                      value={field.key}
                      onChange={(e) => {
                        const newFormData = [...(currentRequest.body.formData || [])];
                        newFormData[index] = { ...field, key: e.target.value };
                        setBody({ type: 'form', formData: newFormData });
                      }}
                      className="flex-1 px-3 py-2 border rounded-md text-sm"
                    />
                    <input
                      type="text"
                      placeholder="Value"
                      value={field.value}
                      onChange={(e) => {
                        const newFormData = [...(currentRequest.body.formData || [])];
                        newFormData[index] = { ...field, value: e.target.value };
                        setBody({ type: 'form', formData: newFormData });
                      }}
                      className="flex-1 px-3 py-2 border rounded-md text-sm"
                    />
                    <button
                      onClick={() => {
                        const newFormData = [...(currentRequest.body.formData || [])];
                        newFormData.splice(index, 1);
                        setBody({ type: 'form', formData: newFormData });
                      }}
                      className="p-2 text-red-500 hover:bg-red-50 rounded-md"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                ))}
                <button
                  onClick={() => {
                    const newFormData = [...(currentRequest.body.formData || []), { key: '', value: '', enabled: true }];
                    setBody({ type: 'form', formData: newFormData });
                  }}
                  className="flex items-center gap-2 px-4 py-2 text-sm text-blue-600 hover:bg-blue-50 rounded-md"
                >
                  <Plus className="w-4 h-4" />
                  Add Field
                </button>
              </div>
            )}
          </div>
        )}

        {/* Auth Tab */}
        {activeTab === 'auth' && (
          <div className="space-y-4">
            <div className="flex gap-2">
              {(['none', 'basic', 'bearer', 'apikey'] as const).map((type) => (
                <button
                  key={type}
                  onClick={() => setAuth({ type })}
                  className={`px-3 py-1.5 text-sm rounded-md ${
                    currentRequest.auth.type === type
                      ? 'bg-blue-100 text-blue-700'
                      : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                  }`}
                >
                  {type === 'apikey' ? 'API Key' : type.charAt(0).toUpperCase() + type.slice(1)}
                </button>
              ))}
            </div>

            {currentRequest.auth.type === 'basic' && (
              <div className="space-y-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Username</label>
                  <input
                    type="text"
                    value={currentRequest.auth.username || ''}
                    onChange={(e) => setAuth({ ...currentRequest.auth, type: 'basic', username: e.target.value })}
                    className="w-full px-3 py-2 border rounded-md"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Password</label>
                  <input
                    type="password"
                    value={currentRequest.auth.password || ''}
                    onChange={(e) => setAuth({ ...currentRequest.auth, type: 'basic', password: e.target.value })}
                    className="w-full px-3 py-2 border rounded-md"
                  />
                </div>
              </div>
            )}

            {currentRequest.auth.type === 'bearer' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">Token</label>
                <input
                  type="text"
                  value={currentRequest.auth.token || ''}
                  onChange={(e) => setAuth({ ...currentRequest.auth, type: 'bearer', token: e.target.value })}
                  className="w-full px-3 py-2 border rounded-md"
                  placeholder="Bearer token"
                />
              </div>
            )}

            {currentRequest.auth.type === 'apikey' && (
              <div className="space-y-3">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Key</label>
                  <input
                    type="text"
                    value={currentRequest.auth.apiKey || ''}
                    onChange={(e) => setAuth({ ...currentRequest.auth, type: 'apikey', apiKey: e.target.value })}
                    className="w-full px-3 py-2 border rounded-md"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Value</label>
                  <input
                    type="text"
                    value={currentRequest.auth.apiValue || ''}
                    onChange={(e) => setAuth({ ...currentRequest.auth, type: 'apikey', apiValue: e.target.value })}
                    className="w-full px-3 py-2 border rounded-md"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">Add to</label>
                  <div className="flex gap-2">
                    <button
                      onClick={() => setAuth({ ...currentRequest.auth, type: 'apikey', apiKeyIn: 'header' })}
                      className={`flex-1 px-3 py-2 text-sm rounded-md ${
                        currentRequest.auth.apiKeyIn === 'header' || !currentRequest.auth.apiKeyIn
                          ? 'bg-blue-100 text-blue-700'
                          : 'bg-gray-100 text-gray-700'
                      }`}
                    >
                      Header
                    </button>
                    <button
                      onClick={() => setAuth({ ...currentRequest.auth, type: 'apikey', apiKeyIn: 'query' })}
                      className={`flex-1 px-3 py-2 text-sm rounded-md ${
                        currentRequest.auth.apiKeyIn === 'query'
                          ? 'bg-blue-100 text-blue-700'
                          : 'bg-gray-100 text-gray-700'
                      }`}
                    >
                      Query Params
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {/* Tests Tab */}
        {activeTab === 'tests' && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-sm font-medium text-gray-700">Test Script</label>
              <button
                onClick={() => setTestScript(`// Test script\npm.test("Status code is 200", function () {\n  pm.response.to.have.status(200);\n});\n`)}
                className="text-xs text-blue-600 hover:underline"
              >
                Insert Template
              </button>
            </div>
            <textarea
              value={currentRequest.testScript || ''}
              onChange={(e) => setTestScript(e.target.value)}
              placeholder="Enter test script (Postman syntax supported)..."
              className="w-full h-64 px-3 py-2 border rounded-md font-mono text-sm resize-none"
              spellCheck={false}
            />
            <p className="text-xs text-gray-500">
              Supports Postman test syntax: pm.test(), pm.expect(), assert statements
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

export default RequestBuilder;
