// 跨页面共享的会话状态: 当前仓库概要、冲突文件列表与菜单终端缓冲
import type { FileRow, RepoInfo } from './api';

/** 全局会话(打开页写入, 列表页/合并页消费) */
export const session = $state({
  info: null as RepoInfo | null,
  files: [] as FileRow[],
});

/** 终端条目: 命令回显 / 标准输出 / 错误输出 / 成功尾行 / 失败尾行 */
export interface TermEntry {
  kind: 'cmd' | 'out' | 'err' | 'ok' | 'fail';
  text: string;
}

/** 菜单页终端缓冲(会话级): 列表页的操作结局(continue 完成 / abort)也汇入这里 */
export const term = $state({ entries: [] as TermEntry[] });
