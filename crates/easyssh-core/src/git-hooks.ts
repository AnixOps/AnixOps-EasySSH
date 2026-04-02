/**
 * Git Client React Hooks
 *
 * These hooks provide React-friendly access to the Git client functionality.
 * They can be used with any UI framework by implementing the appropriate GitApi interface.
 */

import {
  useState,
  useCallback,
  useEffect,
  useRef,
  useContext,
  createContext,
  ReactNode,
} from 'react';
import {
  RepoStatus,
  FileEntry,
  CommitInfo,
  BranchInfo,
  RemoteInfo,
  TagInfo,
  StashEntry,
  SubmoduleInfo,
  DiffEntry,
  BlameLine,
  ConflictInfo,
  MergeResult,
  CloneOptions,
  PushOptions,
  FetchOptions,
  RepoStats,
  CredentialType,
  GitOperationEvent,
} from './git-types';

// ===== Git API Interface =====

export interface GitApi {
  // Repository operations
  openRepo(path: string): Promise<string>;
  cloneRepo(url: string, path: string, options?: CloneOptions, credentials?: CredentialType): Promise<string>;
  initRepo(path: string, bare?: boolean): Promise<string>;
  closeRepo(repoId: string): Promise<void>;

  // Status operations
  getStatus(repoId?: string): Promise<RepoStatus>;
  getFileStatuses(repoId?: string): Promise<FileEntry[]>;

  // Staging operations
  stage(paths: string[], repoId?: string): Promise<void>;
  unstage(paths: string[], repoId?: string): Promise<void>;
  discard(paths: string[], repoId?: string): Promise<void>;

  // Commit operations
  commit(message: string, amend?: boolean, repoId?: string): Promise<string>;
  getLog(branch?: string, limit?: number, repoId?: string): Promise<CommitInfo[]>;

  // Diff operations
  diffCommit(commitId: string, repoId?: string): Promise<DiffEntry[]>;
  diffWorkdir(repoId?: string): Promise<DiffEntry[]>;
  diffStaged(repoId?: string): Promise<DiffEntry[]>;

  // Branch operations
  getBranches(repoId?: string): Promise<BranchInfo[]>;
  createBranch(name: string, startPoint?: string, repoId?: string): Promise<void>;
  checkoutBranch(name: string, create?: boolean, repoId?: string): Promise<void>;
  deleteBranch(name: string, repoId?: string): Promise<void>;
  merge(branchName: string, repoId?: string): Promise<MergeResult>;

  // Conflict operations
  getConflicts(repoId?: string): Promise<ConflictInfo[]>;
  resolveConflict(path: string, content: string, repoId?: string): Promise<void>;
  abortMerge(repoId?: string): Promise<void>;

  // Remote operations
  getRemotes(repoId?: string): Promise<RemoteInfo[]>;
  addRemote(name: string, url: string, repoId?: string): Promise<void>;
  removeRemote(name: string, repoId?: string): Promise<void>;
  fetch(remote: string, prune?: boolean, tags?: boolean, repoId?: string): Promise<void>;
  pull(remote: string, repoId?: string): Promise<MergeResult>;
  push(refspec: string, options: PushOptions, repoId?: string): Promise<void>;

  // Tag operations
  getTags(repoId?: string): Promise<TagInfo[]>;
  createTag(name: string, target?: string, message?: string, repoId?: string): Promise<void>;
  deleteTag(name: string, repoId?: string): Promise<void>;
  pushTag(tagName: string, remote: string, repoId?: string): Promise<void>;

  // Stash operations
  getStashList(repoId?: string): Promise<StashEntry[]>;
  stashSave(message?: string, includeUntracked?: boolean, repoId?: string): Promise<void>;
  stashPop(index: number, repoId?: string): Promise<void>;
  stashApply(index: number, repoId?: string): Promise<void>;
  stashDrop(index: number, repoId?: string): Promise<void>;

  // Submodule operations
  getSubmodules(repoId?: string): Promise<SubmoduleInfo[]>;
  addSubmodule(url: string, path: string, repoId?: string): Promise<void>;
  updateSubmodules(repoId?: string): Promise<void>;

  // Blame operations
  blame(path: string, oldestCommit?: string, repoId?: string): Promise<BlameLine[]>;

  // Statistics
  getStats(repoId?: string): Promise<RepoStats>;

  // File content
  getFileAtCommit(path: string, commitId: string, repoId?: string): Promise<string>;
}

// ===== Git Context =====

interface GitContextValue {
  api: GitApi;
  activeRepoId: string | null;
  setActiveRepoId: (id: string | null) => void;
}

const GitContext = createContext<GitContextValue | null>(null);

export interface GitProviderProps {
  api: GitApi;
  children: ReactNode;
}

export function GitProvider({ api, children }: GitProviderProps) {
  const [activeRepoId, setActiveRepoId] = useState<string | null>(null);

  return (
    <GitContext.Provider value={{ api, activeRepoId, setActiveRepoId }}>
      {children}
    </GitContext.Provider>
  );
}

export function useGit() {
  const context = useContext(GitContext);
  if (!context) {
    throw new Error('useGit must be used within a GitProvider');
  }
  return context;
}

// ===== Status Hook =====

export function useGitStatus(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [status, setStatus] = useState<RepoStatus | null>(null);
  const [fileStatuses, setFileStatuses] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const [newStatus, newFileStatuses] = await Promise.all([
        api.getStatus(targetRepoId),
        api.getFileStatuses(targetRepoId),
      ]);
      setStatus(newStatus);
      setFileStatuses(newFileStatuses);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const stage = useCallback(async (paths: string[]) => {
    if (!targetRepoId) return;
    await api.stage(paths, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const unstage = useCallback(async (paths: string[]) => {
    if (!targetRepoId) return;
    await api.unstage(paths, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const discard = useCallback(async (paths: string[]) => {
    if (!targetRepoId) return;
    await api.discard(paths, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const stageAll = useCallback(async () => {
    const paths = fileStatuses.filter(f => !f.staged).map(f => f.path);
    await stage(paths);
  }, [fileStatuses, stage]);

  const unstageAll = useCallback(async () => {
    const paths = fileStatuses.filter(f => f.staged).map(f => f.path);
    await unstage(paths);
  }, [fileStatuses, unstage]);

  return {
    status,
    fileStatuses,
    loading,
    error,
    refresh,
    stage,
    unstage,
    discard,
    stageAll,
    unstageAll,
  };
}

// ===== Commit Hook =====

export function useGitCommits(repoId?: string, branch?: string, limit = 50) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [commits, setCommits] = useState<CommitInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newCommits = await api.getLog(branch, limit, targetRepoId);
      setCommits(newCommits);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId, branch, limit]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const commit = useCallback(async (message: string, amend = false) => {
    if (!targetRepoId) return;
    const commitId = await api.commit(message, amend, targetRepoId);
    await refresh();
    return commitId;
  }, [api, targetRepoId, refresh]);

  return { commits, loading, error, refresh, commit };
}

// ===== Branch Hook =====

export function useGitBranches(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [branches, setBranches] = useState<BranchInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newBranches = await api.getBranches(targetRepoId);
      setBranches(newBranches);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const currentBranch = branches.find(b => b.is_head);

  const createBranch = useCallback(async (name: string, startPoint?: string) => {
    if (!targetRepoId) return;
    await api.createBranch(name, startPoint, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const checkoutBranch = useCallback(async (name: string, create = false) => {
    if (!targetRepoId) return;
    await api.checkoutBranch(name, create, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const deleteBranch = useCallback(async (name: string) => {
    if (!targetRepoId) return;
    await api.deleteBranch(name, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const merge = useCallback(async (branchName: string) => {
    if (!targetRepoId) return;
    const result = await api.merge(branchName, targetRepoId);
    await refresh();
    return result;
  }, [api, targetRepoId, refresh]);

  return {
    branches,
    currentBranch,
    loading,
    error,
    refresh,
    createBranch,
    checkoutBranch,
    deleteBranch,
    merge,
  };
}

// ===== Remote Hook =====

export function useGitRemotes(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [remotes, setRemotes] = useState<RemoteInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [operationLoading, setOperationLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newRemotes = await api.getRemotes(targetRepoId);
      setRemotes(newRemotes);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const addRemote = useCallback(async (name: string, url: string) => {
    if (!targetRepoId) return;
    await api.addRemote(name, url, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const removeRemote = useCallback(async (name: string) => {
    if (!targetRepoId) return;
    await api.removeRemote(name, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const fetch = useCallback(async (remote: string, prune = false, tags = false) => {
    if (!targetRepoId) return;
    setOperationLoading(true);
    try {
      await api.fetch(remote, prune, tags, targetRepoId);
    } finally {
      setOperationLoading(false);
    }
  }, [api, targetRepoId]);

  const pull = useCallback(async (remote: string) => {
    if (!targetRepoId) return;
    setOperationLoading(true);
    try {
      const result = await api.pull(remote, targetRepoId);
      return result;
    } finally {
      setOperationLoading(false);
    }
  }, [api, targetRepoId]);

  const push = useCallback(async (refspec: string, options: PushOptions) => {
    if (!targetRepoId) return;
    setOperationLoading(true);
    try {
      await api.push(refspec, options, targetRepoId);
    } finally {
      setOperationLoading(false);
    }
  }, [api, targetRepoId]);

  return {
    remotes,
    loading,
    operationLoading,
    error,
    refresh,
    addRemote,
    removeRemote,
    fetch,
    pull,
    push,
  };
}

// ===== Stash Hook =====

export function useGitStash(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [stashes, setStashes] = useState<StashEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newStashes = await api.getStashList(targetRepoId);
      setStashes(newStashes);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const save = useCallback(async (message?: string, includeUntracked = false) => {
    if (!targetRepoId) return;
    await api.stashSave(message, includeUntracked, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const pop = useCallback(async (index: number) => {
    if (!targetRepoId) return;
    await api.stashPop(index, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const apply = useCallback(async (index: number) => {
    if (!targetRepoId) return;
    await api.stashApply(index, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const drop = useCallback(async (index: number) => {
    if (!targetRepoId) return;
    await api.stashDrop(index, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  return {
    stashes,
    loading,
    error,
    refresh,
    save,
    pop,
    apply,
    drop,
  };
}

// ===== Tag Hook =====

export function useGitTags(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [tags, setTags] = useState<TagInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newTags = await api.getTags(targetRepoId);
      setTags(newTags);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const createTag = useCallback(async (name: string, target?: string, message?: string) => {
    if (!targetRepoId) return;
    await api.createTag(name, target, message, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const deleteTag = useCallback(async (name: string) => {
    if (!targetRepoId) return;
    await api.deleteTag(name, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const pushTag = useCallback(async (tagName: string, remote: string) => {
    if (!targetRepoId) return;
    await api.pushTag(tagName, remote, targetRepoId);
  }, [api, targetRepoId]);

  return {
    tags,
    loading,
    error,
    refresh,
    createTag,
    deleteTag,
    pushTag,
  };
}

// ===== Diff Hook =====

export function useGitDiff(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [stagedDiff, setStagedDiff] = useState<DiffEntry[]>([]);
  const [workdirDiff, setWorkdirDiff] = useState<DiffEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const [staged, workdir] = await Promise.all([
        api.diffStaged(targetRepoId),
        api.diffWorkdir(targetRepoId),
      ]);
      setStagedDiff(staged);
      setWorkdirDiff(workdir);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const getCommitDiff = useCallback(async (commitId: string) => {
    if (!targetRepoId) return [];
    return await api.diffCommit(commitId, targetRepoId);
  }, [api, targetRepoId]);

  return {
    stagedDiff,
    workdirDiff,
    loading,
    error,
    refresh,
    getCommitDiff,
  };
}

// ===== Blame Hook =====

export function useGitBlame(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [blameLines, setBlameLines] = useState<BlameLine[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const blame = useCallback(async (path: string, oldestCommit?: string) => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const lines = await api.blame(path, oldestCommit, targetRepoId);
      setBlameLines(lines);
      return lines;
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      return [];
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  return {
    blameLines,
    loading,
    error,
    blame,
  };
}

// ===== Conflict Hook =====

export function useGitConflicts(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [conflicts, setConflicts] = useState<ConflictInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newConflicts = await api.getConflicts(targetRepoId);
      setConflicts(newConflicts);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const resolve = useCallback(async (path: string, content: string) => {
    if (!targetRepoId) return;
    await api.resolveConflict(path, content, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const abort = useCallback(async () => {
    if (!targetRepoId) return;
    await api.abortMerge(targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  return {
    conflicts,
    loading,
    error,
    refresh,
    resolve,
    abort,
    hasConflicts: conflicts.length > 0,
  };
}

// ===== Submodule Hook =====

export function useGitSubmodules(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [submodules, setSubmodules] = useState<SubmoduleInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newSubmodules = await api.getSubmodules(targetRepoId);
      setSubmodules(newSubmodules);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  const addSubmodule = useCallback(async (url: string, path: string) => {
    if (!targetRepoId) return;
    await api.addSubmodule(url, path, targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  const updateSubmodules = useCallback(async () => {
    if (!targetRepoId) return;
    await api.updateSubmodules(targetRepoId);
    await refresh();
  }, [api, targetRepoId, refresh]);

  return {
    submodules,
    loading,
    error,
    refresh,
    addSubmodule,
    updateSubmodules,
  };
}

// ===== Repository Manager Hook =====

export function useGitRepoManager() {
  const { api, setActiveRepoId } = useGit();

  const [repos, setRepos] = useState<{ id: string; path: string | null }[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const openRepo = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);

    try {
      const id = await api.openRepo(path);
      await refreshRepos();
      setActiveRepoId(id);
      return id;
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [api, setActiveRepoId]);

  const cloneRepo = useCallback(async (
    url: string,
    path: string,
    options?: CloneOptions,
    credentials?: CredentialType
  ) => {
    setLoading(true);
    setError(null);

    try {
      const id = await api.cloneRepo(url, path, options, credentials);
      await refreshRepos();
      setActiveRepoId(id);
      return id;
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [api, setActiveRepoId]);

  const initRepo = useCallback(async (path: string, bare = false) => {
    setLoading(true);
    setError(null);

    try {
      const id = await api.initRepo(path, bare);
      await refreshRepos();
      setActiveRepoId(id);
      return id;
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [api, setActiveRepoId]);

  const closeRepo = useCallback(async (id: string) => {
    await api.closeRepo(id);
    await refreshRepos();
  }, [api]);

  const refreshRepos = useCallback(async () => {
    // Note: This would need to be implemented in the API
    // For now, we'll assume the API has a method to list repos
    // setRepos(await api.listRepos());
  }, []);

  return {
    repos,
    loading,
    error,
    openRepo,
    cloneRepo,
    initRepo,
    closeRepo,
  };
}

// ===== Stats Hook =====

export function useGitStats(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [stats, setStats] = useState<RepoStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!targetRepoId) return;

    setLoading(true);
    setError(null);

    try {
      const newStats = await api.getStats(targetRepoId);
      setStats(newStats);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  useEffect(() => {
    if (targetRepoId) {
      refresh();
    }
  }, [targetRepoId, refresh]);

  return { stats, loading, error, refresh };
}

// ===== File Content Hook =====

export function useGitFileContent(repoId?: string) {
  const { api, activeRepoId } = useGit();
  const targetRepoId = repoId ?? activeRepoId;

  const [content, setContent] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const getFileAtCommit = useCallback(async (path: string, commitId: string) => {
    if (!targetRepoId) return '';

    setLoading(true);
    setError(null);

    try {
      const newContent = await api.getFileAtCommit(path, commitId, targetRepoId);
      setContent(newContent);
      return newContent;
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      return '';
    } finally {
      setLoading(false);
    }
  }, [api, targetRepoId]);

  return { content, loading, error, getFileAtCommit };
}

// ===== Operation Progress Hook =====

export function useGitOperationProgress() {
  const [events, setEvents] = useState<GitOperationEvent[]>([]);
  const [currentOperation, setCurrentOperation] = useState<string | null>(null);

  // This would integrate with WebSocket or event source for real-time updates
  const addEvent = useCallback((event: GitOperationEvent) => {
    setEvents(prev => [...prev, event]);
    if (!event.completed) {
      setCurrentOperation(event.operation);
    } else {
      setCurrentOperation(null);
    }
  }, []);

  const clearEvents = useCallback(() => {
    setEvents([]);
    setCurrentOperation(null);
  }, []);

  return {
    events,
    currentOperation,
    addEvent,
    clearEvents,
    isOperationInProgress: currentOperation !== null,
  };
}

// ===== Combined Git Hook =====

export function useGitClient(repoId?: string) {
  const status = useGitStatus(repoId);
  const commits = useGitCommits(repoId);
  const branches = useGitBranches(repoId);
  const remotes = useGitRemotes(repoId);
  const stash = useGitStash(repoId);
  const tags = useGitTags(repoId);
  const diff = useGitDiff(repoId);
  const blame = useGitBlame(repoId);
  const conflicts = useGitConflicts(repoId);
  const submodules = useGitSubmodules(repoId);
  const stats = useGitStats(repoId);
  const fileContent = useGitFileContent(repoId);
  const progress = useGitOperationProgress();

  return {
    // Status
    ...status,

    // Commits
    commits: commits.commits,
    refreshCommits: commits.refresh,
    commit: commits.commit,

    // Branches
    branches: branches.branches,
    currentBranch: branches.currentBranch,
    refreshBranches: branches.refresh,
    createBranch: branches.createBranch,
    checkoutBranch: branches.checkoutBranch,
    deleteBranch: branches.deleteBranch,
    merge: branches.merge,

    // Remotes
    remotes: remotes.remotes,
    refreshRemotes: remotes.refresh,
    addRemote: remotes.addRemote,
    removeRemote: remotes.removeRemote,
    fetch: remotes.fetch,
    pull: remotes.pull,
    push: remotes.push,

    // Stash
    stashes: stash.stashes,
    refreshStash: stash.refresh,
    stashSave: stash.save,
    stashPop: stash.pop,
    stashApply: stash.apply,
    stashDrop: stash.drop,

    // Tags
    tags: tags.tags,
    refreshTags: tags.refresh,
    createTag: tags.createTag,
    deleteTag: tags.deleteTag,
    pushTag: tags.pushTag,

    // Diff
    ...diff,

    // Blame
    blameLines: blame.blameLines,
    blame: blame.blame,

    // Conflicts
    conflicts: conflicts.conflicts,
    hasConflicts: conflicts.hasConflicts,
    refreshConflicts: conflicts.refresh,
    resolveConflict: conflicts.resolve,
    abortMerge: conflicts.abort,

    // Submodules
    submodules: submodules.submodules,
    refreshSubmodules: submodules.refresh,
    addSubmodule: submodules.addSubmodule,
    updateSubmodules: submodules.updateSubmodules,

    // Stats
    stats: stats.stats,
    refreshStats: stats.refresh,

    // File content
    fileContent: fileContent.content,
    getFileAtCommit: fileContent.getFileAtCommit,

    // Progress
    operationEvents: progress.events,
    currentOperation: progress.currentOperation,
    isOperationInProgress: progress.isOperationInProgress,
  };
}
