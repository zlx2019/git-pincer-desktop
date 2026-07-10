// 跨页面共享的会话状态: 当前仓库概要与冲突文件列表
import type { FileRow, RepoInfo } from './api';

/** 全局会话(打开页写入, 列表页/合并页消费) */
export const session = $state({
  info: null as RepoInfo | null,
  files: [] as FileRow[],
});
