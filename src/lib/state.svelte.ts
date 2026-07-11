// 跨页面共享的会话状态: 当前仓库概要、冲突文件列表与菜单终端缓冲
import type { FileRow, RepoInfo } from './api';

/** 全局会话(打开页写入, 列表页/合并页消费) */
export const session = $state({
  info: null as RepoInfo | null,
  files: [] as FileRow[],
  /** 操作进行中用户主动关闭了冲突页(搁置): 抑制自动接管, 菜单顶部显示恢复横幅;
      现场本身在 git 仓库里, 恢复 = 重新进入冲突页推导。op 结束或用户恢复时清除 */
  parked: false,
});

/** 终端条目: 命令回显 / 标准输出 / 错误输出 / 成功尾行 / 失败尾行 */
export interface TermEntry {
  kind: 'cmd' | 'out' | 'err' | 'ok' | 'fail';
  text: string;
}

/** 菜单页终端缓冲(会话级): 列表页的操作结局(continue 完成 / abort)也汇入这里 */
export const term = $state({ entries: [] as TermEntry[] });
