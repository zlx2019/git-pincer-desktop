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
- CLI 的 `file` 单文件模式; 语言与主题已入设置(2026-07-12, "配置系统不做"同日由 Zero 推翻): 默认仍是暗色 + 分层文案(大窗英文/小窗中文), 可切亮色/全英文——**大窗 IDEA 英文原文不随语言切换**(1:1 还原基准是英文截图), 全中文大窗未做、如有需要另行定稿;
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
| `continue_op()` | `git <op> --continue` 管道捕获, stdout/err 聚批发事件; 退出非零且仍有冲突 = 新一轮 (rebase), 否则报错 |
| `abort_op()` | `git <op> --abort` (rebase 用 `--abort`, merge 用 `merge --abort`) |
| `recent_repos() → {path, missing}[]` | 最近打开列表 (tauri app-data 存 JSON); 目录已删/移动的项标记 missing 置灰保留、点击提示、× 手动移除(IDEA 行为, 2026-07-12 定, 原为自动剔除), 打开页聚焦时重拉; 目录选择器由前端直接走 tauri-plugin-dialog, 不设专门命令 |
| `recent_remove(path) → {path, missing}[]` | 从最近列表移除一项, 返回更新后列表 |
| `switch_branch(name)` | `git switch`(菜单顶栏分支 chip, 仅无进行中操作时可用) |
| `launch_op(kind, targets[]) → LaunchOutcome` | 菜单发起五操作; 输出走 `git://output`; 结束探测: 有冲突 → `Conflicts{files}`, 零退出 → `CleanDone`, 否则 `Failed`; 注入 `GIT_TERMINAL_PROMPT=0` 防无终端挂起 |
| `branches() → Branch[]` | 本地分支(带当前标记), merge/rebase 对话框数据 |
| `commits(others_only, limit) → CommitInfo[]` | 最近提交; others_only 只列当前分支未包含的(cherry-pick 场景) |
| `set_window_form(form)` | 窗口形态(compact/large): 最小尺寸/尺寸/定位单 IPC 完成; AppState 缓存已应用形态, 未变直接跳过; 尺寸 = 记忆值(settings.compactSize/largeSize, 钳到形态最小)或出厂默认; 位置本次运行内记忆, 切回原位恢复(不在任何屏幕上则居中), 首次出现居中 |
| `get_settings() → Settings` | 用户设置(启动时 setup 从 app-data/settings.json 载入内存) |
| `set_settings(settings) → Settings` | 归一化(字号钳 8–32, 字体名清洗) → 内存 → 落盘; 返回归一化结果供前端回同步 |

事件只有一个: `git://output [{stream, line}, …]` (launch/continue 的输出流, 发起前订阅、结束即退订;
载荷是一批行——Rust 侧 repo.rs 按 ≤25ms/≤64 行聚批发送, IPC 频次从每行一次降到每窗口一次, 2026-07-12 定)。
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
- 每个 chunk 的中栏区间存前端 `resultRanges` 数组, 用户自由编辑时在 updateListener 里 `changes.mapPos` 手动 remap(实现定稿, 与 §2 一致);
- Apply(侧) = 用该侧行替换 Result 中 chunk 当前区间 + 状态记账; Ignore = 仅记账; 均为本地 CM6 事务;
- 撤销分两轨(实现定稿): 手工编辑走 CM6 历史(⌘Z); chunk 操作走独立撤销栈(底栏 Undo 逐步回退, 接缝 ⟲ 整块回初始), 不与文本历史混编——避免 ⌘Z 撤了文本却留下"已应用"状态的脱节;
- 自由编辑命中 chunk 区间 → 该 chunk 记为 resolved-by-edit (计数联动); 落在区间外仅文本生效;
- 计数 `N changes. M conflicts.` 实时由状态推导; conflicts=0 才亮 Apply (仍可强制, 弹确认——IDEA 同行为)。

## 5. 界面还原清单

### 5.0 菜单指令面板 (无冲突时的入口, 对齐 CLI; 本节即样式基准——IDEA_STYLE.md 与根目录 mockup 已删, 2026-07-12, 要点并入此处)

**小窗形态** (IDEA New UI 三层灰暗色 #1e1f22/#2b2d30/#393b40): 顶栏仓库/分支 chip → PINCER 品牌行 → 五条指令行(图标 + 英文命令 mono + 中文说明 + **真实 ⌘1–⌘5 快捷键**, hover/执行中显示描述行, Search-Everywhere 蓝 #2e436e 选中) → 底部终端风执行输出(➜ 命令回显、stderr 红、✔/✘ 尾行、橙色闪烁光标、可清空) → 状态栏(分支 + 就绪/执行中)。**PINCER 橙 #ff7a2f 仅用于 logo / 活动 tab 下划线 / 终端光标**。早期 mockup 里的左侧工具 rail 与"终端/Git 日志"假 tab 未实现也不摆(未实现的功能不摆死按钮)。

- merge / rebase → 分支选择对话框(单选, 当前分支置灰, 双击即确认);
- cherry-pick → 提交多选(`--all --not HEAD`, 按旧→新顺序应用); revert → 当前分支提交多选(新→旧应用);
- pull → 直接执行(走当前分支跟踪配置);
- 结果分流: 出冲突 → 自动切大窗进冲突列表; 干净完成/失败 → 终端 ✔/✘ 尾行留痕。

**窗口策略**: 打开页/菜单 = 紧凑小窗(420×640, min 380×520, 可当桌面侧边小工具); 冲突列表/三栏 = 大窗(1280×800, min 960×640); 路由切换时前端调 `set_window_form` 命令, 最小尺寸/尺寸/定位在 Rust 侧一次完成(单 IPC, 2026-07-12 定, 原为 3 次串行 JS API), **形态未变时直接跳过**——同形态路由间跳转不再把用户移动/调整过的窗口拽回屏幕中心。**形态位置记忆**(2026-07-12 Zero 定): 切换形态时记住旧形态当前位置(仅内存, 不落盘), 该形态本次运行内出现过就原位恢复——冲突处理完/失败/搁置回小窗时回到进大窗前的位置; 恢复前校验位置仍落在某块屏幕内(拔外接屏防丢窗), 否则居中; 首次出现(含应用启动)居中。**形态尺寸记忆**(2026-07-12 Zero 定): 手动调整过的窗口尺寸按形态持久化进 settings.json(`compactSize`/`largeSize`, 逻辑像素)——采集点 = 切换形态 / 关窗拦截(隐藏或退出) / 退出请求(⌘Q/托盘退出经 RunEvent::ExitRequested), 应用点 = set_window_form(记忆值钳到形态最小尺寸, None 用出厂默认)与启动 setup(show 前应用小窗记忆尺寸并保持居中, 无跳变); 两字段由 Rust 壳层**独占写入**, `set_settings` 忽略前端回传值(前端副本可能陈旧, 防覆盖新快照), 因此设置对话框"恢复默认"不清尺寸记忆; 现成的 tauri-plugin-window-state 按窗口 label 记忆, 不适配单窗口双形态模型, 故自实现。窗口配置 `visible:false` + 前端设置就位后立即 `show()`(防启动白闪; 不等 rAF——隐藏窗口不产帧, rAF 显窗死锁成永不出现, 2026-07-12 v0.1.0 实机修正), `backgroundColor` = 画布色 #1e1f22(防 resize 白边), `theme: Dark`(Windows 标题栏恒暗), `acceptFirstMouse: true`(macOS 未聚焦首击即生效)。**关闭窗口不退出**(2026-07-11 Zero 定): 关闭请求被拦截为隐藏, 应用驻留系统托盘——托盘菜单"显示窗口/退出", Windows/Linux 左键单击唤回, macOS 点 Dock 图标重开; 隐藏不销毁 webview, 会话状态原样保留, 真正退出走托盘菜单(或 ⌘Q)。

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
| 右下 | Close = **搁置**(2026-07-11 Zero 定, 对齐 IDEA): 操作与已解决进度留在 git 仓库, 回菜单小窗且抑制自动接管, 菜单顶部琥珀横幅(op + 剩余冲突数)可随时恢复, op 在外部结束时搁置自动失效; 若操作进行中另有 **Continue** 主按钮 (全部解决后) 与 Abort 次按钮 |
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
| chunk 底色 | `Decoration.line`; 中栏区间在前端 `resultRanges` 手动 remap(`changes.mapPos`), 装饰经 Compartment 重建且**只建视口裁剪窗口内的行**(视口 ±400 行, 逼近边缘 100 行内才按新窗口重建, 2026-07-12 定)——成本与视口成正比, 2MB 降级单 chunk 也不逐键卡顿; 侧栏"类名签名 + 裁剪窗口"双未失效即跳过, 手工打字只重建中栏 |
| 接缝条 ×2 | **44px**(2026-07-11 定稿, 原 ~28px)竖条内嵌 SVG: 每个可见 chunk 画 侧栏区间→Result区间 贝塞尔封闭带 (同底色, 控制点在两端 30% 处成 IDEA 斜扫); ✕ / ≫ / ≪ 按钮浮于其上; 滚动/编辑 rAF 节流重绘; 几何统一在 seamGeoms 每帧算一次(坐标基准每栏只读一次, 布局读取集中在模板写入前), y 坐标取自 `lineBlockAt`; 按中栏 viewport 粗筛, 屏外 chunk 不算不画 |
| gutter 按钮 | 左栏块 `✕ ≫`、右栏块 `≪ ✕` 镜像; hover 显示 tooltip; applied 后变 undo 图标 |
| 同步滚动 | 以 chunk 三栏区间起始行为锚点分段线性插值 × 行高; 程序写入的 scrollTop 逐目标记账, 目标栏的回声 scroll 事件直接吞掉(2026-07-12 定, 原 rAF 全局锁有 1px 抖动) |
| 底部横向滚动条 | 一根共享横条(IDEA 式, 2026-07-12 Zero 定)驱动三栏 scrollLeft 同像素联动(等宽字体下代码列跨栏对齐, 各栏按自身上限夹取); `linkHScroll` 用"横向增量为零即返回"守卫——兼吞程序回声、防纵向滚动把被夹取的栏反拽; 撑杆宽 = 条视宽 + 三栏最大可滚距, 在 bumpTick 的 rAF 里逐帧收敛(CM6 只渲染视口行, scrollWidth 随渲染增长); 无横向溢出时 visibility 隐藏(保留占位不跳布局) |
| overview ruler | 最右 ~12px: 全文 chunk 色块缩略(实时区间换算, 编辑后不漂移), 点击按滚动高度比例跳转 |
| 顶栏 | ↑↓ 上/下一个 change (自动滚三栏); ⋙ 组 = 批量应用非冲突 (Left/All/Right); Highlight words ▾ = words/none 切换 |
| 底栏 | Accept Left/Right = 整文件取侧; Cancel = 放弃本文件改动回列表; **Apply** = Result 全文 `save_result` 后回列表刷新 |

### 5.3 鼠标交互全集 (每一步都可点出来)

拖文件夹开仓库 → 菜单点操作(对话框选分支/提交) → 出冲突自动进列表 → 双击行进三栏 → hover chunk 高亮且按钮浮现 → 点 ≫/≪/✕ 处理 → 点接缝色带/ruler 跳转 → 顶栏 ⋙ 一键清非冲突 → 底栏 Apply → 列表 Continue → 完成回菜单。全程零键盘。

## 6. 主题 tokens (IDEA New UI 双色系, 暗色默认; 2026-07-10 定稿, 2026-07-12 增亮色)

`src/lib/theme.css` 集中 CSS 变量(--d-* 系列), **暗色在 `:root`、亮色在 `html[data-theme='light']` 整体翻转**(设置系统写 data 属性), 页面禁止写死 hex——新颜色必须进 token 双套。暗色基调: 画布 `#1e1f22` · 面板 `#2b2d30` · 分隔线 `#393b40`/`#43454a` · 文本 `#dfe1e5` / 弱化 `#9da0a8` · IDEA 蓝 `#3574f0`(主按钮) · 选中 `#2e436e` · 绿 `#5fad65` · 红 `#e46962` · 琥珀 `#d9a343`("进行中"语义色) · **PINCER 橙 `#ff7a2f`(仅 logo / 活动 tab 下划线 / 终端光标, 两主题共用)** + §5.2 四个 chunk 色。亮色为 IDEA New UI Light 近似初值(白画布 `#ffffff` / 面板 `#f7f8fa` / 选中 `#d4e2ff` 等, M5 与暗色一并逐像素校准)。等宽字体内嵌两款(均 OFL, 许可证 `static/fonts/OFL-*.txt`): JetBrains Mono Regular 默认(2026-07-11 落地, app.html 预载) + Maple Mono Regular/Italic 可选(2026-07-12 落地, 设置选中时才按需加载), UI 字体系统栈。

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

- 超大输入/病态 diff → Rust 侧 500ms deadline + 任一侧 >2MB 整文件降级守护兜底(词级强调另设 200 行 / 1000 字符上限); 降级后的整文件单 chunk 由装饰视口裁剪兜底, 逐键重建不随文件大小恶化;
- 接缝 SVG 与滚动帧同步 → rAF 节流 + viewport 粗筛 + seamGeoms 单点采集, 兜底直角折线;
- webkit2gtk (Linux) 字体度量差异 → M5 专项校准;
- 删除-修改、双添加等边角 stage 组合 → M1 用 playground 全场景覆盖。

## 10. 默认决定 (不同意随时改)

- Conflicts 与三栏为**单窗口内路由页**切换, 不开子窗口(小窗↔大窗靠运行时改尺寸);
- **全局统一 IDEA New UI 暗色**(2026-07-10 Zero 拍板): 冲突大窗布局按参考图 1:1、配色换 IDEA Dark、文案英文; 菜单小窗带中文说明;
- 撤销只在会话内有效, Apply 落盘后不可撤 (可重新 checkout 制造冲突, 不在本工具内做);
- **构建 profile**(2026-07-12 定): dev 下依赖统一 O2(similar 热循环走优化码路, `pnpm tauri dev` 体验≈release; 首次构建慢一次, 本 crate 保持 O0 快增量), release 用完整 LTO + codegen-units=1 + opt-level=3 + strip + `panic = "abort"`(Zero 拍板, 换免展开表的更小更快二进制)。代价要清楚: release 下 spawn_blocking 里的意外 panic 会直接终止应用而不是落成前端 toast——dev 仍是展开, 崩溃排查用 dev 复现; clippy 的 `panic/unwrap_used` 告警把 panic 面压到最低是这个选择的前提;
- **终端缓冲**: `git://output` Rust 侧聚批 + 前端 rAF 合帧批量落地, 缓冲上限 2000 条(超限整批丢最旧;
  条目挂单调 id 作渲染 key, 裁剪 splice 不再整列表重渲), 长会话 DOM 不无界增长;
- **聚焦重探限频**: 800ms 冷却 + 进行中不叠加(菜单页与冲突列表页共同约定);
- **生产加固**: 禁浏览器右键菜单(可编辑区/有选区除外, 选区保留原生 Copy)与刷新/打印快捷键(刷新丢会话状态); dev 构建不受限;
- **设置系统**(2026-07-12 Zero 定, 推翻"配置系统不做"): 存 app-data `settings.json`(与 recent.json 同目录, 应用更新/重装保留); Rust `settings.rs` 类型化 `Settings`, **全字段 `#[serde(default)]`** = 跨版本兼容契约(旧文件缺字段落默认, 新版本删的字段被忽略, 永不因升级重置); 前端 `settings.svelte.ts` **即改即存**(无 OK/Cancel 暂存), 经 `--editor-font-size` / `--editor-font-family` CSS 变量生效(编辑器进 /merge 时新建取当次值, 不做热更新); 入口 = 菜单顶栏齿轮。首批四项: 编辑器字号(钳 8–32)/编辑器字体(空 = 内嵌 JetBrains Mono)/关窗行为(托盘|退出, lib.rs 关闭拦截读 `AppState::close_to_tray()`)/词级强调默认开关。**编辑器字体选择器**(2026-07-12): 自由文本框升级为下拉——内嵌清单 `EMBEDDED_FONTS`(JetBrains Mono 默认 + Maple Mono) + "自定义…"展开文本框输系统字体名, 存储契约不变(仍是单字符串, '' = 默认); 进 /merge 建编辑器前 `ensureEditorFont()` 用 `document.fonts.load` 预载所选字体(与快照/语言包并行), 防 CM6 首次测量按回落字体算宽。**主题与语言**(2026-07-12 同日落地): `theme = dark|light`——亮色即 §6 的 Light tokens 整体翻转, CM6 出亮暗双主题+双语法配色(进 /merge 时按当次设置选用), 窗口原生主题/底色与托盘菜单文案由 Rust `apply_to_shell` 在 setup 与 set_settings 时同步; `language = zh|en`——**zh 保持分层设计(大窗 IDEA 英文原文 + 小窗中文辅助), en 为全英文**, 大窗原文不进词典; 词典在 `src/lib/i18n.ts`(纯模块, [zh,en] 词条, vitest 校验完整性), 响应式 `t()` 读语言设置, 切换后已挂载 UI 即时翻转无需重载。设置入口 ×2 (2026-07-12 Zero 定, 撤掉菜单页齿轮保持顶栏极简): **macOS 应用菜单 "设置…"**(默认菜单插项, About 之后, 事件 `app://open-settings` 唤窗弹框, 文案随语言) / **⌘(Ctrl)+, 快捷键**(webview 侧监听, 全平台全页面); 对话框渲染在根布局, 任何路由可弹, 编辑器相关项注明"重进合并页生效"; **三页签布局**(2026-07-12 Zero 定): 通用(语言/关窗) · 界面(主题/编辑器字号/字体/词级强调) · 关于(字标/版本/主页/许可/致谢), 活动页签橙色下划线(用色约定允许项), 正文定高切页不跳动, 关于页脚只留"完成"。
- **产品名 Pincer, 字标 PINCER**(2026-07-12 Zero 定, 同日由全大写修订): OS 表面用 Title case——`product-name` / 窗口 title / 托盘 tooltip / app.html title / bundle long-description 统一 "Pincer"(.app 名、菜单栏应用菜单标题、安装列表、发布产物文件名随之), 全大写只留给缩写词是业界惯例(VLC/OBS/GIMP), 非缩写全大写属于字标风格化; **品牌画布保持全大写 PINCER**——打开页字标、README 横幅/标题、About 品牌字。`identifier` 不变(用户数据路径不动), 仓库/包名仍 git-pincer-desktop。
- **Tauri 配置 TOML 化**(2026-07-12 Zero 定): `src-tauri/Tauri.toml` 替代 tauri.conf.json——tauri 与 tauri-build **同时启用 `config-toml` feature**(编译期 build.rs 与 `generate_context!` 都要解析), CLI 侧原生支持无需配置; 键名按官方惯例 **kebab-case**(camelCase 经 serde alias 同样可用); TOML 无 null 字面量, 原 `csp: null` 曾以省略键表达。**CSP 加固**(2026-07-12): default-src 'self' + Tauri IPC 通道(ipc:/http://ipc.localhost) + style unsafe-inline(CM6/Svelte 内联样式) + 字体图片随包——纯静态 SPA 无远程内容, 收紧无功能损失; bundle 补齐 macOS 最低版本 10.15 / NSIS+WiX 安装器中英双语 / deb section=vcs。
- **性能 v2**(2026-07-12 本轮, 过程清单见 docs/PERF.md): 显窗时机 = **设置就位后立即 show**(浅色主题启动不再暗→亮闪变, loadSettings 竞速 150ms 兜底; 初版包了一层 rAF, macOS 实机死锁——隐藏窗口不产帧回调永不触发, v0.1.0 验收发现后去掉); SettingsDialog 动态 import 出 boot chunk(它拖着 plugin-opener/getVersion); `git://output` Rust 聚批 + 终端条目 id key(见上); 关窗收托盘时序 = **采尺寸(内存)→hide→落盘**(磁盘延迟不垫关窗手感, commands.rs capture/persist 拆分); `build_snapshot` 三份全文**按值移动**进快照(临近 2MB 的文件省一轮全文拷贝); 冲突列表 path→index Map 消 O(N²); /conflicts 预热 ensureEditorFont(preloadCode 上轮已有)。平台专属(Tauri.toml): Windows `scroll-bar-style = "fluentOverlay"`(WebView2 ≥125, 低版本无操作), macOS 14+ `background-throttling = "disabled"`(默认策略驻留托盘约 5 分钟后挂起页面, 长操作终端输出断流), 打包 `remove-unused-commands`(按 ACL 清未用插件命令)。**明确不做**: Linux 不无条件设 `WEBKIT_DISABLE_*` 环境变量(官方警示会给所有人关掉加速路径, WebKitGTK ≥2.42 已自带 NVIDIA 规避, 等真实报障再按机器条件化); `additional-browser-args` 不碰(一旦设置整体顶掉 wry 默认参数); panes 不加 CSS contain(WKWebView 接缝重绘 bug)。
