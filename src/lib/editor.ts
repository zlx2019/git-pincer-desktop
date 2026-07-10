// CM6 三栏渲染基建: IDEA Dark 主题、语法高亮、chunk 装饰、锚点同步滚动
import { EditorState, RangeSetBuilder, type Extension } from '@codemirror/state';
import { Decoration, EditorView, lineNumbers, type DecorationSet } from '@codemirror/view';
import { HighlightStyle, LanguageDescription, syntaxHighlighting } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import { tags as t } from '@lezer/highlight';
import type { MergeChunk } from './api';

/** 栏位标识 */
export type Pane = 'left' | 'result' | 'right';

/** IDEA New UI Dark 编辑器外观 */
export const ideaTheme = EditorView.theme(
  {
    '&': {
      backgroundColor: 'var(--d-canvas)',
      color: 'var(--d-text)',
      height: '100%',
      fontSize: '12px',
    },
    '.cm-scroller': {
      fontFamily: 'var(--font-mono)',
      lineHeight: '1.6',
      overflow: 'auto',
      // 三栏同步滚动, 原生滚动条只剩噪音: 全部隐藏, 导航交给滚轮与 overview ruler
      scrollbarWidth: 'none',
    },
    '.cm-scroller::-webkit-scrollbar': { display: 'none' },
    '.cm-gutters': {
      backgroundColor: 'var(--d-canvas)',
      color: '#6e7178',
      border: 'none',
      borderRight: '1px solid var(--d-border)',
    },
    '.cm-lineNumbers .cm-gutterElement': { padding: '0 8px 0 12px' },
    '&.cm-focused': { outline: 'none' },
  },
  { dark: true }
);

/** IDEA Dark 代码配色(近似初值, M5 逐像素校准) */
export const ideaHighlight = HighlightStyle.define([
  { tag: [t.keyword, t.modifier, t.operatorKeyword], color: '#cf8e6d' },
  { tag: [t.string, t.special(t.string), t.regexp], color: '#6aab73' },
  { tag: [t.comment, t.blockComment], color: '#7a7e85', fontStyle: 'italic' },
  { tag: [t.number, t.bool], color: '#2aacb8' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: '#56a8f5' },
  { tag: [t.typeName, t.className, t.namespace], color: '#c77dbb' },
  { tag: t.propertyName, color: '#c77dbb' },
  { tag: [t.meta, t.annotation], color: '#b3ae60' },
  { tag: t.tagName, color: '#d5b778' },
  { tag: t.attributeName, color: '#c77dbb' },
]);

/** 按文件名匹配语言支持(找不到或加载失败返回空扩展) */
export async function languageFor(path: string): Promise<Extension> {
  const desc = LanguageDescription.matchFilename(languages, path);
  if (!desc) return [];
  try {
    return await desc.load();
  } catch {
    return [];
  }
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

/** 构建 chunk 行底色与词级强调装饰; classFor 决定每个 chunk 的类名(null = 不着色) */
export function buildPaneDecos(
  doc: EditorState['doc'],
  chunks: MergeChunk[],
  pane: Pane,
  classFor: (c: MergeChunk) => string | null,
  resultRanges?: { from: number; to: number }[]
): [DecorationSet, DecorationSet] {
  const lineB = new RangeSetBuilder<Decoration>();
  const markB = new RangeSetBuilder<Decoration>();
  for (const c of chunks) {
    const cls = classFor(c);
    if (!cls) continue;
    // 中栏用实时位置区间(随编辑 remap), 侧栏用快照静态行区间
    let from: number;
    let to: number;
    if (pane === 'result' && resultRanges) {
      ({ from, to } = resultRanges[c.id]);
    } else {
      const range = paneRange(c, pane);
      if (!range) continue;
      ({ from, to } = lineRangeToPos(doc, range));
    }
    for (let pos = from; pos < to && pos < doc.length; ) {
      const line = doc.lineAt(pos);
      lineB.add(line.from, line.from, Decoration.line({ class: cls }));
      pos = line.to + 1;
    }
    if (pane === 'result') continue;
    const range = paneRange(c, pane);
    if (!range) continue;
    const em = pane === 'left' ? c.leftEmphasis : c.rightEmphasis;
    for (const [rel, a, b] of em) {
      const ln = range[0] + rel;
      if (ln >= doc.lines) continue;
      const line = doc.line(ln + 1);
      const mf = Math.min(line.from + a, line.to);
      const mt = Math.min(line.from + b, line.to);
      if (mt > mf) markB.add(mf, mt, Decoration.mark({ class: 'ck-em' }));
    }
  }
  return [lineB.finish(), markB.finish()];
}

/** 创建 pane(扩展由调用方组合) */
export function createPane(parent: HTMLElement, text: string, extensions: Extension[]): EditorView {
  return new EditorView({ state: EditorState.create({ doc: text, extensions }), parent });
}

/** 常用只读侧栏扩展集 */
export function readonlyExtensions(lang: Extension): Extension[] {
  return [
    lineNumbers(),
    EditorView.editable.of(false),
    EditorState.readOnly.of(true),
    ideaTheme,
    syntaxHighlighting(ideaHighlight, { fallback: true }),
    lang,
  ];
}

/** 锚点分段线性同步滚动; 返回解绑函数 */
export function linkScroll(panes: { view: EditorView; anchors: number[] }[]): () => void {
  let syncing = false;
  const handlers: [HTMLElement, () => void][] = [];
  for (let i = 0; i < panes.length; i++) {
    const src = panes[i];
    const el = src.view.scrollDOM;
    const handler = () => {
      if (syncing) return;
      syncing = true;
      const srcLine = el.scrollTop / src.view.defaultLineHeight;
      const seg = locate(src.anchors, srcLine);
      for (let j = 0; j < panes.length; j++) {
        if (j === i) continue;
        const dst = panes[j];
        const dstLine = mapLine(src.anchors, dst.anchors, seg, srcLine);
        dst.view.scrollDOM.scrollTop = dstLine * dst.view.defaultLineHeight;
      }
      requestAnimationFrame(() => (syncing = false));
    };
    el.addEventListener('scroll', handler);
    handlers.push([el, handler]);
  }
  return () => {
    for (const [el, handler] of handlers) el.removeEventListener('scroll', handler);
  };
}

/** 定位 line 所在的锚点段索引 */
function locate(anchors: number[], line: number): number {
  let k = 0;
  while (k + 2 < anchors.length && anchors[k + 1] <= line) k++;
  return k;
}

/** 段内线性插值: 源栏行号 → 目标栏行号 */
function mapLine(a: number[], b: number[], k: number, line: number): number {
  const a0 = a[k];
  const a1 = a[k + 1];
  const b0 = b[k];
  const b1 = b[k + 1];
  if (a1 <= a0) return b0;
  const f = (line - a0) / (a1 - a0);
  return b0 + f * (b1 - b0);
}
