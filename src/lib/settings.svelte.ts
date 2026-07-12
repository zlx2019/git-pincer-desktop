// 用户设置 store: 启动读一次(root layout), 之后即改即存;
// 生效方式 = 写 documentElement 的 CSS 变量(--editor-font-size / --editor-font-family),
// 三栏编辑器在进入 /merge 时新建, 天然拿到当次变量值, 无需热更新已存在的编辑器
import { api, type Settings } from './api';

/** 出厂默认(与 Rust `Settings::default()` 一致; "恢复默认"按钮用——
    窗口尺寸字段归 Rust 独占, set_settings 会忽略这里的 null, 恢复默认不清尺寸记忆) */
export const DEFAULT_SETTINGS: Settings = {
  editorFontSize: 12,
  editorFontFamily: '',
  closeBehavior: 'tray',
  highlightWords: true,
  theme: 'dark',
  language: 'zh',
  compactSize: null,
  largeSize: null,
};

/** 内嵌等宽字体(随应用分发, 见 theme.css @font-face; 首项为默认, 存储值 '' 表示它) */
export const EMBEDDED_FONTS = ['JetBrains Mono', 'Maple Mono'] as const;

/** 全局设置状态(所有页面共享) */
export const settings = $state({ value: { ...DEFAULT_SETTINGS } });

/** 设置对话框开关(渲染在根布局, 任何页面可弹: 齿轮 / macOS 应用菜单 / ⌘,) */
export const settingsUi = $state({ open: false });

let loaded = false;

/** 启动加载(root layout onMount 调一次); 命令不可用(测试/mock 环境)时按默认运行 */
export async function loadSettings() {
  if (loaded) return;
  loaded = true;
  try {
    const s = await api.getSettings();
    if (s) settings.value = s;
  } catch {
    // 保持默认
  }
  applyToDom();
}

/** 改动并持久化: 立即生效, 以 Rust 归一化结果(钳制字号/清洗字体名)回同步;
    落盘失败不回滚内存值——本次会话仍生效, 只是下次启动回旧值 */
export async function updateSettings(patch: Partial<Settings>) {
  settings.value = { ...settings.value, ...patch };
  applyToDom();
  try {
    const s = await api.setSettings(settings.value);
    if (s) {
      settings.value = s;
      applyToDom();
    }
  } catch {
    // 见函数注释
  }
}

/** 编辑器字体就位保障(进 /merge 建编辑器前 await):
    CM6 创建时测量字符宽度, 内嵌字体(如 Maple Mono)后到会导致度量错位;
    系统字体/未知字体名无对应 @font-face, load() 立即空解析, 不会卡住 */
export async function ensureEditorFont() {
  const fam = settings.value.editorFontFamily.trim() || EMBEDDED_FONTS[0];
  try {
    await document.fonts.load(`12px "${fam}"`);
  } catch {
    // 字体名非法等异常: 按就绪处理, 编辑器走回落字体
  }
}

/** 设置 → DOM: 编辑器 CSS 变量(不动全局 --font-mono, logo/终端不受影响)
    + 主题 data 属性(theme.css 的 html[data-theme=light] 整体翻转 tokens) */
function applyToDom() {
  const st = document.documentElement.style;
  st.setProperty('--editor-font-size', `${settings.value.editorFontSize}px`);
  const fam = settings.value.editorFontFamily.trim();
  if (fam) st.setProperty('--editor-font-family', `"${fam}", var(--font-mono)`);
  else st.removeProperty('--editor-font-family');
  document.documentElement.dataset.theme = settings.value.theme;
}
