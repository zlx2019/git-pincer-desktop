// 界面文案词典(纯模块, 无运行时依赖, 可 node 单测)。
// 语义: zh = 分层设计(大窗 IDEA 英文原文 + 小窗中文辅助), en = 全英文——
// 因此词典只覆盖"小窗/辅助"文案; 大窗(Conflicts/三栏)的 IDEA 原文不进词典。
// 条目为 [zh, en]; "{n}" 是唯一占位符(数字或字符串)。
import type { Language } from './api';

/** [zh, en] 词条表 */
export const dict = {
  // 打开页
  'open-hint': ['…或把仓库文件夹拖进这个窗口', '…or drop a repository folder onto this window'],

  // 菜单: 品牌行 / 指令行
  'brand-sub': ['指令面板', 'Command palette'],
  'act-pull': ['拉取远端', 'Pull remote'],
  'actd-pull': ['从跟踪的远端拉取最新提交并合并', 'Fetch from the tracked remote and merge'],
  'act-merge': ['合并分支', 'Merge branch'],
  'actd-merge': ['选择一个分支合并进当前分支', 'Merge a branch into the current one'],
  'act-rebase': ['变基分支', 'Rebase branch'],
  'actd-rebase': ['将当前分支变基到目标分支之上', 'Rebase the current branch onto a target'],
  'act-cherry-pick': ['摘取提交', 'Cherry-pick'],
  'actd-cherry-pick': [
    '从其他分支摘取提交应用到当前分支',
    'Apply commits from other branches onto this one',
  ],
  'act-revert': ['撤销提交', 'Revert commits'],
  'actd-revert': ['生成反向提交, 撤销所选提交的改动', 'Create inverse commits undoing the selected ones'],

  // 菜单: 搁置恢复横幅
  'resume-state': ['进行中 · 已搁置', 'in progress · parked'],
  'resume-pending': ['{n} 个冲突待解决 · 点击恢复', '{n} conflict(s) unresolved · click to resume'],
  'resume-done': ['冲突已全部解决, 点击继续 (continue)', 'All conflicts resolved — click to continue'],

  // 菜单: 对话框标题
  'dlg-merge': ['合并分支 → {n}', 'Merge a branch → {n}'],
  'dlg-rebase': ['变基 {n} 到目标分支', 'Rebase {n} onto…'],
  'dlg-cherry-pick': ['摘取提交(可多选)', 'Cherry-pick commits (multi-select)'],
  'dlg-revert': ['撤销提交(可多选)', 'Revert commits (multi-select)'],
  'dlg-switch': ['切换分支', 'Switch branch'],

  // 菜单: 终端与状态栏
  'term-tab': ['执行输出', 'Output'],
  'term-done': ['✔ 完成 · 用时 {n}s', '✔ done · {n}s'],
  'sb-running': ['● 执行中', '● running'],
  'sb-oping': ['● {n} 进行中', '● {n} in progress'],
  'sb-ready': ['● 就绪', '● ready'],

  // 冲突列表页(大窗英文, 仅辅助 tooltip 进词典)
  'park-title': [
    '暂时关闭, 操作与已解决进度保留, 可从菜单恢复',
    'Close for now — progress is kept, resume from the menu',
  ],

  // 设置对话框
  'set-title': ['设置', 'Settings'],
  'set-theme': ['主题', 'Theme'],
  'set-theme-dark': ['深色', 'Dark'],
  'set-theme-light': ['浅色', 'Light'],
  'set-lang': ['语言', 'Language'],
  'set-font-size': ['编辑器字号', 'Editor font size'],
  'set-font-family': ['编辑器字体', 'Editor font family'],
  'set-font-ph': ['JetBrains Mono (内嵌)', 'JetBrains Mono (embedded)'],
  'set-close': ['关闭窗口时', 'On window close'],
  'set-close-tray': ['收进托盘', 'Hide to tray'],
  'set-close-quit': ['退出应用', 'Quit app'],
  'set-words': ['词级强调默认开启', 'Highlight words by default'],
  'set-reset': ['恢复默认', 'Reset to defaults'],
  'set-hint': ['即改即存 · 保存在本机', 'Saved locally as you change'],
  'set-done': ['完成', 'Done'],
  'set-editor-note': [
    '编辑器字号/字体/主题在重新进入合并页后生效',
    'Editor font and theme apply when you re-open the merge view',
  ],
} as const;

/** 词条键 */
export type I18nKey = keyof typeof dict;

/** 纯取词: 按语言取词条并代入占位符 */
export function pick(key: I18nKey, lang: Language, n?: string | number): string {
  const s = dict[key][lang === 'en' ? 1 : 0];
  return n === undefined ? s : s.replace('{n}', String(n));
}
