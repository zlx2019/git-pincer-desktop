# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目定位

IDEA 风格的 Git 冲突解决桌面端 (Tauri 2 + SvelteKit/Svelte 5 + CodeMirror 6, 合并引擎在 Rust)。功能对齐姊妹项目 git-pincer (CLI)，但代码独立实现。**只做冲突解决流程**（发起操作 → 冲突列表 → 三栏合并 → continue/abort），不做 log/commit/push 等通用 git 客户端功能。

技术方案的权威文档是 `docs/PLAN.md`（架构、命令表、UI 还原清单、已定稿的默认决定）。PLAN 残留少量 v2 时代的陈旧表述，以定稿决定与代码现状为准：全局 IDEA New UI **暗色**（非 Light）、diff 在 Rust 侧 **500ms** deadline（非 Web Worker/300ms）、事件只有 `git://output`（`git://round`/`git://done` 未实现，结局走命令返回值）、终端面板是手写 CSS（非 xterm.js）。样式基准分层：`docs/IDEA_STYLE.md` 定义全局 IDEA New UI 三层灰暗色基调，根目录 mockup 是**菜单小窗**的形态基准，冲突列表与三栏的布局则按 IDEA 参考截图 1:1 还原——截图不在仓库内（M5 像素校准依赖 Zero 的截图反馈）。

## 常用命令

```bash
pnpm install              # pnpm 版本由 package.json 的 packageManager 锁定 (10.13.1), Node >= 22
pnpm tauri dev            # 运行应用 (vite 端口固定 1420 strictPort, 被占用直接失败)
pnpm check                # svelte-check 类型检查
pnpm test                 # vitest 前端纯逻辑单测 (src/lib/*.test.ts, node 环境)
pnpm build                # 前端构建 → build/
pnpm tauri build          # 本机打包; 正式发布 = 推 v* 标签走 release.yml
                          # (四平台矩阵: dmg / nsis+msi / AppImage+deb+rpm, git-cliff 生成 changelog)
```

Rust 侧检查（在 `src-tauri/` 目录下执行）：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features --no-tests pass    # 全部测试
cargo nextest run <测试名>                           # 单个测试
cargo doc --no-deps --all-features                  # CI 以 RUSTDOCFLAGS=-D warnings 把守, 文档警告会挂 CI
cargo deny check                                     # 依赖审计
typos                                                # 拼写检查
```

**重要**: `tauri::generate_context!` 编译期要求前端产物 `build/` 存在——首次 clone 或清理后，任何 cargo check/clippy/test 之前必须先 `pnpm build`。

单元测试写在模块末尾 `#[cfg(test)]`（引擎测试在 `merge.rs` 尾部），集成测试在 `src-tauri/tests/plumbing.rs`（真实 git 管道）。前端可测的纯逻辑（chunk 状态机/导航/批量应用/文本组装/区间换算）集中在 `src/lib/chunks.ts`，vitest 测试同目录（`chunks.test.ts`，配置在 `vitest.config.ts`，不经 SvelteKit 插件）——给 merge 页加逻辑时优先抽到该模块并补测试。写集成测试注意：lib crate 名为 **`git_pincer_desktop_lib`**（非包名）；clippy.toml 的 `allow-unwrap-in-tests` 不覆盖集成测试的辅助函数，文件首行需 `#![allow(clippy::unwrap_used)]`；测试仓库统一注入 `GIT_CONFIG_GLOBAL=/dev/null` + `GIT_CONFIG_SYSTEM=/dev/null` 并在仓库本地关闭 gpgsign/rerere 保证确定性。

测试数据用姊妹仓库的演练场生成（含 merge / 两轮 rebase / cherry-pick / revert / 二进制场景）：

```bash
cd ../git-pincer && cargo run --example playground   # 生成 /tmp/git-pincer-playground
```

提交遵循 Conventional Commits（无钩子/CI 强制，只供 git-cliff 生成 changelog；**含中文的提交消息会被 cliff.toml 跳过**，不进 release notes）；pre-commit 钩子（`pre-commit install`）会跑 fmt / deny / typos / svelte-check / vitest / check / clippy / test 全套（cargo 类钩子缺 `build/` 时会自动先 `pnpm build`）。

## 架构: 计算在 Rust, 交互状态在前端

Rust 壳层**不持业务状态**，`open_merge` 是纯查询；chunk 的交互状态（应用/忽略/撤销）与文档模型全部留在前端（零 IPC 往返）。

```
前端 (SvelteKit + CM6): 三栏视图 + chunk 交互状态 + 撤销栈
        ↓ invoke / ↑ event (仅 git://output)
src-tauri (无业务状态):
  merge.rs   合并引擎(similar): 行 diff / 分块 / 词级强调 → MergeSnapshot
  repo.rs    git 管道: 读三方 / 写结果+add / launch / continue / abort
  commands.rs Tauri 命令层 (async, git 调用走 spawn_blocking), lib.rs 只做注册
  error.rs   thiserror 错误, 序列化为消息字符串 → 前端 toast
        ↓
git 二进制 (继承用户 credentials/hooks/rerere 配置)
```

- **IPC 契约**: `src/lib/api.ts` 与 Rust serde 输出严格镜像，casing 规则——struct 字段 camelCase；单元枚举 lowercase（`ChunkKind`/`ChunkVisual`/`SideStatus`/`PickSide`）或 kebab-case（`Op`/`LaunchKind`）；`LaunchOutcome`/`RoundOutcome` 是 `tag = "kind"` 的 tagged union（`cleanDone|conflicts|failed` / `done|nextRound|failed`）。行区间一律 **0-based 半开 `[start, end)`**；词级强调三元组 `[chunk 内行, UTF-16 起, UTF-16 止]`（与 CM6 文档坐标一致）。改任何命令签名/结构体，两侧同步改。
- **合并引擎不变量** (merge.rs, 纯函数): 两次行级 Myers diff (`base→ours`, `base→theirs`) 按 base 区间碰撞归簇（相触即归并），**宁可多报冲突不静默错合**；守护: 500ms diff deadline、任一侧 >2MB 降级整文件单冲突、>200 行 chunk 跳过词级强调。二进制判定在 repo.rs：工作区文件**前 8KB 含 NUL**（读不到按文本处理），binary 行走 pick-one 不进三栏。
- **三栏视图模型**: Result 栏初始 = base 全文（IDEA 行为）；chunk 的中栏区间存前端 `resultRanges` 数组，在 CM6 updateListener 里用 `changes.mapPos` 手动 remap（不是 RangeSet 自动映射）；装饰经 Compartment.reconfigure 全量重建（queueMicrotask 合批）。撤销分两轨——手工编辑走 CM6 历史 (⌘Z)；chunk 操作带 `chunkOp` annotation + `addToHistory.of(false)`，走独立撤销栈，不混编。已应用一侧后再应用另一侧 = **追加**到区间尾（keep both，对齐 CLI 的 take order）。
- **git 安全策略** (repo.rs): 参数永远按数组传递不经 shell；所有流式命令（launch 与 continue）统一注入 `GIT_EDITOR=true` `GIT_SEQUENCE_EDITOR=true` `GIT_TERMINAL_PROMPT=0` 且 stdin 置 null；统一入口清洗宿主 `GIT_DIR` 类环境变量（SCRUBBED_ENV）。
- **结局分流**: launch/continue 结束后**只要 `conflicts()` 非空即接管，与退出码无关**；干净且零退出才算完成。`git://output` 监听是发起前订阅、finally 退订的 JIT 模式，无操作等待期间的输出会被丢弃。
- **顺序不变量**: cherry-pick 对话框列表新→旧，确认后反转为**旧→新**逐个应用；revert 保持新→旧。
- **路由即窗口形态** (`src/lib/win.ts`): `/`(打开页) 与 `/menu`(指令面板) 用紧凑小窗 420×640；`/conflicts` 与 `/merge` 切大窗 1280×800。跨页会话状态在 `src/lib/state.svelte.ts` 的 `session` ($state rune)——刷新即失，除打开页外所有路由 onMount 都守卫 `!session.info → goto('/')`（/menu 还会在 op 存在时转 /conflicts）。
- **接管机制**: 无论操作在面板发起还是终端发起，窗口重获焦点会重探仓库状态，出现冲突即接管切大窗。
- **能力白名单**: 前端调用的窗口/插件 API 必须列入 `src-tauri/capabilities/default.json`（现有 core/dialog/opener + window 的 set-size/set-min-size/center），否则运行时被拒。

## 约定

- 工具链 1.96.0 锁定于 `rust-toolchain.toml`；edition 2024 与 rust-version 声明在 `src-tauri/Cargo.toml`。lint 已开 `unwrap_used` / `expect_used` / `panic` / `dbg_macro` / `missing_docs` / `unsafe_code` 警告，clippy CI 用 `-D warnings`——公共项必须写文档注释，禁用 `.unwrap()`（测试除外）。
- 前端是纯 SPA：`ssr = false` + adapter-static（fallback index.html），**禁止** server load / `+server.ts` 等服务端能力；`src-tauri/**` 不在 vite watch 内（改 Rust 不触发前端 HMR）。
- 主题 tokens 集中在 `src/lib/theme.css`（`--d-*` 变量，IDEA New UI 暗色）；PINCER 橙 `#ff7a2f` 仅用于 logo / 活动 tab 下划线 / 终端光标，不得挪作他用。
- 无 UI 组件库、无 JS diff 库：样式手写 CSS，diff 全在 Rust；CM6 共用基建（主题/高亮/装饰/同步滚动）在 `src/lib/editor.ts`，纯 chunk 逻辑（无视图依赖）在 `src/lib/chunks.ts`。合理需要的新依赖可直接引入，无需逐项报批（2026-07-11 Zero 定）——但上述两条架构决定（不引 UI 组件库/JS diff 库）仍然有效。
- UI 文案：大窗（Conflicts/三栏）用 IDEA 英文原文；小窗（打开页/菜单）的辅助文案用中文（说明行、终端 tab、状态栏、✔/✘ 尾行）。
