import type { Server } from '../domain/server.types';

export function filterServersByQuery(servers: Server[], query: string): Server[] {
  if (!query.trim()) return servers;

  const normalizedQuery = query.toLowerCase().trim();
  return servers.filter(
    (server) =>
      server.name.toLowerCase().includes(normalizedQuery) ||
      server.host.toLowerCase().includes(normalizedQuery) ||
      server.username.toLowerCase().includes(normalizedQuery)
  );
}

export function groupServersByGroupId(servers: Server[]): Map<string | undefined, Server[]> {
  const grouped = new Map<string | undefined, Server[]>();

  for (const server of servers) {
    const key = server.group_id;
    if (!grouped.has(key)) {
      grouped.set(key, []);
    }
    grouped.get(key)!.push(server);
  }

  return grouped;
}

export function sortServersByName(servers: Server[]): Server[] {
  return [...servers].sort((a, b) => a.name.localeCompare(b.name));
}
