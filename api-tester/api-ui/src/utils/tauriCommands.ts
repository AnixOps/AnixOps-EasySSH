import { invoke } from '@tauri-apps/api/core';
import {
  ApiRequest,
  ApiResponse,
  Collection,
  Environment,
  HistoryEntry,
  TestResult,
} from '../stores/apiTesterStore';

// HTTP Commands
export async function executeRequest(
  request: ApiRequest,
  environmentId?: string
): Promise<{ response: ApiResponse; testResults: TestResult[] }> {
  return invoke('execute_request', { request, environmentId });
}

export async function sendHttpRequest(
  url: string,
  method: string,
  headers: { key: string; value: string; enabled: boolean }[],
  body: any,
  auth: any
): Promise<ApiResponse> {
  return invoke('send_http_request', { url, method, headers, body, auth });
}

// Collection Commands
export async function saveCollection(collection: Collection): Promise<void> {
  return invoke('save_collection', { collection });
}

export async function getCollection(id: string): Promise<Collection | null> {
  return invoke('get_collection', { id });
}

export async function listCollections(): Promise<Collection[]> {
  return invoke('list_collections');
}

export async function deleteCollection(id: string): Promise<void> {
  return invoke('delete_collection', { id });
}

export async function saveRequest(
  request: ApiRequest,
  collectionId?: string,
  folderId?: string
): Promise<void> {
  return invoke('save_request', { request, collectionId, folderId });
}

export async function getRequest(id: string): Promise<ApiRequest | null> {
  return invoke('get_request', { id });
}

export async function deleteRequest(id: string): Promise<void> {
  return invoke('delete_request', { id });
}

export async function duplicateRequest(id: string): Promise<ApiRequest> {
  return invoke('duplicate_request', { id });
}

export async function searchRequests(query: string): Promise<ApiRequest[]> {
  return invoke('search_requests', { query });
}

// Environment Commands
export async function saveEnvironment(env: Environment): Promise<void> {
  return invoke('save_environment', { env });
}

export async function getEnvironment(id: string): Promise<Environment | null> {
  return invoke('get_environment', { id });
}

export async function listEnvironments(): Promise<Environment[]> {
  return invoke('list_environments');
}

export async function setActiveEnvironment(id: string | null): Promise<void> {
  return invoke('set_active_environment', { id });
}

export async function getActiveEnvironment(): Promise<Environment | null> {
  return invoke('get_active_environment');
}

export async function deleteEnvironment(id: string): Promise<void> {
  return invoke('delete_environment', { id });
}

// History Commands
export async function getHistory(limit: number = 100): Promise<HistoryEntry[]> {
  return invoke('get_history', { limit });
}

export async function searchHistory(query: string): Promise<HistoryEntry[]> {
  return invoke('search_history', { query });
}

export async function clearHistory(olderThanDays?: number): Promise<void> {
  return invoke('clear_history', { olderThanDays });
}

export async function replayRequest(entryId: string): Promise<ApiRequest | null> {
  return invoke('replay_request', { entryId });
}

// Import/Export Commands
export async function importPostmanCollection(data: string): Promise<Collection> {
  return invoke('import_postman_collection', { data });
}

export async function importPostmanEnvironment(data: string): Promise<Environment> {
  return invoke('import_postman_environment', { data });
}

export async function importCurlCommand(command: string): Promise<ApiRequest> {
  return invoke('import_curl_command', { command });
}

export async function exportPostmanCollection(collection: Collection): Promise<string> {
  return invoke('export_postman_collection', { collection });
}

export async function exportPostmanEnvironment(env: Environment): Promise<string> {
  return invoke('export_postman_environment', { env });
}

export async function exportCurlCommand(request: ApiRequest): Promise<string> {
  return invoke('export_curl_command', { request });
}

// Test Commands
export async function runTests(
  testScript: string,
  response: ApiResponse
): Promise<TestResult[]> {
  return invoke('run_tests', { testScript, response });
}

export async function generateTestScript(response: ApiResponse): Promise<string> {
  return invoke('generate_test_script', { response });
}

// WebSocket Commands
export async function wsConnect(
  id: string,
  url: string,
  headers?: { key: string; value: string; enabled: boolean }[]
): Promise<void> {
  return invoke('ws_connect', { id, url, headers });
}

export async function wsSend(id: string, message: string): Promise<void> {
  return invoke('ws_send', { id, message });
}

export async function wsGetMessages(id: string): Promise<any[]> {
  return invoke('ws_get_messages', { id });
}

export async function wsDisconnect(id: string): Promise<void> {
  return invoke('ws_disconnect', { id });
}

export async function wsIsConnected(id: string): Promise<boolean> {
  return invoke('ws_is_connected', { id });
}
