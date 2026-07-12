<p align="center">
  <img src="assets/banner.svg" alt="PINCER — IDEA-style Git conflict resolver" />
</p>

# PINCER · git-pincer-desktop

[![CI](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/zlx2019/git-pincer-desktop?include_prereleases)](https://github.com/zlx2019/git-pincer-desktop/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-8a8f98)

[English](README.md) | **简体中文**

> **PINCER** 名字取自 Rust 的吉祥物——螃蟹。Git 冲突就像两个分支同时"夹"住同一段代码,
> 而蟹钳意味着稳定、精准与掌控。愿它像一把可靠的钳子, 牢牢夹住每一处冲突的两侧,
> 帮你更高效地理解差异、完成合并。

[git-pincer](https://github.com/zlx2019/git-pincer) (CLI/TUI) 的桌面端姊妹项目, 给想在真正的
窗口里解冲突的时刻: 发起操作、接管冲突、三栏解决、continue——不做 log、不做 commit、不做 push。

<!-- TODO(首个 release 前): assets/screenshots/ 补 菜单小窗 + 三栏大窗 截图, 更好是一张主流程 GIF -->

## 功能

- **指令面板** —— 420×640 紧凑小窗, 五个操作挂 ⌘1–⌘5, 终端式实时输出; 可当桌面侧边小工具
- **自动接管** —— 无论冲突产生自面板还是外部终端, 出现的瞬间切换到大窗冲突界面
- **冲突列表** —— 多选 Accept Yours / Theirs、目录分组、删除冲突处理、二进制 pick-one、
  多轮 `--continue` 接力、Abort
- **三栏编辑器** —— IDEA 式 chunk 色带 + 词级强调、接缝连接带的应用/忽略/撤销按钮、批量应用、
  F7 导航、底部共享横向滚动条、结果栏自由编辑
- **Rust 合并引擎** —— 两次行级 Myers diff + base 区间碰撞分块; 宁可多报一个冲突, 不静默错合
- **原生 git、零魔法** —— 一切经由你本机的 git 二进制, 凭据、钩子、rerere 全走现有配置;
  不联网、无遥测

深色/浅色主题、中英界面、编辑器字体、关窗收托盘, 都在设置里 (⌘,)。

## 安装

从 [Releases](https://github.com/zlx2019/git-pincer-desktop/releases) 下载对应平台的产物:

| 平台 | 产物 |
|---|---|
| macOS | `.dmg` |
| Windows | `.exe` (NSIS) / `.msi` |
| Linux | `.AppImage` / `.deb` / `.rpm` (依赖 `webkit2gtk`) |

当前产物**未做代码签名**, 首次打开需要手动放行:

- **macOS**: 右键 App → 打开; 仍不行则执行
  `xattr -dr com.apple.quarantine /Applications/PINCER.app`
- **Windows**: SmartScreen 拦截时点 "更多信息 → 仍要运行"
- **Linux**: AppImage 先 `chmod +x` 再运行

## 30 秒上手

```bash
scripts/demo-repo.sh          # 在 /tmp/pincer-demo 造好一个带现成冲突的演示仓库
```

在应用里打开 `/tmp/pincer-demo`, 从面板发起 **合并分支 → feature/merge**——接管、冲突列表、
三栏编辑一步到位。

## 开发

```bash
pnpm install        # Node >= 22, pnpm 版本由 packageManager 锁定
pnpm tauri dev
```

检查项、提交规范与发布流程见 [CONTRIBUTING](CONTRIBUTING.md)。

## FAQ

**为什么只做冲突流程, 不做完整 git 客户端?**
定位使然。log / commit / push 在终端和现有 GUI 里已经足够好, 真正疼的只有解冲突这一刻,
PINCER 只做这一段。

**和 IDEA 内置合并工具是什么关系?**
IDEA 的解决器是交互基准 (UI 刻意 1:1 还原), 但不用为它打开一个 IDE——一个轻量原生窗口,
直接驱动你本机的 git。

**macOS 提示"已损坏/无法验证开发者"?**
未签名产物的预期行为, 见[安装](#安装)——右键 → 打开一次, 或清除 quarantine 属性。

**Linux 运行时需要什么?**
`webkit2gtk` (Tauri 的 webview)。`.deb` / `.rpm` 已声明依赖; AppImage 用户从发行版仓库安装即可。

## License

[MIT](LICENSE) · 基于 [similar](https://github.com/mitsuhiko/similar)、
[CodeMirror 6](https://codemirror.net) 与 [Tauri](https://tauri.app); 内嵌字体
JetBrains Mono 与 Maple Mono (OFL 1.1), 文件图标来自
[file-icons](https://github.com/file-icons) (ISC / MIT)。
