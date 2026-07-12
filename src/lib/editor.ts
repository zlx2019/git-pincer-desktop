// CM6 三栏渲染基建: IDEA Dark 主题、语法高亮、chunk 装饰、锚点同步滚动
import { EditorState, RangeSetBuilder, type Extension, type RangeSet } from '@codemirror/state';
import {
  Decoration,
  EditorView,
  GutterMarker,
  gutterLineClass,
  lineNumbers,
  type DecorationSet,
} from '@codemirror/view';
import { HighlightStyle, LanguageDescription, syntaxHighlighting } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import { tags as t } from '@lezer/highlight';
import type { MergeChunk } from './api';
import { lineRangeToPos, paneRange, type Pane } from './chunks';

// 纯几何/区间逻辑在 chunks.ts(可 node 单测), 此处转出保持既有导入路径
export { lineRangeToPos, paneRange, type Pane };

/// 编辑器外观规格: 颜色全部走主题 tokens(亮暗由 html[data-theme] 翻转),
/// 亮暗两个实例只差 CM6 的 dark 标志(决定原生选区/光标等的基础配色)
const themeSpec = {
  '&': {
    backgroundColor: 'var(--d-canvas)',
    color: 'var(--d-text)',
    height: '100%',
    // 字号/字体走设置系统的可覆盖变量(进入 /merge 时编辑器新建, 取当次值)
    fontSize: 'var(--editor-font-size, 12px)',
  },
  '.cm-scroller': {
    fontFamily: 'var(--editor-font-family, var(--font-mono))',
    lineHeight: '1.6',
    overflow: 'auto',
    // 三栏同步滚动, 原生滚动条只剩噪音: 全部隐藏, 导航交给滚轮与 overview ruler
    scrollbarWidth: 'none',
  },
  '.cm-scroller::-webkit-scrollbar': { display: 'none' },
  // 行号槽不设竖分隔线: chunk 行的槽位与内容同色(gutterLineClass), 色带得以穿过行号列(IDEA 行为)
  '.cm-gutters': {
    backgroundColor: 'var(--d-canvas)',
    color: 'var(--d-gutter)',
    border: 'none',
  },
  '.cm-lineNumbers .cm-gutterElement': { padding: '0 8px 0 12px' },
  '&.cm-focused': { outline: 'none' },
};

/** IDEA New UI 暗色编辑器外观 */
export const ideaThemeDark = EditorView.theme(themeSpec, { dark: true });

/** IDEA New UI 亮色编辑器外观 */
export const ideaThemeLight = EditorView.theme(themeSpec, { dark: false });

/** IDEA Dark 代码配色(近似初值, M5 逐像素校准) */
export const ideaHighlightDark = HighlightStyle.define([
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

/** IDEA Light 代码配色(近似初值, M5 逐像素校准) */
export const ideaHighlightLight = HighlightStyle.define([
  { tag: [t.keyword, t.modifier, t.operatorKeyword], color: '#0033b3' },
  { tag: [t.string, t.special(t.string), t.regexp], color: '#067d17' },
  { tag: [t.comment, t.blockComment], color: '#8c8c8c', fontStyle: 'italic' },
  { tag: [t.number, t.bool], color: '#1750eb' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: '#00627a' },
  { tag: [t.typeName, t.className, t.namespace], color: '#7a3e9d' },
  { tag: t.propertyName, color: '#871094' },
  { tag: [t.meta, t.annotation], color: '#9e880d' },
  { tag: t.tagName, color: '#0033b3' },
  { tag: t.attributeName, color: '#174ad4' },
]);

/** 按主题取编辑器外观扩展(主题 + 语法高亮) */
export function appearanceExtensions(light: boolean): Extension[] {
  return [
    light ? ideaThemeLight : ideaThemeDark,
    syntaxHighlighting(light ? ideaHighlightLight : ideaHighlightDark, { fallback: true }),
  ];
}

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

/** 行号槽底色标记(复用 chunk 类名, 让色带视觉上穿过行号列) */
class GutterLineMarker extends GutterMarker {
  constructor(cls: string) {
    super();
    this.elementClass = cls;
  }

  override eq(other: GutterLineMarker): boolean {
    return other.elementClass === this.elementClass;
  }
}

const gutterMarkers = new Map<string, GutterLineMarker>();

/** 同类名标记实例复用(RangeSet eq 友好) */
function markerFor(cls: string): GutterLineMarker {
  let m = gutterMarkers.get(cls);
  if (!m) {
    m = new GutterLineMarker(cls);
    gutterMarkers.set(cls, m);
  }
  return m;
}

/** 构建 chunk 行底色、词级强调与行号槽底色; classFor 决定每个 chunk 的类名(null = 不着色)。
    clip 为可选裁剪窗口(文档位置区间): 只为窗口内的行产出装饰——装饰成本从 O(全部 chunk 行)
    降为 O(视口行), 大文件降级为整文件单 chunk 时逐键重建不再卡顿 */
export function buildPaneDecos(
  doc: EditorState['doc'],
  chunks: MergeChunk[],
  pane: Pane,
  classFor: (c: MergeChunk) => string | null,
  resultRanges?: { from: number; to: number }[],
  clip?: { from: number; to: number }
): [DecorationSet, DecorationSet, RangeSet<GutterMarker>] {
  const lineB = new RangeSetBuilder<Decoration>();
  const markB = new RangeSetBuilder<Decoration>();
  const gutterB = new RangeSetBuilder<GutterMarker>();
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
    if (clip) {
      if (to < clip.from || from > clip.to) continue;
      from = Math.max(from, doc.lineAt(Math.min(clip.from, doc.length)).from);
      to = Math.min(to, clip.to);
    }
    // 行号槽只取基础类(去掉 ck-cur 之类的附加态, 避免槽位重复描边)
    const gutterCls = cls.split(' ')[0];
    for (let pos = from; pos < to && pos < doc.length; ) {
      const line = doc.lineAt(pos);
      lineB.add(line.from, line.from, Decoration.line({ class: cls }));
      gutterB.add(line.from, line.from, markerFor(gutterCls));
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
      if (clip && (line.to < clip.from || line.from > clip.to)) continue;
      const mf = Math.min(line.from + a, line.to);
      const mt = Math.min(line.from + b, line.to);
      if (mt > mf) markB.add(mf, mt, Decoration.mark({ class: 'ck-em' }));
    }
  }
  return [lineB.finish(), markB.finish(), gutterB.finish()];
}

/** 创建 pane(扩展由调用方组合) */
export function createPane(parent: HTMLElement, text: string, extensions: Extension[]): EditorView {
  return new EditorView({ state: EditorState.create({ doc: text, extensions }), parent });
}

/** 常用只读侧栏扩展集(light 按设置主题传入) */
export function readonlyExtensions(lang: Extension, light = false): Extension[] {
  return [
    lineNumbers(),
    EditorView.editable.of(false),
    EditorState.readOnly.of(true),
    ...appearanceExtensions(light),
    lang,
  ];
}

/** 锚点分段线性同步滚动; 返回解绑函数。
    程序写入的 scrollTop 逐目标记账: 目标栏随后触发的 scroll 事件是回声, 直接吞掉——
    旧实现用 rAF 重置全局锁, 回声穿过锁窗口后被反向映射, 往返取整造成 1px 级抖动/橡皮感 */
export function linkScroll(panes: { view: EditorView; anchors: number[] }[]): () => void {
  const written = new Map<HTMLElement, number>();
  const handlers: [HTMLElement, () => void][] = [];
  for (let i = 0; i < panes.length; i++) {
    const src = panes[i];
    const el = src.view.scrollDOM;
    const handler = () => {
      const w = written.get(el);
      if (w !== undefined) {
        written.delete(el);
        // 与记账值一致 = 纯回声; 不一致说明用户在该栏另有滚动, 照常转发
        if (Math.abs(el.scrollTop - w) < 1) return;
      }
      const srcLine = el.scrollTop / src.view.defaultLineHeight;
      const seg = locate(src.anchors, srcLine);
      for (let j = 0; j < panes.length; j++) {
        if (j === i) continue;
        const dst = panes[j];
        const dstEl = dst.view.scrollDOM;
        const top = mapLine(src.anchors, dst.anchors, seg, srcLine) * dst.view.defaultLineHeight;
        if (Math.abs(dstEl.scrollTop - top) < 1) continue;
        dstEl.scrollTop = top;
        // 记录写后的实际值(浏览器会按可滚动范围取整/夹取)
        written.set(dstEl, dstEl.scrollTop);
      }
    };
    el.addEventListener('scroll', handler, { passive: true });
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
