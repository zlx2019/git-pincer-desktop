// merge 页的纯 chunk 逻辑(无 CM6 视图/DOM 依赖): 状态机、导航、批量应用、文本组装。
// 独立成模块以便 vitest 单测; 页面组件只做 CM6 编排与状态持有。
import type { EditorState } from '@codemirror/state';
import type { MergeChunk } from './api';

/** 栏位标识 */
export type Pane = 'left' | 'result' | 'right';

/** 一侧的处理状态 */
export type SideState = 'pending' | 'applied' | 'ignored';

/** 一个 chunk 的交互状态 */
export interface ChunkState {
  ours: SideState;
  theirs: SideState;
  edited: boolean;
}

/** 该 chunk 需要处理的侧 */
export function relevantSides(c: MergeChunk): ('ours' | 'theirs')[] {
  if (c.kind === 'ours' || c.kind === 'agree') return ['ours'];
  if (c.kind === 'theirs') return ['theirs'];
  return ['ours', 'theirs'];
}

/** chunk 是否已解决(手工编辑或相关侧全部处理) */
export function isResolved(c: MergeChunk | undefined, st: ChunkState | undefined): boolean {
  if (!c || !st) return false;
  return st.edited || relevantSides(c).every((s) => st[s] !== 'pending');
}

/** 侧栏底色类名; 未触及该栏(单侧改动的对侧)返回 null 不着色 */
export function paneClass(c: MergeChunk, st: ChunkState, pane: 'left' | 'right'): string | null {
  if (pane === 'left' ? c.kind === 'theirs' : c.kind === 'ours') return null;
  const done = st.edited || st[pane === 'left' ? 'ours' : 'theirs'] !== 'pending';
  return done ? 'ck-done' : `ck-${c.visual}`;
}

/** 上/下一个目标 id: pool 升序, 越过端点回绕(F7/⇧F7 导航) */
export function navTarget(pool: number[], cur: number, dir: 1 | -1): number {
  return dir > 0
    ? (pool.find((i) => i > cur) ?? pool[0])
    : ([...pool].reverse().find((i) => i < cur) ?? pool[pool.length - 1]);
}

/** 批量应用的目标列表: 非冲突、方向匹配(agree 双方内容一致, 两个方向都放行)、pending 且未手工编辑 */
export function applyAllTargets(
  chunks: MergeChunk[],
  states: ChunkState[],
  direction: 'left' | 'all' | 'right'
): { id: number; side: 'ours' | 'theirs' }[] {
  const out: { id: number; side: 'ours' | 'theirs' }[] = [];
  for (const c of chunks) {
    if (c.kind === 'conflict') continue;
    if (direction === 'left' && c.kind === 'theirs') continue;
    if (direction === 'right' && c.kind === 'ours') continue;
    const side = c.kind === 'theirs' ? 'theirs' : 'ours';
    if (states[c.id][side] === 'pending' && !states[c.id].edited) out.push({ id: c.id, side });
  }
  return out;
}

/** 区间替换文本组装: 维持行边界(区间尾不在文末时补尾换行) */
export function joinedText(lines: string[], to: number, docLen: number): string {
  if (!lines.length) return '';
  return lines.join('\n') + (to < docLen ? '\n' : '');
}

/** applySide 的文档变更计划: 首侧 = 区间替换; 已有一侧应用 = 追加到区间尾
    (keep both, 语义同 CLI 的 take order; 区间尾即文末时前补换行避免同行粘连) */
export function applyEdit(
  lines: string[],
  r: { from: number; to: number },
  bothApplied: boolean,
  docLen: number
): { from: number; to: number; insert: string } {
  if (!bothApplied) return { from: r.from, to: r.to, insert: joinedText(lines, r.to, docLen) };
  let insert = joinedText(lines, r.to, docLen);
  if (insert && r.to >= docLen && docLen > 0) insert = '\n' + insert;
  return { from: r.to, to: r.to, insert };
}

/** 该 chunk 在指定栏位的行区间; 未触及该栏(单侧改动的对侧)返回 null 不着色 */
export function paneRange(c: MergeChunk, pane: Pane): [number, number] | null {
  if (pane === 'left') return c.kind === 'theirs' ? null : c.leftRange;
  if (pane === 'right') return c.kind === 'ours' ? null : c.rightRange;
  return c.resultRange;
}

/** 行区间 → 文档位置区间: from = 首行行首, to = 末行下一行行首(区间尾为文末则取文末) */
export function lineRangeToPos(
  doc: EditorState['doc'],
  range: [number, number]
): { from: number; to: number } {
  const [s, e] = range;
  const from = s < doc.lines ? doc.line(s + 1).from : doc.length;
  const to = e <= s ? from : e < doc.lines ? doc.line(e + 1).from : doc.length;
  return { from, to };
}
