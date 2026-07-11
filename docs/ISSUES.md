# 已知问题清单

> 2026-07-11 全库通读后整理（结合 4 路并行审计交叉验证）。建档前已修复两项，不在此列：
> ⇧F7"上一个 change"错跳（`merge/+page.svelte` nav()）、词级强调缺超长行守护（`merge.rs` 新增 `MAX_EMPHASIS_LINE_CHARS`）。
>
> 处理顺序按 Zero 安排：**先 UI/UX 批次**，其余后续再改。修完一项就从本文档删除对应条目。

## UI / UX（下一批）

### 1. rebase 进行中标题的 onto 侧退化为短 sha

- **现象**: rebase 中 HEAD 游离，`labels()` 的 yours 取 `rev-parse --abbrev-ref HEAD` 得到 `HEAD` 后退化为短 sha，标题显示 "Rebasing branch feature onto **a1b2c3d**" 而非分支名。
- **位置**: `src-tauri/src/repo.rs:196`（`labels()`）。
- **方向**: onto 侧可读 `.git/rebase-merge/onto`（sha）再经 `name-rev` / `ref_label()` 解析成分支名，解析不出再退短 sha；注意 `rebase-apply` 路径同理。

### 2. PINCER 橙出现第四处使用 —— 需 Zero 拍板

- **现象**: 菜单状态栏"● 执行中"用了 `var(--d-orange)`，违反主题约定"橙仅用于 logo / 活动 tab 下划线 / 终端光标"。
- **位置**: `src/routes/menu/+page.svelte:630`（`.sb-busy`）。
- **方向**: 二选一——改用 `--d-amber`（琥珀本就是"进行中"语义色），或修订 CLAUDE.md/PLAN 的三处规则把"状态栏执行中指示"列为第四处合法使用。

### 3. agree chunk 右栏有底色无词级强调

- **现象**: 双方一致的 chunk 左栏有词级强调、右栏没有（内容相同，强调理应对称）。
- **位置**: `src-tauri/src/merge.rs:310`（`emphasis()` 对 `Agree` 只写 `left`）。
- **方向**: Agree 分支把同一组区间同时写入 right（ours == theirs，区间可直接复用）。

### 4. applyAll 左右不对称：⋙ Right 不应用 agree chunk

- **现象**: `applyAll('left')` 会应用 ours + agree，`applyAll('right')` 只应用 theirs-only，agree 被跳过——"≪ Right"清完后 agree 仍 pending。
- **位置**: `src/routes/merge/+page.svelte:344`（`applyAll()` 的方向过滤）。
- **方向**: `direction === 'right'` 时放行 `kind === 'agree'`（用 side 'theirs' 或 'ours' 均可，内容相同）；若这是有意对齐 IDEA 的行为则在代码注释里写明并关闭本条。

### 5. overview ruler 与点击跳转用快照静态坐标，编辑后漂移

- **现象**: ruler 色块位置/高度用 `c.resultRange`（快照静态行区间）、跳转用 `snap.result` 的静态总行数估算文档高度；中栏一经编辑（apply/手工改动使行数变化），色块与实际 chunk 位置、点击落点都会偏移。
- **位置**: `src/routes/merge/+page.svelte:581`（色块渲染）、`:470`（`jump()`）。
- **方向**: 色块改用实时 `resultRanges`（换算行号可用 `doc.lineAt`），总高度用 `views[1].state.doc.lines`（或直接 `scrollDOM.scrollHeight` 比例）；随 `scrollTick`/文档变更失效重算。

### 6. app.html 仍是模板默认 title

- **现象**: `<title>Tauri + SvelteKit + Typescript App</title>`。Tauri 窗口标题来自 tauri.conf.json 不受影响，但属于模板残留。
- **位置**: `src/app.html:7`。
- **方向**: 改成 `git-pincer` 或 `PINCER`。

### 7. JetBrains Mono 未内嵌，跨平台等宽字体不一致

- **现象**: PLAN §6 声称"JetBrains Mono (OFL 内嵌)"，但仓库无字体文件，`--font-mono` 依赖系统已装；Linux/webkit2gtk 与未装该字体的机器会回落到 Menlo/Consolas/monospace，行高与字宽度量不同，影响 M5 像素校准。
- **位置**: `src/lib/theme.css:6`；`static/` 目前只有 favicon。
- **方向**: 内嵌 OFL 许可的 JetBrains Mono woff2（Regular 即可，@font-face 放 theme.css），或修订 PLAN 明确"依赖系统字体"。

## 性能（大文件场景才显现，优先级低于 UI 批次）

### 8. 每次编辑全量重建三栏装饰

- **现象**: 任何 doc 变更（含每个 chunk 操作）都经 `Compartment.reconfigure` 重建三栏全部 line/mark 装饰，O(chunks × lines)；chunk 很多时每次按键都有可感开销（已有 queueMicrotask 合批，但仍是全量）。
- **位置**: `src/routes/merge/+page.svelte:226`（`refreshDecos()`）、`src/lib/editor.ts:85`（`buildPaneDecos()`）。
- **方向**: 侧栏装饰静态可只建一次（仅类名随状态变的 chunk 重建）；中栏改 StateField + `decorations.map(tr.changes)` 自动映射，仅状态变化的 chunk 重建对应区间。

### 9. 接缝 SVG 每帧为所有 chunk 画路径（含屏外）

- **现象**: 每次 scrollTick 对全部 chunk 调 `chunkGeom()`（各含多次 `lineBlockAt`）并渲染贝塞尔带与按钮，屏外 chunk 白算白画。
- **位置**: `src/routes/merge/+page.svelte:419`（`chunkGeom()`）、`:516` 起（两条 seam 的 `{#each snap.chunks}`）。
- **方向**: 按视口（scrollTop ± clientHeight 的行区间）先粗筛可见 chunk 再计算/渲染；粗筛可用二分。

### 10. accept_side 每路径全量重扫 conflicts()

- **现象**: `accept_side()` 先跑一次全量 `conflicts()`（含每文件 8KB 二进制嗅探）再逐路径处理；列表页多选批量 Accept 时是"一次全扫 + N 次 git 调用"，文件多时变慢。
- **位置**: `src-tauri/src/repo.rs:279`（`accept_side()`）。
- **方向**: 嗅探仅对传入 paths 做；或让前端把已知的 `FileRow` 状态传下来免重扫（注意保持"以 git 实际状态为准"的保守原则）。

## 工程 / 文档

### 11. 前端零测试

- **现象**: 前端无任何测试设施（无 vitest）；chunk 状态机、applySide 追加语义、nav 循环等纯逻辑只能靠人工回归（本次 ⇧F7 bug 即无测试兜底）。
- **方向**: 引入 vitest（新增依赖需 Zero 同意），把 `merge/+page.svelte` 内的纯逻辑（状态机/导航/joined 拼接）抽到可测的 `src/lib/` 模块再补测。

### 12. PLAN.md 残留 v2 陈旧表述

- **现象**: §1"v1 硬编码 IDEA Light 主题"、§5.2"自写 IDEA Light HighlightStyle"、§9"Web Worker + >2MB/300ms"、§3 事件表列有未实现的 `git://round`/`git://done`、§6"JetBrains Mono OFL 内嵌"（见第 7 条）、IDEA_STYLE.md 的 xterm.js/portable-pty 路线未采用。CLAUDE.md 已加"以定稿决定与代码为准"的护栏，但 PLAN 本体宜清理。
- **位置**: `docs/PLAN.md:22,132,175,64,147`、`docs/IDEA_STYLE.md:5`。
- **方向**: 按代码现状修订 PLAN（或在对应行加"已被 vN 决定取代"标注），避免后来者按旧行实现。
