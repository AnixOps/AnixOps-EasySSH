/**
 * Git Client API Types
 *
 * This file contains TypeScript type definitions for the Git client functionality.
 * These types mirror the Rust types defined in the core library.
 */

// ===== Basic Types =====

export interface GitSignature {
  name: string;
  email: string;
  timestamp: string;
}

export interface RepoStatus {
  is_bare: boolean;
  branch: string;
  upstream?: string;
  ahead: number;
  behind: number;
  has_changes: boolean;
  staged_count: number;
  unstaged_count: number;
  untracked_count: number;
  conflicted_count: number;
  stashed_count: number;
  last_commit?: string;
  state: RepoState;
}

export type RepoState =
  | 'Clean'
  | 'Merge'
  | 'Rebase'
  | 'Revert'
  | 'CherryPick'
  | 'Bisect'
  | 'ApplyMailbox';

export type FileStatus =
  | 'Unmodified'
  | 'Added'
  | 'Modified'
  | 'Deleted'
  | 'Renamed'
  | 'Copied'
  | 'Ignored'
  | 'Untracked'
  | 'TypeChange'
  | 'Conflict';

export interface FileEntry {
  path: string;
  status: FileStatus;
  staged: boolean;
  old_path?: string;
  similarity: number;
}

export interface CommitInfo {
  id: string;
  short_id: string;
  message: string;
  summary: string;
  author: GitSignature;
  committer: GitSignature;
  parents: string[];
  tree_id: string;
}

export interface BranchInfo {
  name: string;
  is_head: boolean;
  upstream?: string;
  ahead: number;
  behind: number;
  last_commit?: string;
  is_remote: boolean;
}

export interface RemoteInfo {
  name: string;
  url: string;
  push_url?: string;
}

export interface TagInfo {
  name: string;
  target: string;
  message?: string;
  tagger?: GitSignature;
  is_annotated: boolean;
}

export interface StashEntry {
  index: number;
  message: string;
  id: string;
}

export interface SubmoduleInfo {
  name: string;
  path: string;
  url: string;
  is_initialized: boolean;
  head_id?: string;
  index_id?: string;
  workdir_id?: string;
  ignore: SubmoduleIgnore;
  update: SubmoduleUpdate;
}

export type SubmoduleIgnore = 'None' | 'Untracked' | 'Dirty' | 'All';
export type SubmoduleUpdate = 'Checkout' | 'Rebase' | 'Merge' | 'None' | 'Default';

// ===== Diff Types =====

export interface DiffEntry {
  old_file?: string;
  new_file?: string;
  status: FileStatus;
  hunks: DiffHunk[];
}

export interface DiffHunk {
  old_start: number;
  old_lines: number;
  new_start: number;
  new_lines: number;
  header: string;
  lines: DiffLine[];
}

export interface DiffLine {
  origin: string;
  content: string;
  old_lineno?: number;
  new_lineno?: number;
}

// ===== Blame Types =====

export interface BlameLine {
  line_no: number;
  commit_id: string;
  author: GitSignature;
  summary: string;
  content: string;
}

// ===== Merge/Conflict Types =====

export interface ConflictInfo {
  path: string;
  our_id: string;
  their_id: string;
  ancestor_id?: string;
  our_content?: string;
  their_content?: string;
  ancestor_content?: string;
}

export interface MergeResult {
  success: boolean;
  conflicts: ConflictInfo[];
  auto_committed: boolean;
  message: string;
}

// ===== Options Types =====

export interface CloneOptions {
  branch?: string;
  depth?: number;
  recursive?: boolean;
  bare?: boolean;
  single_branch?: boolean;
}

export interface PushOptions {
  remote: string;
  force?: boolean;
  set_upstream?: boolean;
}

export interface FetchOptions {
  remote: string;
  prune?: boolean;
  tags?: boolean;
}

// ===== Statistics Types =====

export interface RepoStats {
  commit_count: number;
  branch_count: number;
  tag_count: number;
  stash_count: number;
  contributor_count: number;
  size_bytes: number;
}

export interface ContributorStats {
  name: string;
  email: string;
  commit_count: number;
  additions: number;
  deletions: number;
}

// ===== API Response Types =====

export interface GitApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ===== Credential Types =====

export interface SshKeyCredential {
  type: 'ssh_key';
  username: string;
  private_key: string;
  passphrase?: string;
}

export interface SshAgentCredential {
  type: 'ssh_agent';
  username: string;
}

export interface HttpsCredential {
  type: 'https';
  username: string;
  password: string;
}

export interface TokenCredential {
  type: 'token';
  token: string;
}

export type CredentialType =
  | SshKeyCredential
  | SshAgentCredential
  | HttpsCredential
  | TokenCredential;

// ===== Event Types =====

export interface GitOperationEvent {
  operation: 'clone' | 'fetch' | 'push' | 'pull' | 'commit' | 'merge';
  progress?: number;
  message: string;
  completed: boolean;
  error?: string;
}
