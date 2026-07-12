// 用户设置 store: 启动读一次(root layout), 之后即改即存;
// 生效方式 = 写 documentElement 的 CSS 变量(--editor-font-size / --editor-font-family),
// 三栏编辑器在进入 /merge 时新建, 天然拿到当次变量值, 无需热更新已存在的编辑器
import { api, type Settings } from './api';

/** 出厂默认(与 Rust `Settings::default()` 一致; "恢复默认"按钮用) */
export const DEFAULT_SETTINGS: Settings = {
  editorFontSize: 12,
  editorFontFamily: '',
  closeBehavior: 'tray',
  highlightWords: true,
};

/** 全局设置状态(所有页面共享) */
export const settings = $state({ value: { ...DEFAULT_SETTINGS } });

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

/** 设置 → CSS 变量; 只覆盖编辑器专用变量, 不动全局 --font-mono(logo/终端等不受影响) */
function applyToDom() {
  const st = document.documentElement.style;
  st.setProperty('--editor-font-size', `${settings.value.editorFontSize}px`);
  const fam = settings.value.editorFontFamily.trim();
  if (fam) st.setProperty('--editor-font-family', `"${fam}", var(--font-mono)`);
  else st.removeProperty('--editor-font-family');
}
