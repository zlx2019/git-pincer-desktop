// 与 Rust 壳层的 IPC 契约: 类型定义、命令封装与事件订阅
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/** 进行中的合并类操作 */
export type Op = 'merge' | 'rebase' | 'cherry-pick' | 'revert';

/** 仓库概要 */
export interface RepoInfo {
  root: string;
  op: Op | null;
  yoursLabel: string;
  theirsLabel: string;
  dirty: number;
}

/** 可从菜单发起的操作 */
export type LaunchKind = 'pull' | 'merge' | 'rebase' | 'cherry-pick' | 'revert';

/** 发起操作的结果 */
export type LaunchOutcome =
  | { kind: 'cleanDone' }
  | { kind: 'conflicts'; files: FileRow[] }
  | { kind: 'failed'; message: string };

/** 本地分支 */
export interface Branch {
  name: string;
  current: boolean;
}

/** 供选择的提交 */
export interface CommitInfo {
  sha: string;
  subject: string;
  /** 来源分支名(cherry-pick 场景; 当前分支历史为空串) */
  branch: string;
}

/** chunk 来源侧 */
export type ChunkKind = 'ours' | 'theirs' | 'agree' | 'conflict';

/** chunk 着色形态 */
export type ChunkVisual = 'added' | 'deleted' | 'modified' | 'conflict';

/** 三栏快照中的一个 chunk; 行区间 [start, end), 强调为 [chunk内行, utf16起, utf16止] */
export interface MergeChunk {
  id: number;
  kind: ChunkKind;
  visual: ChunkVisual;
  leftRange: [number, number];
  resultRange: [number, number];
  rightRange: [number, number];
  leftEmphasis: [number, number, number][];
  rightEmphasis: [number, number, number][];
}

/** 三栏合并快照(Rust 引擎输出) */
export interface MergeSnapshot {
  path: string;
  left: string;
  result: string;
  right: string;
  chunks: MergeChunk[];
  changes: number;
  conflicts: number;
}

/** 冲突文件一侧的状态 */
export type SideStatus = 'modified' | 'deleted' | 'added';

/** Conflicts 列表的一行 */
export interface FileRow {
  path: string;
  yours: SideStatus;
  theirs: SideStatus;
  binary: boolean;
}

/** 三方内容 */
export interface ThreeWay {
  base: string;
  ours: string;
  theirs: string;
}

/** continue 一轮的结果 */
export type RoundOutcome =
  | { kind: 'done' }
  | { kind: 'nextRound'; files: FileRow[] }
  | { kind: 'failed'; message: string };

/** continue 过程的一行输出 */
export interface OutputLine {
  stream: 'stdout' | 'stderr';
  line: string;
}

/** 壳层命令封装 */
export const api = {
  repoOpen: (path?: string) => invoke<RepoInfo>('repo_open', { path: path ?? null }),
  conflicts: () => invoke<FileRow[]>('conflicts'),
  readThree: (path: string) => invoke<ThreeWay>('read_three', { path }),
  openMerge: (path: string) => invoke<MergeSnapshot>('open_merge', { path }),
  acceptSide: (paths: string[], side: 'yours' | 'theirs') =>
    invoke<void>('accept_side', { paths, side }),
  saveResult: (path: string, text: string) => invoke<void>('save_result', { path, text }),
  continueOp: () => invoke<RoundOutcome>('continue_op'),
  abortOp: () => invoke<void>('abort_op'),
  recentRepos: () => invoke<string[]>('recent_repos'),
  recentRemove: (path: string) => invoke<string[]>('recent_remove', { path }),
  launchOp: (kind: LaunchKind, targets: string[]) =>
    invoke<LaunchOutcome>('launch_op', { kind, targets }),
  branches: () => invoke<Branch[]>('branches'),
  switchBranch: (name: string) => invoke<void>('switch_branch', { name }),
  commits: (othersOnly: boolean, limit = 30) =>
    invoke<CommitInfo[]>('commits', { othersOnly, limit }),
  /** 订阅 continue 的输出流 */
  onOutput: (cb: (l: OutputLine) => void): Promise<UnlistenFn> =>
    listen<OutputLine>('git://output', (e) => cb(e.payload)),
};

/** 标题片段(bold 的片段渲染为粗体) */
export interface TitleSegment {
  text: string;
  bold?: boolean;
}

/** 列表页标题: "Merging branch X into branch Y" 等, 措辞随操作类型 */
export function opTitleSegments(info: RepoInfo): TitleSegment[] {
  const yours = { text: info.yoursLabel, bold: true };
  const theirs = { text: info.theirsLabel, bold: true };
  switch (info.op) {
    case 'merge':
      return [{ text: 'Merging branch ' }, theirs, { text: ' into branch ' }, yours];
    case 'rebase':
      return [{ text: 'Rebasing branch ' }, theirs, { text: ' onto ' }, yours];
    case 'cherry-pick':
      return [{ text: 'Cherry-picking commit ' }, theirs, { text: ' into branch ' }, yours];
    case 'revert':
      return [{ text: 'Reverting commit ' }, theirs, { text: ' on branch ' }, yours];
    default:
      return [{ text: 'No merge operation in progress' }];
  }
}
