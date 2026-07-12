# 性能优化 v2（perf/smoothness 分支工作清单）

目标: 启动更快、点击/滚动更跟手、大文件更稳。上一轮已落地视口裁剪装饰、接缝几何去重、
git 输出前端合帧、单 IPC 切窗、构建参数调优——本轮在其之上，完成后要点回写 PLAN/CLAUDE，本文档归档删除。

状态: ☐ 待做 / ☑ 已做 / ✗ 否决（含原因，防止后人重查）。

## 基线事实（2026-07-12 审计）

- 启动路径已很轻: Rust setup 无 git 进程、settings.json <1KB 同步读、托盘图标 927B 内嵌;
  CM6 (270K) 与 fileicon (27K) 均不在 boot chunk（路由级懒加载 + language-data 按需拉语法）;
  boot 预载 = 9 个小 chunk + JetBrains Mono Regular。瓶颈是 webview 进程初始化（平台固有）。
- `/merge` 首次进入需现场加载 ~700KB JS（`goto()` 导航吃不到 hover 预载）。
- 滚动帧路径已优化到位: bumpTick 空闲即停、布局读取批量前置、scroll/wheel passive、
  装饰约每 ~300 行滚动才重建一次。CSS 无 filter/transition/will-change 隐患。
- release cargo profile 已顶配（fat LTO + codegen-units=1 + O3 + strip + panic=abort），无油水。

## 阶段一 跨平台

启动/首屏:

- ☑ 浅色主题首帧闪变: `data-theme` 在 get_settings IPC 返回后才落 DOM，而 `show()` 首帧 rAF 就触发
  → 浅色用户每次启动暗→亮闪一下。已改为设置就位后（150ms 兜底竞速）直接 show。(+layout)
  ⚠ 初版在 show 外还包了一层 rAF——macOS 实机死锁: 隐藏窗口 WKWebView 不产帧，
  回调永不触发，窗口永不出现（点 Dock 图标经 Reopen 才被 Rust 侧救活）。v0.1.0 验收发现后去掉；
  显窗早于首帧无闪变，透出的是 Rust 已按主题就位的原生底色。
- ☑ SettingsDialog 挪出 boot chunk: 静态 import 把 plugin-opener + getVersion 拖进 nodes/0，
  实际渲染在 `{#if settingsUi.open}` 之后。已改 `{#await import(...)}`。(+layout)

首次进三栏:

- ☑ 预载 `/merge`: `preloadCode` 上轮 f0edabf 已加在 conflicts onMount（本轮审计代理漏报）；
  本轮补 `ensureEditorFont()` 预热。**menu 页不加**——常驻小窗不背 CM6 的解析内存，
  conflicts 页恰好只在有冲突时存在，预载点已精确。

交互路径:

- ☑ `git://output` Rust 侧合批: 原逐行 emit，啰嗦操作一次几百个 IPC 事件（前端 rAF 合帧只省了
  DOM，省不掉序列化/派发）。repo.rs 流式 drain 循环改 `recv_timeout` 聚批（≤25ms 或 ≤64 行，
  常量 STREAM_BATCH_*），事件载荷 `OutputLine` → `OutputLine[]`，api.ts 与两处监听已适配。
  **IPC 契约变更**，PLAN §3 已更新。
- ☑ 关窗收托盘手感: 原先同步写 settings.json 再 `hide()`。已拆 capture_form_size（纯内存）/
  persist_settings（落盘），CloseRequested 时序 = 采尺寸 → hide → 落盘。(commands.rs + lib.rs)
- ☑ 冲突列表 O(N²): 每行 `flat.indexOf(f)` → derived 预建 `flatIndex` path→index Map。(conflicts)
- ☑ 终端列表按索引 key: 2000 条封顶 splice 后整列表重渲 → TermRow 挂单调 id，按 id key。
  (state.svelte.ts + menu。conflicts 的 console 每次操作前清空、只追加不裁剪，索引 key 本就稳定，不动)
- ☑ 聚焦重探比较去 `JSON.stringify`×2 → filesKey 逐字段拼接比较。(conflicts)

大文件/内存:

- ☑ open_merge 快照双拷贝: `build_snapshot` 对三份全文 `.to_owned()`，临近 2MB 上限
  的文件瞬时 ~2×6MB。已改移动语义（三份 String 按值传入，直接 move 进 MergeSnapshot）。

否决/搁置（本轮结论，勿重查）:

- ✗ `drag-drop-enabled = false`: 打开页在用 `onDragDropEvent`（拖文件夹开仓库），关了会坏。
- ✗ 三栏 panes 加 CSS `contain`/`content-visibility`: WKWebKit 接缝重绘 bug，merge 页有注释记录。
- ✗ cargo profile 再调: 已顶配；`lto="thin"` 只换编译速度，运行时是倒退。
- ✗ set_settings/recent 等小文件 fs 不进 spawn_blocking: <1KB 低频，改了只添噪音。
- ⏸ seamGeoms 视口裁剪 O(全 chunks)/帧、rulerBlock O(全 chunks)/键: 仅数千 chunk 的极端文件受益，
  改二分风险>收益，等真实场景反馈。
- ⏸ open_merge 走 binary channel 替代 JSON invoke: 仅临近 2MB 的文件收益明显，等反馈。

## 阶段二 平台专属（2026-07-12 调研定稿并落地，键名已对照 tauri-utils 2.9.3 源码）

- ☑ Windows `scroll-bar-style = "fluentOverlay"`（Tauri 2.8.0+ 官方开关, WebView2 ≥125 生效、
  低版本无操作; 取代 `--enable-features=OverlayScrollbar` 参数黑法）——原生 Fluent 悬浮滚动条。
- ☑ macOS `background-throttling = "disabled"`（Tauri 2.3.0+, 仅 WebKit/macOS 14+ 生效）:
  默认策略下驻留托盘约 5 分钟后页面被挂起，进行中操作的终端输出会断流; disabled 保持隐藏期
  JS 照常。空闲无定时器，电池代价可忽略。
- ☑ 打包 `remove-unused-commands = true`（[build], Tauri 2.4+）: 按 capabilities ACL 清理
  未用到的插件命令粘合代码，**只涉插件**（dialog/opener），应用自身命令不受影响。体积项。
- ✗ Windows `additional-browser-args`: 源码确认一旦设置**整体顶掉** wry 默认参数
  （三个 msWebOOUI/msPdfOOUI/msSmartScreenProtection disable + autoplay/proxy 派生参数），
  且 MS 官方声明生产环境不应使用浏览器 flag。不碰。
- ✗ Windows `webview-install-mode`: 各 evergreen 模式稳态启动无差别，只影响安装器体积;
  fixedRuntime 反而更冷（不与系统共享预热）。维持默认 downloadBootstrapper。
- ✗ Linux `WEBKIT_DISABLE_DMABUF_RENDERER` / `WEBKIT_DISABLE_COMPOSITING_MODE` 无条件注入:
  官方文档明示"仅在确认受影响时才整体覆盖"——会给所有人关掉加速渲染路径; WebKitGTK ≥2.42
  已自带 NVIDIA 规避。等真实报障，届时用按机器条件化方案（如 webkit2gtk-nvidia-quirk）。
- 备忘: WebView2 冷启动慢的常见外因是杀软实时扫描 UDF（微软文档有案；用户侧排除项，应用无解）;
  Windows 上 >MB 级 IPC 明显比 macOS 慢（社区实测 10MB ≈ 200ms vs 5ms），open_merge 若收到
  Windows 大文件卡顿反馈，升级路径是 `tauri::ipc::Response` 二进制通道（见阶段一 ⏸ 项）。

## 阶段三 验证

- vitest / svelte-check / clippy / nextest 全套 + mock 环境过主流程。
- 真机手感验收（Zero）: 浅色主题启动无闪变、点关闭到窗口消失无迟滞、冲突列表→首次进三栏的
  等待感、长输出操作终端流畅度。
