# git-pincer-desktop 技术方案 (v3)

> 定位: **独立的 Git 冲突解决桌面工具, 功能与 CLI 版几乎一致**。功能参照 git-pincer CLI 与 IDEA, 代码全新实现, **不依赖 CLI 源码**, 不改动 git-pincer 仓库。
> 界面 1:1 复刻 IDEA: **Conflicts 文件列表**(参考图 1) + **Merge Revisions 三栏合并**(参考图 2)。所有操作可用鼠标完成。
> v3 (2026-07-10): 按 Zero 重新对齐范围, 恢复操作发起器——打开仓库有冲突直接接管, 无冲突显示菜单首页; 菜单可发起五种操作, 一旦产生冲突立刻接管。

## 1. 功能范围

**只做 7 件事**

1. 打开仓库 (目录选择 / 拖拽文件夹进窗口 / 记住上次): 有冲突直接接管进列表, 无冲突进菜单首页;
2. 菜单首页 (对齐 CLI): **pull / merge / rebase / cherry-pick / revert**, 输出实时流入控制台, 产生冲突立刻接管; 在终端发起的操作靠窗口聚焦重探同样被接管;
3. 冲突文件列表: Accept Yours / Accept Theirs (支持多选) / Merge...;
4. 三栏合并编辑器 (核心界面);
5. 二进制文件 pick-one;
6. 全部解决后一键 Continue (`git <op> --continue`), 多轮 rebase 自动回列表;
7. Abort (带确认对话框)。

**明确不做**

- log / commit / push 等冲突流程之外的 git 客户端功能 (例外, 2026-07-11 Zero 定: 菜单顶栏分支 chip 可切换本地分支, 服务于操作发起, 不再外扩);
- CLI 的 `file` 单文件模式、i18n、配置系统 (硬编码全局 IDEA New UI **暗色**主题; 大窗文案用 IDEA 英文原文, 小窗辅助文案中文);
- 键盘驱动的操作流 (仅保留 ⌘Z / ⌘⇧Z / ⌘⏎ / Esc 等常规编辑键)。

## 2. 架构: 计算在 Rust, 交互状态在前端

Rust 壳不持业务状态; **diff/分块引擎放 Rust(`similar`, 2026-07-10 Zero 定)**, `open_merge` 是纯查询; chunk 的交互状态(应用/忽略/撤销)与文档模型留在前端。

```
┌ 前端 (SvelteKit + CodeMirror 6) ─────────────────────────────┐
│  三栏视图 + chunk 交互状态 + 统一撤销栈  ← 本地, 点击零延迟    │
│            invoke ↓↑ events                                  │
├ src-tauri (无业务状态) ───────────────────────────────────────┤
│  merge.rs 合并引擎(similar): 行 diff / 分块 / 词级强调 → 快照  │
│  repo.rs  git 管道: 读三方 / 写结果+add / launch / continue…  │
├ git 二进制 (继承用户 credentials/hooks/rerere 配置) ──────────┤
```

**分工理由**

- diff 放 Rust: `similar` 成熟且与 CLI 同库同参(Myers + 500ms deadline), 纯函数可单测, 免引 JS diff 依赖;
- 交互状态留前端: 点击 / 悬停 / 撤销零 IPC 往返; 中栏 chunk 区间存前端 `resultRanges` 数组, 在 CM6 updateListener 里用 `changes.mapPos` 手动 remap(实现定稿, 非 RangeSet 自动映射);
- 快照一次性下发(词级强调按 **UTF-16 偏移**, 与 CM6 文档坐标一致), 之后的 chunk 操作全部本地。

**安全策略** (功能上借鉴 CLI, 独立实现): git 参数永远按数组传递不经 shell; continue 类命令注入 `GIT_EDITOR=true`; 清洗宿主 `GIT_DIR` 类环境变量防嵌套劫持。

## 3. Rust 壳命令 (全部 `async` + `spawn_blocking`)

| 命令 | 实现要点 |
|---|---|
| `repo_open(path?) → RepoInfo` | `rev-parse --show-toplevel`; 探测 `MERGE_HEAD` / `rebase-merge` / `CHERRY_PICK_HEAD` / `REVERT_HEAD` 定操作类型; 双方标签 = 当前分支名 + 对方分支/short-sha (标题栏 "Merging branch X into Y" 用) |
| `conflicts() → FileRow[]` | `git ls-files -u` 聚合 stage; 缺 stage 推导 Modified / Deleted / Added; blob 首 8KB NUL 嗅探标记 binary |
| `read_three(path) → {base, ours, theirs}` | `git show :1:p` `:2:p` `:3:p`; 缺失 stage 返回空串 |
| `accept_side(paths[], side)` | `git checkout --ours/--theirs --` + `git add`; 取"删除侧"时 `git rm --cached` + 删工作区文件 |
| `save_result(path, text)` | 写工作区文件 + `git add` (三栏 Apply / binary 选择共用) |
| `continue_op()` | `git <op> --continue` 管道捕获, stdout/err 逐行发事件; 退出非零且仍有冲突 = 新一轮 (rebase), 否则报错 |
| `abort_op()` | `git <op> --abort` (rebase 用 `--abort`, merge 用 `merge --abort`) |
| `recent_repos() → path[]` | 最近打开列表 (tauri app-data 存 JSON); 目录选择器由前端直接走 tauri-plugin-dialog, 不设专门命令 |
| `recent_remove(path) → path[]` | 从最近列表移除一项, 返回过滤后列表 |
| `switch_branch(name)` | `git switch`(菜单顶栏分支 chip, 仅无进行中操作时可用) |
| `launch_op(kind, targets[]) → LaunchOutcome` | 菜单发起五操作; 输出走 `git://output`; 结束探测: 有冲突 → `Conflicts{files}`, 零退出 → `CleanDone`, 否则 `Failed`; 注入 `GIT_TERMINAL_PROMPT=0` 防无终端挂起 |
| `branches() → Branch[]` | 本地分支(带当前标记), merge/rebase 对话框数据 |
| `commits(others_only, limit) → CommitInfo[]` | 最近提交; others_only 只列当前分支未包含的(cherry-pick 场景) |

事件只有一个: `git://output {stream, line}` (launch/continue 的输出流, 发起前订阅、结束即退订)。
结局不走事件——launch/continue 的结果由命令返回值带回 (`LaunchOutcome` / `RoundOutcome`, `tag="kind"` 的 tagged union); 旧方案的 `git://round` / `git://done` 未实现也不再计划。
错误统一 `thiserror` 定义 + serde 序列化, 前端 toast 呈现。

## 4. 合并引擎 (`src-tauri/src/merge.rs`, 纯函数, 已实现)

输入三方全文, 输出 `MergeSnapshot{三栏全文, chunks, 计数}` (经 `open_merge` 命令下发):

1. 两次行级 Myers diff(`similar`): `base→ours`、`base→theirs`, 500ms deadline 守护;
2. **按 base 行区间碰撞归簇**(相触即归并, 刻意保守): 仅单侧 → Ours/Theirs; 双侧相同 → Agree; 双侧不同 → Conflict;
3. 形态分类 (决定着色, 对应参考图 ①~④): 改动侧空 = **deleted 灰** / base 空 = **added 绿** / 其余单侧 = **modified 蓝** / 碰撞 = **conflict 红**;
4. 词级强调: Conflict 比 ours↔theirs、单侧比该侧↔base; 字符级 diff、相邻区间合并, 输出 `[chunk 内行, UTF-16 起, 止]`; 行数 >200 的 chunk 跳过;
5. 守护: 任一侧 >2MB → 降级为"整文件单个 conflict"。分组原则与 CLI 一致: **宁可多报一个冲突, 不静默合错**。

视图状态模型:

- Result 栏初始 = **base 全文** (IDEA 行为: 所有 chunk 初始待处理, 含非冲突项);
- 每个 chunk 的三栏行区间放入 CM6 RangeSet → 用户自由编辑时自动 remap;
- Apply(侧) = 用该侧行替换 Result 中 chunk 当前区间 + 状态记账; Ignore = 仅记账; 均为本地 CM6 事务;
- 撤销分两轨(实现定稿): 手工编辑走 CM6 历史(⌘Z); chunk 操作走独立撤销栈(底栏 Undo 逐步回退, 接缝 ⟲ 整块回初始), 不与文本历史混编——避免 ⌘Z 撤了文本却留下"已应用"状态的脱节;
- 自由编辑命中 chunk 区间 → 该 chunk 记为 resolved-by-edit (计数联动); 落在区间外仅文本生效;
- 计数 `N changes. M conflicts.` 实时由状态推导; conflicts=0 才亮 Apply (仍可强制, 弹确认——IDEA 同行为)。

## 5. 界面还原清单

### 5.0 菜单指令面板 (无冲突时的入口, 对齐 CLI; 样式基准: docs/IDEA_STYLE.md + 根目录 mockup)

**小窗形态** (IDEA New UI 三层灰暗色 #1e1f22/#2b2d30/#393b40): 顶栏仓库/分支 chip → PINCER 品牌行 → 五条指令行(图标 + 英文命令 mono + 中文说明 + **真实 ⌘1–⌘5 快捷键**, hover/执行中显示描述行, Search-Everywhere 蓝 #2e436e 选中) → 底部终端风执行输出(➜ 命令回显、stderr 红、✔/✘ 尾行、橙色闪烁光标、可清空) → 状态栏(分支 + 就绪/执行中)。**PINCER 橙 #ff7a2f 仅用于 logo / 活动 tab 下划线 / 终端光标**。mockup 中的左侧工具 rail 与"终端/Git 日志"假 tab 暂不放(未实现的功能不摆死按钮)。

- merge / rebase → 分支选择对话框(单选, 当前分支置灰, 双击即确认);
- cherry-pick → 提交多选(`--all --not HEAD`, 按旧→新顺序应用); revert → 当前分支提交多选(新→旧应用);
- pull → 直接执行(走当前分支跟踪配置);
- 结果分流: 出冲突 → 自动切大窗进冲突列表; 干净完成/失败 → 终端 ✔/✘ 尾行留痕。

**窗口策略**: 打开页/菜单 = 紧凑小窗(420×640, min 380×520, 可当桌面侧边小工具); 冲突列表/三栏 = 大窗(1280×800, min 960×640); 路由切换时运行时 setSize + center。

### 5.1 Conflicts 文件列表 (参考图 1)

| 元素 | 还原要点 |
|---|---|
| 标题行 | "Merging branch **2023.3** into branch **vb/46**" — 措辞随操作类型 (Rebasing/Cherry-picking...) |
| 表格三列 | Name (文件图标 + 红色文件名 + 灰色目录后缀) / Yours (分支名) / Theirs (分支名) |
| 状态字样 | Modified / Deleted / Added (由 stage 缺失推导) |
| 行为 | 单击选中 (蓝底), ⌘/Shift 多选, **双击 = Merge...** |
| 右侧按钮列 | Accept Yours / Accept Theirs / **Merge...** (蓝主按钮); 删除-修改冲突禁用 Merge... (tooltip 说明), 只能 Accept |
| 二进制行 | Merge... 改弹 pick-one 对话框 (左右预览 + Accept Left/Right) |
| 左下 | "Group files by directory" 复选 → 目录树分组 |
| 右下 | Close; 若操作进行中另有 **Continue** 主按钮 (全部解决后) 与 Abort 次按钮 |
| 无操作空态 | 引导文案(去终端/IDE 发起操作) + Refresh 按钮; **窗口重获焦点自动重探**——外部产生的冲突切回窗口即出现, 列表未变时保留多选状态 |

### 5.2 Merge Revisions 三栏 (参考图 2)

```
┌ 标题: Merge Revisions for /abs/path ────────────────────────────────┐
│ ↑ ↓ │ Apply non-conflicting: ≫Left ⋙All ≪Right │ Highlight words ▾│
│                                              3 changes. 1 conflict. │
├──────────────┬───┬──────────────────┬───┬───────────────────────────┤
│🔒 Changes    │接 │      Result      │接 │  🔒 Changes from 2576     │
│  from vb/24  │缝 │     (可编辑)     │缝 │                           │
│  CM6 只读    │条 │       CM6        │条 │   CM6 只读                │
├──────────────┴───┴──────────────────┴───┴───────────────────────────┤
│ Accept Left  Accept Right                         Cancel  [ Apply ] │
└──────────────────────────────────────────────────────────────────────┘
```

chunk 着色 (布局按参考图 1:1, 配色按 **IDEA Dark diff** 初值, M5 逐像素校准): 蓝 `#385570` 修改 · 绿 `#294436` 新增 · 灰 `#3F4145` 删除 · 红 `#45302B` 冲突; 词级强调同色系加亮一档; applied/ignored 后整块降饱和变暗。

| 组件 | 要点 |
|---|---|
| 三个 CM6 实例 | 侧栏只读; 关软换行 (行高恒定简化几何); 行号; 语法高亮按扩展名加载语言包, 自写 IDEA Dark HighlightStyle |
| chunk 底色 | `Decoration.line`; 中栏区间在前端 `resultRanges` 手动 remap(`changes.mapPos`), 装饰经 Compartment 重建——侧栏按类名签名跳过未变栏, 手工打字只重建中栏 |
| 接缝条 ×2 | **44px**(2026-07-11 定稿, 原 ~28px)竖条内嵌 SVG: 每个可见 chunk 画 侧栏区间→Result区间 贝塞尔封闭带 (同底色, 控制点在两端 30% 处成 IDEA 斜扫); ✕ / ≫ / ≪ 按钮浮于其上; 滚动/编辑 rAF 节流重绘; y 坐标取自 `lineBlockAt`; 按中栏 viewport 粗筛, 屏外 chunk 不算不画 |
| gutter 按钮 | 左栏块 `✕ ≫`、右栏块 `≪ ✕` 镜像; hover 显示 tooltip; applied 后变 undo 图标 |
| 同步滚动 | 以 chunk 三栏区间起始行为锚点分段线性插值 × 行高; 标记驱动源防回声 |
| overview ruler | 最右 ~12px: 全文 chunk 色块缩略(实时区间换算, 编辑后不漂移), 点击按滚动高度比例跳转 |
| 顶栏 | ↑↓ 上/下一个 change (自动滚三栏); ⋙ 组 = 批量应用非冲突 (Left/All/Right); Highlight words ▾ = words/none 切换 |
| 底栏 | Accept Left/Right = 整文件取侧; Cancel = 放弃本文件改动回列表; **Apply** = Result 全文 `save_result` 后回列表刷新 |

### 5.3 鼠标交互全集 (每一步都可点出来)

拖文件夹开仓库 → 菜单点操作(对话框选分支/提交) → 出冲突自动进列表 → 双击行进三栏 → hover chunk 高亮且按钮浮现 → 点 ≫/≪/✕ 处理 → 点接缝色带/ruler 跳转 → 顶栏 ⋙ 一键清非冲突 → 底栏 Apply → 列表 Continue → 完成回菜单。全程零键盘。

## 6. 主题 tokens (全局统一 IDEA New UI 暗色, 2026-07-10 定稿)

`src/lib/theme.css` 集中 CSS 变量(--d-* 系列): 画布 `#1e1f22` · 面板 `#2b2d30` · 分隔线 `#393b40`/`#43454a` · 文本 `#dfe1e5` / 弱化 `#9da0a8` · IDEA 蓝 `#3574f0`(主按钮) · 选中 `#2e436e` · 绿 `#5fad65` · 红 `#e46962` · 琥珀 `#d9a343`("进行中"语义色, 如菜单状态栏) · **PINCER 橙 `#ff7a2f`(仅 logo / 活动 tab 下划线 / 终端光标)** + §5.2 四个 chunk 暗色。等宽字体 JetBrains Mono Regular (OFL 内嵌于 `static/fonts/`, 2026-07-11 落地), UI 字体系统栈。

## 7. 里程碑 (不含任何 git-pincer 仓库改动)

| # | 内容 | 验收 |
|---|---|---|
| M1 | Rust 壳 9 命令 + 打开仓库 + Conflicts 列表页完整 (Accept/多选/分组/删除冲突/binary 对话框) | 对照图 1; playground 仓库 merge 场景 |
| M2 | 合并引擎 (Rust, merge.rs) + 三栏只读: 着色/词级/行号/语法高亮/同步滚动/ruler | 对照图 2 静态形态 |
| M3 | 全部交互: gutter/接缝/apply/ignore/undo/apply-all/自由编辑/计数/Apply&Cancel | 单文件 merge 闭环 |
| M4 | Continue 事件流 + 多轮 rebase + abort (结局统一回菜单终端留痕, 无单独完成页) | playground 六场景全通 |
| M5 | 像素校准 (与截图对比)、打包 (dmg / nsis+msi / AppImage+deb+rpm) + GitHub Actions | 四平台矩阵产物可装 |

测试数据借用 git-pincer 的 `cargo run --example playground` 生成 /tmp/git-pincer-playground (只造数据, 非代码依赖); 引擎的分组/降级逻辑用 Rust 单测(merge.rs 模块尾)。

**进度 (2026-07-10)**: M1–M4 全部完成(菜单化 + 暗色小窗/大窗切换 + 三栏全交互 + continue 循环); M5 的打包配置/图标/CI(ci.yml + release.yml + cargo-deny)已就绪, **像素级校准依赖 Zero 的截图反馈持续进行**。快捷键: 合并页 F7/⇧F7 导航、⌘⏎ Apply、Esc 返回、Highlight words 开关; 菜单页 ⌘1–⌘5。

## 8. 依赖清单 (2026-07-11 Zero 定: 合理需要的新依赖可直接引入, 不再逐项报批)

| 侧 | 依赖 | 用途 |
|---|---|---|
| Rust | `tauri-plugin-dialog = "2"` | 选仓库目录 |
| Rust | `thiserror = "2"` | 壳层错误类型 |
| Rust | `similar = "3.1"` | 合并引擎行/字符级 diff(与 CLI 同库) |
| npm | `@codemirror/state` `view` `language` `language-data` + `@lezer/highlight` | 三栏编辑器; 语言包经 language-data 按需动态加载 |
| npm | `vitest` (dev) | 前端纯逻辑单测(`src/lib/chunks.ts`) |
| npm | 无 UI 组件库、无 JS diff 库 | 1:1 手写 CSS; diff 全在 Rust(架构决定, 保持有效) |

## 9. 风险

- 超大输入/病态 diff → Rust 侧 500ms deadline + 任一侧 >2MB 整文件降级守护兜底(词级强调另设 200 行 / 1000 字符上限);
- 接缝 SVG 与滚动帧同步 → rAF 节流 + viewport 粗筛, 兜底直角折线;
- webkit2gtk (Linux) 字体度量差异 → M5 专项校准;
- 删除-修改、双添加等边角 stage 组合 → M1 用 playground 全场景覆盖。

## 10. 默认决定 (不同意随时改)

- Conflicts 与三栏为**单窗口内路由页**切换, 不开子窗口(小窗↔大窗靠运行时改尺寸);
- **全局统一 IDEA New UI 暗色**(2026-07-10 Zero 拍板): 冲突大窗布局按参考图 1:1、配色换 IDEA Dark、文案英文; 菜单小窗带中文说明;
- 撤销只在会话内有效, Apply 落盘后不可撤 (可重新 checkout 制造冲突, 不在本工具内做)。
