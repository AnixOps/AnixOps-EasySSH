# EasySSH Pro API Client Examples

## TypeScript/JavaScript Client

```typescript
// api-client.ts
export class EasySSHProClient {
  private baseUrl: string;
  private token: string | null = null;

  constructor(baseUrl: string = 'https://api.easyssh.io') {
    this.baseUrl = baseUrl;
  }

  setToken(token: string) {
    this.token = token;
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}/api/v1${endpoint}`;
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      ...options.headers,
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(url, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || `HTTP ${response.status}`);
    }

    return response.json();
  }

  // Auth
  async login(email: string, password: string) {
    return this.request('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
  }

  async register(email: string, password: string, name: string) {
    return this.request('/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, name }),
    });
  }

  async refreshToken(refreshToken: string) {
    return this.request('/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
  }

  async logout() {
    return this.request('/auth/logout', { method: 'POST' });
  }

  async getCurrentUser() {
    return this.request('/auth/me');
  }

  async createApiKey(name: string, scopes?: string[], expiresInDays?: number) {
    return this.request('/auth/api-keys', {
      method: 'POST',
      body: JSON.stringify({ name, scopes, expires_in_days: expiresInDays }),
    });
  }

  // Teams
  async createTeam(name: string, description?: string) {
    return this.request('/teams', {
      method: 'POST',
      body: JSON.stringify({ name, description }),
    });
  }

  async listTeams(page?: number, limit?: number) {
    const params = new URLSearchParams();
    if (page) params.append('page', page.toString());
    if (limit) params.append('limit', limit.toString());
    return this.request(`/teams?${params}`);
  }

  async getTeam(teamId: string) {
    return this.request(`/teams/${teamId}`);
  }

  async updateTeam(teamId: string, updates: { name?: string; description?: string }) {
    return this.request(`/teams/${teamId}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  }

  async deleteTeam(teamId: string) {
    return this.request(`/teams/${teamId}`, { method: 'DELETE' });
  }

  async listTeamMembers(teamId: string) {
    return this.request(`/teams/${teamId}/members`);
  }

  async inviteMember(teamId: string, email: string, role: 'admin' | 'member' | 'guest') {
    return this.request(`/teams/${teamId}/members`, {
      method: 'POST',
      body: JSON.stringify({ email, role }),
    });
  }

  async acceptInvitation(token: string) {
    return this.request(`/teams/invitations/${token}/accept`, { method: 'POST' });
  }

  // Audit Logs
  async queryAuditLogs(params: {
    team_id?: string;
    user_id?: string;
    action?: string;
    resource_type?: string;
    from_date?: string;
    to_date?: string;
    limit?: number;
    offset?: number;
  }) {
    const searchParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined) searchParams.append(key, String(value));
    });
    return this.request(`/audit?${searchParams}`);
  }

  async exportAuditLogs(params: object) {
    const searchParams = new URLSearchParams();
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined) searchParams.append(key, String(value));
    });
    return this.request(`/audit/export?${searchParams}`);
  }

  // RBAC
  async listRoles(page?: number, limit?: number) {
    const params = new URLSearchParams();
    if (page) params.append('page', page.toString());
    if (limit) params.append('limit', limit.toString());
    return this.request(`/rbac/roles?${params}`);
  }

  async listPermissions() {
    return this.request('/rbac/permissions');
  }

  async checkPermission(resourceType: string, action: string, resourceId?: string, teamId?: string) {
    return this.request('/rbac/check', {
      method: 'POST',
      body: JSON.stringify({
        resource_type: resourceType,
        action,
        resource_id: resourceId,
        team_id: teamId,
      }),
    });
  }

  // Resources
  async shareServer(serverId: string, teamId: string, permissions?: object) {
    return this.request('/resources/servers', {
      method: 'POST',
      body: JSON.stringify({
        server_id: serverId,
        team_id: teamId,
        permissions,
      }),
    });
  }

  async createSnippet(teamId: string, name: string, content: string, options?: {
    description?: string;
    language?: string;
    tags?: string[];
    is_public?: boolean;
  }) {
    return this.request('/resources/snippets', {
      method: 'POST',
      body: JSON.stringify({
        team_id: teamId,
        name,
        content,
        ...options,
      }),
    });
  }

  async listSnippets(teamId?: string) {
    const params = teamId ? `?team_id=${teamId}` : '';
    return this.request(`/resources/snippets${params}`);
  }

  async updateSnippet(snippetId: string, updates: object) {
    return this.request(`/resources/snippets/${snippetId}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  }

  async deleteSnippet(snippetId: string) {
    return this.request(`/resources/snippets/${snippetId}`, { method: 'DELETE' });
  }
}

// WebSocket Client
export class EasySSHProWebSocket {
  private ws: WebSocket;
  private listeners: Map<string, Function[]> = new Map();

  constructor(url: string, token: string) {
    this.ws = new WebSocket(`${url}/api/v1/ws?token=${token}`);

    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      this.emit(data.type, data);
    };
  }

  on(event: string, handler: Function) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event)!.push(handler);
  }

  private emit(event: string, data: any) {
    const handlers = this.listeners.get(event) || [];
    handlers.forEach((h) => h(data));
  }

  subscribe(channels: string[]) {
    this.ws.send(JSON.stringify({ type: 'Subscribe', channels }));
  }

  unsubscribe(channels: string[]) {
    this.ws.send(JSON.stringify({ type: 'Unsubscribe', channels }));
  }

  close() {
    this.ws.close();
  }
}

// Usage Example
async function example() {
  const client = new EasySSHProClient('http://localhost:8080');

  // Login
  const { access_token, refresh_token, user } = await client.login(
    'user@example.com',
    'password123'
  );
  client.setToken(access_token);

  // Create team
  const team = await client.createTeam('Engineering', 'Engineering Team');
  console.log('Created team:', team);

  // Invite member
  await client.inviteMember(team.data.id, 'new@example.com', 'member');

  // Create snippet
  const snippet = await client.createSnippet(
    team.data.id,
    'SSH Config',
    'Host prod\n  HostName prod.example.com',
    { language: 'ssh_config', tags: ['production', 'ssh'] }
  );
  console.log('Created snippet:', snippet);

  // Query audit logs
  const logs = await client.queryAuditLogs({ team_id: team.data.id, limit: 10 });
  console.log('Audit logs:', logs);

  // WebSocket
  const ws = new EasySSHProWebSocket('ws://localhost:8080', access_token);
  ws.on('Notification', (msg) => console.log('Notification:', msg));
  ws.on('CollaborationUpdate', (msg) => console.log('Collaboration:', msg));
  ws.subscribe(['team:updates', 'notifications']);
}
```

## Python Client

```python
# api_client.py
import requests
from typing import Optional, Dict, Any, List


class EasySSHProClient:
    def __init__(self, base_url: str = "https://api.easyssh.io"):
        self.base_url = base_url
        self.token: Optional[str] = None
        self.session = requests.Session()

    def set_token(self, token: str):
        self.token = token
        self.session.headers["Authorization"] = f"Bearer {token}"

    def _request(self, method: str, endpoint: str, **kwargs) -> Dict[str, Any]:
        url = f"{self.base_url}/api/v1{endpoint}"
        headers = kwargs.pop("headers", {})

        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"

        response = self.session.request(method, url, headers=headers, **kwargs)
        response.raise_for_status()
        return response.json()

    # Auth
    def login(self, email: str, password: str) -> Dict[str, Any]:
        return self._request("POST", "/auth/login", json={"email": email, "password": password})

    def register(self, email: str, password: str, name: str) -> Dict[str, Any]:
        return self._request("POST", "/auth/register", json={"email": email, "password": password, "name": name})

    def refresh_token(self, refresh_token: str) -> Dict[str, Any]:
        return self._request("POST", "/auth/refresh", json={"refresh_token": refresh_token})

    def logout(self):
        return self._request("POST", "/auth/logout")

    def get_current_user(self) -> Dict[str, Any]:
        return self._request("GET", "/auth/me")

    def create_api_key(self, name: str, scopes: Optional[List[str]] = None, expires_in_days: Optional[int] = None):
        return self._request("POST", "/auth/api-keys", json={"name": name, "scopes": scopes, "expires_in_days": expires_in_days})

    # Teams
    def create_team(self, name: str, description: Optional[str] = None):
        return self._request("POST", "/teams", json={"name": name, "description": description})

    def list_teams(self, page: Optional[int] = None, limit: Optional[int] = None):
        params = {}
        if page:
            params["page"] = page
        if limit:
            params["limit"] = limit
        return self._request("GET", "/teams", params=params)

    def get_team(self, team_id: str):
        return self._request("GET", f"/teams/{team_id}")

    def update_team(self, team_id: str, name: Optional[str] = None, description: Optional[str] = None):
        return self._request("PUT", f"/teams/{team_id}", json={"name": name, "description": description})

    def delete_team(self, team_id: str):
        return self._request("DELETE", f"/teams/{team_id}")

    def list_team_members(self, team_id: str):
        return self._request("GET", f"/teams/{team_id}/members")

    def invite_member(self, team_id: str, email: str, role: str = "member"):
        return self._request("POST", f"/teams/{team_id}/members", json={"email": email, "role": role})

    # Audit Logs
    def query_audit_logs(self, **kwargs):
        return self._request("GET", "/audit", params=kwargs)

    def export_audit_logs(self, **kwargs):
        return self._request("GET", "/audit/export", params=kwargs)

    # Resources
    def share_server(self, server_id: str, team_id: str, permissions: Optional[Dict] = None):
        return self._request("POST", "/resources/servers", json={
            "server_id": server_id,
            "team_id": team_id,
            "permissions": permissions
        })

    def create_snippet(self, team_id: str, name: str, content: str, **kwargs):
        return self._request("POST", "/resources/snippets", json={
            "team_id": team_id,
            "name": name,
            "content": content,
            **kwargs
        })

    def list_snippets(self, team_id: Optional[str] = None):
        params = {}
        if team_id:
            params["team_id"] = team_id
        return self._request("GET", "/resources/snippets", params=params)


# Usage
if __name__ == "__main__":
    client = EasySSHProClient("http://localhost:8080")

    # Login
    result = client.login("user@example.com", "password123")
    client.set_token(result["data"]["access_token"])

    # Create team
    team = client.create_team("DevOps", "DevOps Team")
    print(f"Created team: {team}")

    # Create snippet
    snippet = client.create_snippet(
        team["data"]["id"],
        "Docker Compose",
        "version: '3.8'\\nservices:",
        language="yaml",
        tags=["docker", "compose"]
    )
    print(f"Created snippet: {snippet}")
```

## curl Examples

```bash
# Login
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com","password":"password123"}' | jq -r '.data.access_token')

# Create team
curl -X POST http://localhost:8080/api/v1/teams \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Engineering","description":"Engineering Team"}'

# List teams
curl -X GET http://localhost:8080/api/v1/teams \
  -H "Authorization: Bearer $TOKEN"

# Create snippet
curl -X POST http://localhost:8080/api/v1/resources/snippets \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "team_id": "<team-id>",
    "name": "SSH Config",
    "content": "Host prod\\n  HostName prod.example.com",
    "language": "ssh_config",
    "tags": ["production", "ssh"]
  }'

# Query audit logs
curl -X GET "http://localhost:8080/api/v1/audit?team_id=<team-id>&limit=10" \
  -H "Authorization: Bearer $TOKEN"

# Export audit logs to CSV
curl -X GET "http://localhost:8080/api/v1/audit/export?team_id=<team-id>" \
  -H "Authorization: Bearer $TOKEN" \
  -o audit_logs.csv
```
