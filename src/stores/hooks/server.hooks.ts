import { useMemo } from 'react';
import { useServerStore } from '../serverStore';
import { filterServersByQuery, groupServersByGroupId, sortServersByName } from '../utils/server.utils';

export function useFilteredServers() {
  const { servers, searchQuery } = useServerStore();

  return useMemo(() => {
    const filtered = filterServersByQuery(servers, searchQuery);
    return sortServersByName(filtered);
  }, [servers, searchQuery]);
}

export function useGroupedServers() {
  const { servers } = useServerStore();

  return useMemo(() => {
    const sorted = sortServersByName(servers);
    return groupServersByGroupId(sorted);
  }, [servers]);
}

export function useServerCount() {
  const { servers } = useServerStore();
  return useMemo(() => servers.length, [servers]);
}
