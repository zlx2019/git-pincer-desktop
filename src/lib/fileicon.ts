// 文件类型图标(对齐 IDEA 列表的按类型着色): 扩展名 → 字母徽章/图形 SVG + 品类色。
// 颜色只复用既有主题 token(低饱和, 亮色主题自动翻转, 不抢冲突红文件名的焦点);
// 未覆盖的扩展名与二进制回落通用文档图标。纯模块, 可 node 单测。

/** 单枚文件图标: 内联 SVG 字面量 + 品类色(CSS 变量表达式) */
export interface FileIcon {
  svg: string;
  color: string;
}

/** 字母徽章(圆角框 + 1–2 字母), 视觉基线与菜单页手写线条图标一致 */
function badge(label: string): string {
  const size = label.length > 1 ? 6.2 : 7.5;
  return (
    `<svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">` +
    `<rect x="1" y="2.5" width="14" height="11" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.1"/>` +
    `<text x="8" y="10.9" text-anchor="middle" font-size="${size}" font-weight="600" ` +
    `font-family="ui-monospace, monospace" fill="currentColor">${label}</text></svg>`
  );
}

/** 图片类: 山形缩略图 */
const IMAGE_SVG =
  `<svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true" fill="none" stroke="currentColor" ` +
  `stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round">` +
  `<rect x="1.5" y="2.5" width="13" height="11" rx="2"/><circle cx="5.4" cy="6.4" r="1.1"/>` +
  `<path d="m3.2 12 3.4-3.8 2.4 2.8 2-2.1 1.8 3"/></svg>`;

/** 通用文档(未知扩展名/二进制兜底, 造型同旧版通用图标) */
const GENERIC_SVG =
  `<svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true">` +
  `<path fill="none" stroke="currentColor" d="M3.5 1.5h6l3 3v10h-9zM9.5 1.5v3h3"/></svg>`;

/** 扩展名 → [徽章字母, 品类色] */
const BADGES: Record<string, [string, string]> = {
  rs: ['RS', 'var(--d-amber)'],
  ts: ['TS', 'var(--d-blue-lt)'],
  tsx: ['TS', 'var(--d-blue-lt)'],
  js: ['JS', 'var(--d-blue-lt)'],
  mjs: ['JS', 'var(--d-blue-lt)'],
  cjs: ['JS', 'var(--d-blue-lt)'],
  svelte: ['SV', 'var(--d-red)'],
  vue: ['V', 'var(--d-green)'],
  json: ['{}', 'var(--d-desc)'],
  toml: ['T', 'var(--d-desc)'],
  yaml: ['Y', 'var(--d-desc)'],
  yml: ['Y', 'var(--d-desc)'],
  lock: ['LK', 'var(--d-desc)'],
  md: ['MD', 'var(--d-dim)'],
  txt: ['TX', 'var(--d-dim)'],
  html: ['<>', 'var(--d-red)'],
  xml: ['<>', 'var(--d-red)'],
  css: ['#', 'var(--d-blue-lt)'],
  scss: ['#', 'var(--d-blue-lt)'],
  less: ['#', 'var(--d-blue-lt)'],
  py: ['PY', 'var(--d-green)'],
  go: ['GO', 'var(--d-blue-lt)'],
  java: ['J', 'var(--d-amber)'],
  kt: ['KT', 'var(--d-amber)'],
  c: ['C', 'var(--d-blue-lt)'],
  h: ['H', 'var(--d-blue-lt)'],
  cpp: ['C+', 'var(--d-blue-lt)'],
  hpp: ['H+', 'var(--d-blue-lt)'],
  sh: ['SH', 'var(--d-dimmer)'],
  bash: ['SH', 'var(--d-dimmer)'],
  zsh: ['SH', 'var(--d-dimmer)'],
};

/** 图片扩展名(共用山形图标) */
const IMAGES = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico', 'bmp', 'avif']);

/** 取路径末段的小写扩展名; 无点(如 Makefile)返回空串 */
function extOf(path: string): string {
  const name = path.slice(path.lastIndexOf('/') + 1);
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
}

/** 路径 → 文件图标(二进制且扩展名未覆盖时与未知一样走通用兜底) */
export function fileIcon(path: string): FileIcon {
  const ext = extOf(path);
  const hit = BADGES[ext];
  if (hit) return { svg: badge(hit[0]), color: hit[1] };
  if (IMAGES.has(ext)) return { svg: IMAGE_SVG, color: 'var(--d-green)' };
  return { svg: GENERIC_SVG, color: 'var(--d-dim)' };
}
