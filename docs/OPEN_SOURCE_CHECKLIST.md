# 开源成熟度清单 (2026-07-12 盘点)

> 目标: 作为一个"合格标准"的开源桌面应用, 该有的都有。本文列出现状盘点与缺口,
> 按优先级排序; 每项给出做法与规格, 完成后勾选并把定稿决定回写 PLAN.md。

## 0. 已具备 (不用重复建设)

| 类别 | 已有 |
|---|---|
| 法务 | `LICENSE` (MIT) · 字体 OFL 许可证随附 (`static/fonts/OFL-*.txt`, JetBrains Mono + Maple Mono) |
| 社区 | `CONTRIBUTING.md` · `SECURITY.md` · issue 模板 ×2 · PR 模板 · dependabot |
| 工程 | CI (fmt/clippy/nextest/doc/deny/typos/svelte-check/vitest/build) · release.yml 四平台矩阵 · git-cliff · pre-commit · `.editorconfig` · `rust-toolchain.toml` |
| 品牌 | 应用图标全尺寸 (源 `assets/icon.svg`; v2 2026-07-12 Zero 设计: 深蓝底+品牌橙齿轮盘+merge 分叉图, 含 Windows Store 全尺寸) · favicon 同步 · 托盘 template 剪影同构 (`assets/tray.svg`) |
| 文档 | README (中文) · `docs/PLAN.md` (技术方案) · `docs/IDEA_STYLE.md` (样式基准) · `CLAUDE.md` |

---

## 1. P0 — 首个正式 Release 前必须补齐

### 1.1 README 门面三件套: 截图 / 徽章 / 安装说明 — 🔶 部分完成 (2026-07-12)
✅ 徽章 (CI/Release/License/平台) · 安装节 (Releases 链接 + 平台产物表) · 过时修正 (pnpm test / rpm) 已落 README;
⏳ **截图与主流程 GIF 待 Zero 出素材** (README 里留了 TODO 注释位)。
桌面应用的 README 没有截图 = 没有转化率, 这是当前最大的缺口。

- **截图**: 至少 2 张 (菜单小窗 + 三栏合并大窗), 存 `assets/screenshots/`;
  更好的是一张 10–20s 的 GIF/WebP 走完 "发起 merge → 接管 → 三栏解决 → continue" 主流程
  (macOS 用 `⌘⇧5` 录屏, `gifski` 转 GIF; 控制在 5MB 内)。
- **徽章**: CI 状态 · Release 版本 · License · 平台支持 (macOS/Windows/Linux), 放标题下一行。
- **面向用户的安装节**: 现在 README 只有"开发/打包", 没有"用户怎么装"。
  加 Install 节链接到 GitHub Releases, 按平台列产物 (dmg / nsis+msi / AppImage+deb+rpm),
  并附 **未签名产物的放行说明** (见 1.3)。
- 顺手修两处过时: 检查节缺 `pnpm test`; 打包产物列表缺 rpm。

### 1.2 CHANGELOG.md 落库 — ✅ 完成 (2026-07-12)
`git cliff -o CHANGELOG.md` 已入库; 发版时按 CONTRIBUTING 的 Release Process 用 `git cliff --tag` 重新生成。
cliff 目前只在打 tag 时生成 release notes, 仓库里没有可浏览的历史。

- `git cliff -o CHANGELOG.md` 一次性生成, 提交入库;
- release.yml 加一步在发版时重新生成并随版本提交 (或发版前手动跑), 保证与 tag 同步。

### 1.3 未签名分发的用户文档 (macOS Gatekeeper / Windows SmartScreen) — 🔶 README 部分完成 (2026-07-12)
✅ 放行说明已写进 README 安装节; ⏳ 签名/公证待 Zero 的开发者账号。
没有付费开发者账号前, 下载的 dmg 会提示"已损坏/无法验证开发者", exe 会被 SmartScreen 拦。
这不是 bug, 但**必须写出来**, 否则第一批用户直接流失:

- README 安装节写明: macOS 右键 → 打开, 或 `xattr -dr com.apple.quarantine /Applications/git-pincer-desktop.app`;
  Windows 点"更多信息 → 仍要运行";
- 长期项: Apple Developer ID 签名 + 公证 (release.yml 已是矩阵, 加签名步骤即可, 需账号与证书 secrets),
  Windows 代码签名证书 (成本更高, 可后置)。

### 1.4 bundle 元数据补全 (`src-tauri/Tauri.toml`) — ✅ 完成 (2026-07-12)
publisher / copyright / homepage / license / license-file 已进 `[bundle]`。
Linux 包管理器与 Windows 安装器会展示这些字段, 现在只有 category:

```jsonc
"bundle": {
  "publisher": "Zero",
  "copyright": "Copyright © 2026 Zero",
  "homepage": "https://github.com/zlx2019/git-pincer-desktop",
  "licenseFile": "../LICENSE"   // nsis/deb 会内嵌
}
```

### 1.5 CODE_OF_CONDUCT.md — ✅ 完成 (2026-07-12, Contributor Covenant 2.1, 联系邮箱 = git 身份邮箱)
社区标配三件套的最后一件 (Contributor Covenant 2.1 模板直接用, GitHub 社区检查项会亮绿)。

### 1.6 版本号三处同步的发布流程 — ✅ 完成 (2026-07-12, 写入 CONTRIBUTING 的 Release Process 节)
`package.json` / `src-tauri/Cargo.toml` / `src-tauri/Tauri.toml` 三处版本要一致。
在 CONTRIBUTING 或 README 写死发布流程:

```bash
# 1. 三处 bump 版本 (可写个 scripts/bump.sh 或用 cargo-edit + npm version)
# 2. git cliff -o CHANGELOG.md && git commit
# 3. git tag v0.x.y && git push --tags   → release.yml 出四平台产物
```

---

## 2. P1 — 强烈建议 (你点名的都在这)

### 2.1 托盘专用单色 template 图标 (macOS 菜单栏惯例) — ✅ 完成 (2026-07-12)
源 `assets/tray.svg` (与 icon.svg 同构型剪影, 线宽加粗), 产物 `src-tauri/icons/tray.png` (22pt@2x);
macOS `icon_as_template(true)` 内嵌加载 (tauri 开 `image-png` feature), 其余平台保留彩色应用图标。
现在托盘直接用彩色应用图标, 在 macOS 菜单栏里像个"外来户"。
惯例是**单色剪影 + template 模式**, 随亮暗菜单栏自动反色:

- 出一张 PINCER 钳臂轮廓的纯黑透明 PNG (22×22@1x + 44×44@2x, 或 SVG 转);
- Rust 侧 `TrayIconBuilder.icon_as_template(true)` (macOS), Windows/Linux 继续用彩色版;
- 源文件放 `assets/tray.svg`, 与 icon.svg 同风格。

### 2.2 冲突列表按文件类型出图标 (对齐 IDEA) — ✅ 完成 (2026-07-12)
`src/lib/fileicon.ts`: 扩展名 → 字母徽章/图形 SVG + 既有 token 低饱和品类色 (rs/ts/js/svelte/vue/json/toml/yaml/lock/md/txt/html/xml/css/py/go/java/kt/c/cpp/sh + 图片类), 未知与二进制回落通用文档图标; 冲突列表与三栏 header 共用, vitest 覆盖映射。
现在所有文件共用一枚通用文档图标 (`conflicts/+page.svelte` 的 `.ficon`)。
IDEA 的列表是按类型着色图标, 辨识度高很多。做法 (不违反"无 UI 组件库"约定):

- `src/lib/fileicon.ts`: 扩展名 → 手写 16×16 线条 SVG 字面量 + 品类色;
  第一批覆盖: rs / ts·js·svelte / json·toml·yaml / md / html·css / py / go / java·kt /
  sh / lock / 图片类 / 二进制兜底; 未知扩展名回落现有通用图标;
- 色彩用主题 token 的低饱和版, 不抢冲突红文件名的视觉焦点;
- 顺手给三栏页 header 的文件名也用上同一枚图标。

### 2.3 README 顶部品牌横幅 + GitHub 社交预览图
- README 标题上方放一张横幅 (logo + 一句话定位, 深色底, `assets/banner.svg` 或 png);
- GitHub 仓库 Settings → Social preview 上传 1280×640 社图 (被分享到任何平台时的门面),
  内容可以就是横幅加一张三栏截图。

### 2.4 英文 README
Tauri/Rust 社区受众大半英文。推荐 `README.md` 英文为主 (国际默认) + `README.zh.md` 中文,
或反之但顶部互链。UI 本身的中英分层 (大窗英文/小窗中文) 也值得在 README 里说明一句。

### 2.5 GitHub 仓库侧设置 (代码之外, 五分钟)
- About: 描述 + 官网留空/文档链 + topics (`tauri` `svelte` `codemirror` `git` `merge-conflicts` `rust` `desktop`);
- Releases 置顶说明; Discussions 开不开想好 (开了就在 issue 模板 config.yml 里把"提问"引流过去);
- 分支保护: main 要求 CI 绿 + PR (单人项目可只开 CI 必须绿)。

---

## 3. P2 — 锦上添花 (成熟项目的"没想到"清单)

| 项 | 说明 |
|---|---|
| 自动更新 | `tauri-plugin-updater` + release.yml 产 `latest.json`; 桌面工具的留存关键。需要 updater 签名密钥对, 建议 P0 做完就排上 |
| dmg 安装体验 | tauri dmg 配置支持背景图 + 图标摆位 (`bundle.macOS.dmg`), 一张 "拖到 Applications" 的深色背景图即可 |
| About 面板 | macOS 默认菜单的 About 显示包名裸信息; 应用内加"关于"(版本/许可证/主页/致谢 similar·CM6·JetBrains Mono) 更体面 |
| 产品显示名 | `productName` 目前是 `git-pincer-desktop`, .app/菜单栏就叫这个; 若想显示 "PINCER" 需改 productName (影响产物文件名, 发布前定, 之后别动) |
| issue 表单化 | `.github/ISSUE_TEMPLATE/*.md` 升级为 `.yml` 表单 (必填项/下拉选平台版本), 提质 bug 报告 |
| FUNDING.yml | 想接受赞助就加 (GitHub Sponsors / 爱发电), 不想就跳过 |
| 隐私一句话 | ✅ (2026-07-12) README 已写明 "不联网、无遥测、git 凭据走你本机配置" |
| 性能/体积卖点 | README 放硬数字: .app 5.1MB、启动 <1s、纯 Rust diff 引擎——对比 Electron 类是差异化卖点 |
| FAQ | "为什么只做冲突流程不做 git 客户端" (定位)、"和 IDEA 内置合并的关系"、Gatekeeper 放行、Linux 依赖 (webkit2gtk) |
| 演示仓库 | README 里 playground 依赖姊妹仓库源码; 提供一个 `scripts/demo-repo.sh` 直接造演示冲突, 降低尝鲜门槛 |
| win/linux 实测记录 | release 矩阵会出包但没人验证过; 找台 Windows/Linux 跑一遍冒烟, README 标注"已验证平台" |
| 代码统计徽章/覆盖率 | 可选; 单人项目性价比低, 放最后 |

---

## 4. 建议的推进顺序

1. **P0 全部** → 打 `v0.1.0` 首个 Release (含放行说明);
2. P1 的 2.1 托盘图标 + 2.2 文件类型图标 (你点名的两项, 纯代码可直接做) + 2.3 横幅/社图;
3. P1 的英文 README + 仓库设置;
4. P2 按兴趣挑: 自动更新 > About 面板 > dmg 背景 > 其余。

其中 2.1 / 2.2 / 1.1 的过时修正 / 1.2 / 1.4 / 1.5 / 1.6 是纯代码活, 随时可以让我直接做;
logo 横幅、截图、GIF、社图需要你的审美定夺 (我可以出初稿); 签名/公证需要你的开发者账号。
