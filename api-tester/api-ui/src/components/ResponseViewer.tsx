import React, { useState } from 'react';
import { Check, X, Clock, FileJson, FileText, Table, Eye, Download, Copy, CheckCircle, XCircle } from 'lucide-react';
import { useApiTesterStore } from '../stores/apiTesterStore';

export const ResponseViewer: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'body' | 'headers' | 'cookies' | 'test-results'>('body');
  const [bodyFormat, setBodyFormat] = useState<'pretty' | 'raw' | 'preview'>('pretty');
  const [copied, setCopied] = useState(false);

  const { currentResponse, testResults } = useApiTesterStore();

  if (!currentResponse) {
    return (
      <div className="h-full flex items-center justify-center text-gray-500">
        <div className="text-center">
          <p className="text-lg mb-2">Send a request to see the response</p>
          <p className="text-sm">Use the URL bar above to make a request</p>
        </div>
      </div>
    );
  }

  const getStatusColor = (status: number) => {
    if (status >= 200 && status < 300) return 'text-green-600 bg-green-50';
    if (status >= 300 && status < 400) return 'text-yellow-600 bg-yellow-50';
    if (status >= 400 && status < 500) return 'text-orange-600 bg-orange-50';
    if (status >= 500) return 'text-red-600 bg-red-50';
    return 'text-gray-600 bg-gray-50';
  };

  const formatBody = (body: string, contentType?: string) => {
    if (bodyFormat === 'raw') {
      return body;
    }

    if (bodyFormat === 'pretty') {
      // Try to format as JSON
      if (contentType?.includes('json') || body.trim().startsWith('{') || body.trim().startsWith('[')) {
        try {
          const parsed = JSON.parse(body);
          return JSON.stringify(parsed, null, 2);
        } catch {
          return body;
        }
      }

      // Try to format as XML
      if (contentType?.includes('xml')) {
        // Basic XML formatting (could use a proper XML formatter)
        return body.replace(/></g, '>\n<').replace(/\n\s*\n/g, '\n');
      }
    }

    if (bodyFormat === 'preview') {
      if (contentType?.includes('image')) {
        return (
          <img
            src={`data:${contentType};base64,${currentResponse.bodyBase64 || btoa(body)}`}
            alt="Response"
            className="max-w-full max-h-96"
          />
        );
      }
      if (contentType?.includes('html')) {
        return (
          <iframe
            srcDoc={body}
            className="w-full h-96 border"
            sandbox="allow-scripts"
            title="Response Preview"
          />
        );
      }
    }

    return body;
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(currentResponse.body);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleDownload = () => {
    const blob = new Blob([currentResponse.body], { type: currentResponse.contentType || 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `response-${currentResponse.status}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const parsedCookies = Object.entries(currentResponse.headers)
    .filter(([key]) => key.toLowerCase() === 'set-cookie')
    .map(([_, value]) => value);

  return (
    <div className="flex flex-col h-full">
      {/* Status Bar */}
      <div className="flex items-center gap-4 px-4 py-2 border-b bg-gray-50">
        <span className={`px-3 py-1 rounded-full text-sm font-medium ${getStatusColor(currentResponse.status)}`}>
          {currentResponse.status} {currentResponse.statusText}
        </span>
        <span className="text-sm text-gray-600 flex items-center gap-1">
          <Clock className="w-4 h-4" />
          {currentResponse.timeMs}ms
        </span>
        <span className="text-sm text-gray-600">
          {currentResponse.sizeBytes > 1024
            ? `${(currentResponse.sizeBytes / 1024).toFixed(2)} KB`
            : `${currentResponse.sizeBytes} B`}
        </span>
        {currentResponse.contentType && (
          <span className="text-sm text-gray-500">{currentResponse.contentType}</span>
        )}
      </div>

      {/* Tabs */}
      <div className="flex border-b">
        {(['body', 'headers', 'cookies', 'test-results'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab
                ? 'border-blue-600 text-blue-600'
                : 'border-transparent text-gray-600 hover:text-gray-800'
            }`}
          >
            {tab === 'test-results' ? 'Test Results' : tab.charAt(0).toUpperCase() + tab.slice(1)}
            {tab === 'test-results' && testResults.length > 0 && (
              <span className={`ml-1 text-xs px-1.5 py-0.5 rounded-full ${
                testResults.every(t => t.passed) ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'
              }`}>
                {testResults.filter(t => t.passed).length}/{testResults.length}
              </span>
            )}
            {tab === 'cookies' && parsedCookies.length > 0 && (
              <span className="ml-1 text-xs bg-gray-200 px-1.5 py-0.5 rounded-full">
                {parsedCookies.length}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-auto">
        {/* Body Tab */}
        {activeTab === 'body' && (
          <div className="h-full flex flex-col">
            {/* Format Controls */}
            <div className="flex items-center justify-between px-4 py-2 border-b">
              <div className="flex gap-1">
                {(['pretty', 'raw', 'preview'] as const).map((format) => (
                  <button
                    key={format}
                    onClick={() => setBodyFormat(format)}
                    className={`px-3 py-1 text-sm rounded-md ${
                      bodyFormat === format
                        ? 'bg-blue-100 text-blue-700'
                        : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                    }`}
                  >
                    {format.charAt(0).toUpperCase() + format.slice(1)}
                  </button>
                ))}
              </div>
              <div className="flex gap-1">
                <button
                  onClick={handleCopy}
                  className="flex items-center gap-1 px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 rounded-md"
                >
                  {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                  {copied ? 'Copied!' : 'Copy'}
                </button>
                <button
                  onClick={handleDownload}
                  className="flex items-center gap-1 px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 rounded-md"
                >
                  <Download className="w-4 h-4" />
                  Download
                </button>
              </div>
            </div>

            {/* Body Content */}
            <div className="flex-1 overflow-auto p-4">
              <pre className="font-mono text-sm whitespace-pre-wrap">
                {formatBody(currentResponse.body, currentResponse.contentType)}
              </pre>
            </div>
          </div>
        )}

        {/* Headers Tab */}
        {activeTab === 'headers' && (
          <div className="p-4">
            <table className="w-full text-sm">
              <thead className="bg-gray-50">
                <tr>
                  <th className="text-left px-4 py-2 font-medium text-gray-700">Key</th>
                  <th className="text-left px-4 py-2 font-medium text-gray-700">Value</th>
                </tr>
              </thead>
              <tbody className="divide-y">
                {Object.entries(currentResponse.headers).map(([key, value]) => (
                  <tr key={key} className="hover:bg-gray-50">
                    <td className="px-4 py-2 font-medium text-gray-700">{key}</td>
                    <td className="px-4 py-2 text-gray-600 break-all">{value}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* Cookies Tab */}
        {activeTab === 'cookies' && (
          <div className="p-4">
            {parsedCookies.length > 0 ? (
              <table className="w-full text-sm">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="text-left px-4 py-2 font-medium text-gray-700">Cookie</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {parsedCookies.map((cookie, index) => (
                    <tr key={index} className="hover:bg-gray-50">
                      <td className="px-4 py-2 text-gray-600 break-all font-mono text-xs">{cookie}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : (
              <div className="text-center text-gray-500 py-8">
                <p>No cookies in response</p>
              </div>
            )}
          </div>
        )}

        {/* Test Results Tab */}
        {activeTab === 'test-results' && (
          <div className="p-4">
            {testResults.length > 0 ? (
              <div className="space-y-2">
                {testResults.map((result, index) => (
                  <div
                    key={index}
                    className={`flex items-center gap-3 p-3 rounded-md ${
                      result.passed ? 'bg-green-50 border border-green-200' : 'bg-red-50 border border-red-200'
                    }`}
                  >
                    {result.passed ? (
                      <CheckCircle className="w-5 h-5 text-green-600" />
                    ) : (
                      <XCircle className="w-5 h-5 text-red-600" />
                    )}
                    <div className="flex-1">
                      <div className="font-medium">{result.name}</div>
                      {!result.passed && result.errorMessage && (
                        <div className="text-sm text-red-600 mt-1">{result.errorMessage}</div>
                      )}
                    </div>
                    <div className="text-sm text-gray-500">{result.durationMs}ms</div>
                  </div>
                ))}
                <div className="flex items-center justify-between pt-4 border-t mt-4">
                  <div className="text-sm">
                    <span className="font-medium">{testResults.filter(t => t.passed).length}</span> of{' '}
                    <span className="font-medium">{testResults.length}</span> tests passed
                  </div>
                  <div
                    className={`px-3 py-1 rounded-full text-sm font-medium ${
                      testResults.every(t => t.passed)
                        ? 'bg-green-100 text-green-700'
                        : 'bg-red-100 text-red-700'
                    }`}
                  >
                    {testResults.every(t => t.passed) ? 'All Passed' : 'Failed'}
                  </div>
                </div>
              </div>
            ) : (
              <div className="text-center text-gray-500 py-8">
                <p>No tests run</p>
                <p className="text-sm mt-1">Add a test script in the Tests tab to see results</p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default ResponseViewer;
