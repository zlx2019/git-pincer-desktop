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

/** 入缓冲后的终端条目: 附单调 id 作 {#each} 的稳定 key——
    封顶裁剪的 splice 会移位索引, 按索引 key 会让整个可见列表重渲 */
export interface TermRow extends TermEntry {
  id: number;
}

/** 菜单页终端缓冲(会话级): 列表页的操作结局(continue 完成 / abort)也汇入这里 */
export const term = $state({ entries: [] as TermRow[] });

/** 终端缓冲上限: 超限丢最旧的一批(留余量, 摊薄触发频率), DOM 不随长会话无界增长 */
const TERM_LIMIT = 2000;
const TERM_KEEP = 1800;

let termSeq = 0;

/** 追加终端条目并执行上限裁剪(高频来源请配合 rAF 合帧批量调用) */
export function pushTerm(...batch: TermEntry[]) {
  term.entries.push(...batch.map((e) => ({ ...e, id: termSeq++ })));
  if (term.entries.length > TERM_LIMIT) {
    term.entries.splice(0, term.entries.length - TERM_KEEP);
  }
}
