// 窗口尺寸策略: 菜单/打开页用紧凑小窗, 冲突处理切换大窗
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

/** 紧凑小窗(打开页 / 菜单指令面板) */
export async function compactWindow() {
  const win = getCurrentWindow();
  await win.setMinSize(new LogicalSize(380, 520));
  await win.setSize(new LogicalSize(420, 640));
  await win.center();
}

/** 大窗(冲突列表 / 三栏合并) */
export async function largeWindow() {
  const win = getCurrentWindow();
  await win.setMinSize(new LogicalSize(960, 640));
  await win.setSize(new LogicalSize(1280, 800));
  await win.center();
}
