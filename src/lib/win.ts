// 窗口尺寸策略: 菜单/打开页用紧凑小窗, 冲突处理切换大窗。
// 最小尺寸/尺寸/居中在 Rust 侧单命令完成(1 次 IPC, 原先是 3 次串行);
// 形态未变时 Rust 直接跳过, 不会把用户移动/调整过的窗口拽回屏幕中心。
// 返回形态是否真的变了: 页面以此决定要不要播放内容入场淡入(同形态导航硬切)
import { invoke } from '@tauri-apps/api/core';

/** 紧凑小窗(打开页 / 菜单指令面板) */
export function compactWindow(): Promise<boolean> {
  return invoke('set_window_form', { form: 'compact' });
}

/** 大窗(冲突列表 / 三栏合并) */
export function largeWindow(): Promise<boolean> {
  return invoke('set_window_form', { form: 'large' });
}
