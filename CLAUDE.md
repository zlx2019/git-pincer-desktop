# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目定位

IDEA 风格的 Git 冲突解决桌面端 (Tauri 2 + SvelteKit/Svelte 5 + CodeMirror 6, 合并引擎在 Rust)。功能对齐姊妹项目 git-pincer (CLI)，但代码独立实现。**只做冲突解决流程**（发起操作 → 冲突列表 → 三栏合并 → continue/abort），不做 log/commit/push 等通用 git 客户端功能。

技术方案的权威文档是 `docs/PLAN.md`（架构、命令表、UI 还原清单、已定稿的默认决定都在里面）；样式基准见 `docs/IDEA_STYLE.md` 与根目录 mockup HTML。

## 常用命令

```bash
pnpm install
pnpm tauri dev            # 运行应用
pnpm check                # svelte-check 类型检查
pnpm build                # 前端构建 → build/
pnpm tauri build          # 打包 (dmg / nsis / AppImage+deb)
```

Rust 侧检查（在 `src-tauri/` 目录下执行）：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features --no-tests pass    # 全部测试
cargo nextest run <测试名>                           # 单个测试
cargo deny check                                     # 依赖审计
typos                                                # 拼写检查
```

**重要**: `tauri::generate_context!` 编译期要求前端产物 `build/` 存在——首次 clone 或清理后，任何 cargo check/clippy/test 之前必须先 `pnpm build`。

单元测试写在模块末尾 `#[cfg(test)]`（引擎测试在 `merge.rs` 尾部），集成测试在 `src-tauri/tests/plumbing.rs`（真实 git 管道）。

测试数据用姊妹仓库的演练场生成（含 merge / 两轮 rebase / cherry-pick / revert / 二进制场景）：

```bash
cd ../git-pincer && cargo run --example playground   # 生成 /tmp/git-pincer-playground
```

提交遵循 Conventional Commits；pre-commit 钩子（`pre-commit install`）会跑 fmt / deny / typos / svelte-check / check / clippy / test 全套。

## 架构: 计算在 Rust, 交互状态在前端

Rust 壳层**不持业务状态**，`open_merge` 是纯查询；chunk 的交互状态（应用/忽略/撤销）与文档模型全部留在前端（零 IPC 往返）。

```
前端 (SvelteKit + CM6): 三栏视图 + chunk 交互状态 + 撤销栈
        ↓ invoke / ↑ events (git://output)
src-tauri (无业务状态):
  merge.rs   合并引擎(similar): 行 diff / 分块 / 词级强调 → MergeSnapshot
  repo.rs    git 管道: 读三方 / 写结果+add / launch / continue / abort
  commands.rs Tauri 命令层 (全部 async + spawn_blocking), lib.rs 只做注册
  error.rs   thiserror 错误 + serde 序列化 → 前端 toast
        ↓
git 二进制 (继承用户 credentials/hooks/rerere 配置)
```

- **IPC 契约**: `src/lib/api.ts` 中的 TS 类型必须与 Rust 的 serde 输出严格镜像（Rust 侧 `rename_all = "camelCase"`）。改任何命令签名/结构体，两侧同步改。
- **合并引擎不变量** (merge.rs, 纯函数): 两次行级 Myers diff (`base→ours`, `base→theirs`) 按 base 区间碰撞归簇, **宁可多报冲突不静默错合**; 守护: 500ms diff deadline、任一侧 >2MB 降级整文件单冲突、>200 行 chunk 跳过词级强调。词级强调偏移是 **UTF-16**（与 CM6 文档坐标一致）。
- **三栏视图模型**: Result 栏初始 = base 全文（IDEA 行为）；chunk 区间放 CM6 RangeSet 随编辑自动 remap；撤销分两轨——手工编辑走 CM6 历史 (⌘Z)，chunk 操作走独立撤销栈，不混编。
- **git 安全策略** (repo.rs): 参数永远按数组传递不经 shell；continue 类命令注入 `GIT_EDITOR=true`；launch 注入 `GIT_TERMINAL_PROMPT=0`；清洗宿主 `GIT_DIR` 类环境变量。
- **路由即窗口形态** (`src/lib/win.ts`): `/`(打开页) 与 `/menu`(指令面板) 用紧凑小窗 420×640；`/conflicts` 与 `/merge` 切大窗 1280×800。跨页会话状态在 `src/lib/state.svelte.ts` 的 `session` ($state rune)。
- **接管机制**: 无论操作在面板发起还是终端发起，窗口重获焦点会重探仓库状态，出现冲突即接管切大窗。

## 约定

- Rust 1.96.0 / edition 2024 锁定于 `rust-toolchain.toml`；lint 已开 `unwrap_used` / `expect_used` / `panic` / `missing_docs` 警告，clippy CI 用 `-D warnings`——公共项必须写文档注释，禁用 `.unwrap()`（测试除外）。
- 主题 tokens 集中在 `src/lib/theme.css`（`--d-*` 变量，IDEA New UI 暗色）；PINCER 橙 `#ff7a2f` 仅用于 logo / 活动 tab 下划线 / 终端光标，不得挪作他用。
- 无 UI 组件库、无 JS diff 库：样式手写 CSS，diff 全在 Rust。新增依赖前先征求同意。
- UI 文案用 IDEA 英文原文（菜单小窗的说明行除外，用中文）。
