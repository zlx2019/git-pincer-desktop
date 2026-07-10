# PINCER · git-pincer-desktop

IDEA 风格的 Git 冲突解决桌面端。[git-pincer](https://github.com/zlx2019/git-pincer) (CLI/TUI) 的姊妹项目——功能对齐、独立实现。

Tauri 2 · SvelteKit (Svelte 5) · CodeMirror 6 · 合并引擎在 Rust (`similar`)。

## 功能

- **小窗指令面板** (420×640, 可当侧边小工具): `pull / merge / rebase / cherry-pick / revert`, ⌘1–⌘5 快捷键, 终端式执行输出; 无论操作在面板里发起还是在终端里发起, **一旦出现冲突立刻接管并切换大窗**
- **Conflicts 列表**: 多选 Accept Yours / Theirs、目录分组、删除冲突处理、二进制 pick-one、`--continue` 循环 (多轮 rebase 自动接力)、Abort
- **三栏合并编辑器**: chunk 四色底纹 + 词级强调、接缝连接带与 `≫ / ≪ / ✕ / ⟲` 按钮、批量应用非冲突、F7/⇧F7 导航、中栏自由编辑 (命中 chunk 即记为已解决)、⌘⏎ Apply 落盘
- **合并引擎** (Rust, 纯函数): 两次行级 Myers diff + base 区间碰撞分块, 宁可多报冲突不静默错合; >2MB / 500ms 超时降级整文件冲突

设计基准见 `docs/IDEA_STYLE.md` 与 `docs/PLAN.md`。

## 开发

```bash
pnpm install
pnpm tauri dev
```

测试数据用主仓库的演练场生成 (含 merge / 两轮 rebase / cherry-pick / revert / 二进制场景):

```bash
cd ../git-pincer && cargo run --example playground
cd /tmp/git-pincer-playground && git merge feature/merge   # 或直接在应用菜单里发起
```

## 检查

```bash
pnpm check && pnpm build                                   # svelte-check + 前端构建
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run                                          # 引擎单测 + git 管道集成测试
```

## 打包

```bash
pnpm tauri build    # dmg / nsis / AppImage+deb (按平台)
```
