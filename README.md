# PINCER · git-pincer-desktop

[![CI](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/zlx2019/git-pincer-desktop?include_prereleases)](https://github.com/zlx2019/git-pincer-desktop/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-8a8f98)

IDEA 风格的 Git 冲突解决桌面端。[git-pincer](https://github.com/zlx2019/git-pincer) (CLI/TUI) 的姊妹项目——功能对齐、独立实现。

Tauri 2 · SvelteKit (Svelte 5) · CodeMirror 6 · 合并引擎在 Rust (`similar`)。

<!-- TODO(首个 release 前): assets/screenshots/ 补 菜单小窗 + 三栏大窗 截图, 更好是一张主流程 GIF -->

## 功能

- **小窗指令面板** (420×640, 可当侧边小工具): `pull / merge / rebase / cherry-pick / revert`, ⌘1–⌘5 快捷键, 终端式执行输出; 无论操作在面板里发起还是在终端里发起, **一旦出现冲突立刻接管并切换大窗**
- **Conflicts 列表**: 多选 Accept Yours / Theirs、目录分组、删除冲突处理、二进制 pick-one、`--continue` 循环 (多轮 rebase 自动接力)、Abort
- **三栏合并编辑器**: chunk 四色底纹 + 词级强调、接缝连接带与 `≫ / ≪ / ✕ / ⟲` 按钮、批量应用非冲突、F7/⇧F7 导航、底部共享横向滚动条、中栏自由编辑 (命中 chunk 即记为已解决)、⌘⏎ Apply 落盘
- **合并引擎** (Rust, 纯函数): 两次行级 Myers diff + base 区间碰撞分块, 宁可多报冲突不静默错合; >2MB / 500ms 超时降级整文件冲突

隐私: **不联网、无遥测**; git 操作直接调用你本机的 git 二进制, 凭据 / 钩子 / rerere 全走你的现有配置。

界面语言分层: 大窗 (Conflicts / 三栏) 保持 IDEA 英文原文 (1:1 还原基准), 小窗辅助文案默认中文; 设置里可切全英文。

设计基准见 `docs/IDEA_STYLE.md` 与 `docs/PLAN.md`。

## 安装

从 [Releases](https://github.com/zlx2019/git-pincer-desktop/releases) 下载对应平台的产物:

| 平台 | 产物 |
|---|---|
| macOS | `.dmg` |
| Windows | `.exe` (NSIS) / `.msi` |
| Linux | `.AppImage` / `.deb` / `.rpm` (依赖 `webkit2gtk`) |

当前产物**未做代码签名**, 首次打开需要手动放行 (不是损坏, 是没花钱买证书):

- **macOS**: 提示"已损坏/无法验证开发者"时, 右键 App → 打开; 仍不行则执行
  `xattr -dr com.apple.quarantine /Applications/git-pincer-desktop.app`
- **Windows**: SmartScreen 拦截时点 "更多信息 → 仍要运行"
- **Linux**: AppImage 先 `chmod +x` 再运行

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
pnpm check && pnpm test && pnpm build                      # svelte-check + vitest + 前端构建
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run                                          # 引擎单测 + git 管道集成测试
```

## 打包与发布

```bash
pnpm tauri build    # 本机打包: dmg / nsis+msi / AppImage+deb+rpm (按平台)
```

正式发布 = 推 `v*` 标签走 `release.yml` 四平台矩阵, 流程见 [CONTRIBUTING](CONTRIBUTING.md)。
