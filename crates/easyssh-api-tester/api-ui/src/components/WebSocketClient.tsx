import React, { useState, useRef, useEffect } from 'react';
import { Play, Square, Send, Trash2, Download, Copy, Check } from 'lucide-react';
import { useApiTesterStore, WebSocketMessage } from '../stores/apiTesterStore';
// TODO: Replace with native API calls
// import { wsConnect, wsSend, wsGetMessages, wsDisconnect, wsIsConnected } from '../utils/tauriCommands';

// Stubs for WebSocket commands until native API is implemented
const wsConnect = async (_id: string, _url: string, _headers?: any) => {
  console.log('wsConnect not implemented');
  return Promise.resolve();
};
const wsSend = async (_id: string, _message: string) => {
  console.log('wsSend not implemented');
  return Promise.resolve();
};
const wsGetMessages = async (_id: string): Promise<WebSocketMessage[]> => {
  console.log('wsGetMessages not implemented');
  return Promise.resolve([]);
};
const wsDisconnect = async (_id: string) => {
  console.log('wsDisconnect not implemented');
  return Promise.resolve();
};
const wsIsConnected = async (_id: string): Promise<boolean> => {
  console.log('wsIsConnected not implemented');
  return Promise.resolve(false);
};

export const WebSocketClient: React.FC = () => {
  const [url, setUrl] = useState('wss://echo.websocket.org');
  const [message, setMessage] = useState('');
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedFormat, setSelectedFormat] = useState<'text' | 'json'>('text');
  const [copied, setCopied] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const {
    wsMessages,
    addWsMessage,
    clearWsMessages,
  } = useApiTesterStore();

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [wsMessages]);

  const handleConnect = async () => {
    if (!url.trim()) return;

    setConnecting(true);
    setError(null);

    try {
      await wsConnect('main', url, []);
      setConnected(true);
      addWsMessage({
        timestamp: new Date().toISOString(),
        direction: 'sent',
        content: `Connected to ${url}`,
        type: 'system',
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect');
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      await wsDisconnect('main');
      setConnected(false);
      addWsMessage({
        timestamp: new Date().toISOString(),
        direction: 'sent',
        content: 'Disconnected',
        type: 'system',
      });
    } catch (err) {
      console.error('Disconnect error:', err);
    }
  };

  const handleSend = async () => {
    if (!message.trim() || !connected) return;

    try {
      await wsSend('main', message);
      addWsMessage({
        timestamp: new Date().toISOString(),
        direction: 'sent',
        content: message,
        type: selectedFormat,
      });
      setMessage('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send message');
    }
  };

  const handleClear = () => {
    clearWsMessages();
  };

  const handleCopy = () => {
    const text = wsMessages.map(m => `[${m.direction}] ${m.content}`).join('\n');
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const formatContent = (content: string, type: string) => {
    if (type === 'json') {
      try {
        const parsed = JSON.parse(content);
        return JSON.stringify(parsed, null, 2);
      } catch {
        return content;
      }
    }
    return content;
  };

  return (
    <div className="h-full flex flex-col">
      {/* URL Bar */}
      <div className="flex items-center gap-2 p-4 border-b">
        <input
          type="text"
          placeholder="wss://echo.websocket.org"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          disabled={connected || connecting}
          className="flex-1 px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:bg-gray-100"
        />
        {connected ? (
          <button
            onClick={handleDisconnect}
            className="flex items-center gap-2 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
          >
            <Square className="w-4 h-4" />
            Disconnect
          </button>
        ) : (
          <button
            onClick={handleConnect}
            disabled={connecting || !url.trim()}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
          >
            <Play className="w-4 h-4" />
            {connecting ? 'Connecting...' : 'Connect'}
          </button>
        )}
      </div>

      {error && (
        <div className="px-4 py-2 bg-red-50 text-red-700 text-sm">
          {error}
        </div>
      )}

      {/* Messages Area */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <div className="flex items-center justify-between px-4 py-2 border-b bg-gray-50">
          <span className="text-sm font-medium">Messages ({wsMessages.length})</span>
          <div className="flex gap-1">
            <button
              onClick={handleCopy}
              className="flex items-center gap-1 px-2 py-1 text-sm bg-white border rounded-md hover:bg-gray-100"
            >
              {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
              {copied ? 'Copied!' : 'Copy'}
            </button>
            <button
              onClick={handleClear}
              className="flex items-center gap-1 px-2 py-1 text-sm bg-white border rounded-md hover:bg-gray-100"
            >
              <Trash2 className="w-4 h-4" />
              Clear
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-auto p-4 space-y-2">
          {wsMessages.length === 0 ? (
            <div className="text-center text-gray-500 py-8">
              <p>Connect to a WebSocket server to start messaging</p>
              <p className="text-sm mt-1">Try wss://echo.websocket.org for testing</p>
            </div>
          ) : (
            wsMessages.map((msg, index) => (
              <div
                key={index}
                className={`flex ${msg.direction === 'sent' ? 'justify-end' : 'justify-start'}`}
              >
                <div
                  className={`max-w-[80%] rounded-lg p-3 ${
                    msg.type === 'system'
                      ? 'bg-yellow-50 text-yellow-800'
                      : msg.direction === 'sent'
                      ? 'bg-blue-100 text-blue-900'
                      : 'bg-gray-100 text-gray-900'
                  }`}
                >
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-xs font-medium">
                      {msg.direction === 'sent' ? 'Sent' : msg.direction === 'received' ? 'Received' : 'System'}
                    </span>
                    <span className="text-xs text-gray-500">
                      {new Date(msg.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  <pre className="text-sm whitespace-pre-wrap font-mono">
                    {formatContent(msg.content, msg.type)}
                  </pre>
                </div>
              </div>
            ))
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Message Input */}
        <div className="border-t p-4">
          <div className="flex gap-2">
            <div className="flex gap-1">
              <button
                onClick={() => setSelectedFormat('text')}
                className={`px-3 py-2 text-sm rounded-md ${
                  selectedFormat === 'text'
                    ? 'bg-blue-100 text-blue-700'
                    : 'bg-gray-100 text-gray-700'
                }`}
              >
                Text
              </button>
              <button
                onClick={() => setSelectedFormat('json')}
                className={`px-3 py-2 text-sm rounded-md ${
                  selectedFormat === 'json'
                    ? 'bg-blue-100 text-blue-700'
                    : 'bg-gray-100 text-gray-700'
                }`}
              >
                JSON
              </button>
            </div>
            <input
              type="text"
              placeholder="Enter message..."
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSend()}
              disabled={!connected}
              className="flex-1 px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:bg-gray-100"
            />
            <button
              onClick={handleSend}
              disabled={!connected || !message.trim()}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
            >
              <Send className="w-4 h-4" />
              Send
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default WebSocketClient;
