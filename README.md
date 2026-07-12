<p align="center">
  <img src="assets/banner.svg" alt="PINCER — IDEA-style Git conflict resolver" />
</p>

# PINCER · git-pincer-desktop

[![CI](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/zlx2019/git-pincer-desktop?include_prereleases)](https://github.com/zlx2019/git-pincer-desktop/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-8a8f98)

**English** | [简体中文](README.zh.md)

> **PINCER** takes its name from Rust's mascot — the crab. A Git conflict is two branches
> pinching the same piece of code at the same time, while a crab's pincer stands for stability,
> precision and control. May it grip both sides of every conflict firmly and carry the merge
> through.

The desktop version of [git-pincer](https://github.com/zlx2019/git-pincer) (CLI/TUI).

<!-- TODO(before first release): screenshots in assets/screenshots/ — compact palette + three-pane merge, ideally a main-flow GIF -->

## Features

- **Command palette** — compact 420×640 window with the five operations on ⌘1–⌘5 and
  terminal-style live output; usable as a desktop side widget
- **Automatic takeover** — conflicts born in the palette *or* in an external terminal switch
  the app to the large conflict window the moment they appear
- **Conflicts list** — multi-select Accept Yours / Theirs, directory grouping, delete-conflict
  handling, binary pick-one, multi-round `--continue` relay, Abort
- **Three-pane editor** — IDEA-style chunk bands with word-level emphasis, seam ribbons with
  apply / ignore / revert buttons, batch-apply, F7 navigation, a shared horizontal scrollbar,
  free editing in the result pane
- **Rust merge engine** — two line-level Myers diffs with base-range collision chunking;
  reports one conflict too many rather than silently mis-merging
- **Native git, zero magic** — everything shells out to your git binary, so credentials, hooks
  and rerere follow your existing configuration; no network, no telemetry

Dark / light theme, Chinese / English UI, editor font and close-to-tray behavior live in
Settings (⌘,).

## Install

Grab the artifact for your platform from
[Releases](https://github.com/zlx2019/git-pincer-desktop/releases):

| Platform | Artifacts |
|---|---|
| macOS | `.dmg` |
| Windows | `.exe` (NSIS) / `.msi` (stable releases only) |
| Linux | `.AppImage` / `.deb` / `.rpm` (requires `webkit2gtk`) |

Current builds are **not code-signed**, so the first launch needs a manual bypass:

- **macOS**: right-click the app → Open; if that fails run
  `xattr -dr com.apple.quarantine /Applications/PINCER.app`
- **Windows**: when SmartScreen blocks, click "More info → Run anyway"
- **Linux**: `chmod +x` the AppImage before running

## Try it in 30 seconds

```bash
scripts/demo-repo.sh          # creates /tmp/pincer-demo with ready-made conflicts
```

Open `/tmp/pincer-demo` in the app and launch **Merge branch → feature/merge** from the
palette — takeover, conflicts list and the three-pane editor are one step away.

## Develop

```bash
pnpm install        # Node >= 22, pnpm pinned via packageManager
pnpm tauri dev
```

Checks, commit conventions and the release process are documented in
[CONTRIBUTING](CONTRIBUTING.md).

## FAQ

**Why only the conflict flow, and not a full git client?**
Positioning. Log / commit / push are already served well by your shell and existing GUIs; the
one moment that still hurts is conflict resolution, so PINCER does exactly that slice.

**How does it relate to IDEA's built-in merge tool?**
IDEA's resolver is the interaction benchmark (the UI is a deliberate 1:1 restoration), but you
get it without opening an IDE — a lightweight native window over your own git binary.

**macOS says the app is damaged / from an unidentified developer.**
Expected for unsigned builds — see [Install](#install). Right-click → Open once, or clear the
quarantine attribute.

**What does Linux need at runtime?**
`webkit2gtk` (the Tauri webview). The `.deb` / `.rpm` declare it as a dependency; for the
AppImage install it from your distribution's repositories.

## License

[MIT](LICENSE) · built with [similar](https://github.com/mitsuhiko/similar),
[CodeMirror 6](https://codemirror.net) and [Tauri](https://tauri.app); bundled fonts
JetBrains Mono & Maple Mono (OFL 1.1), file icons from
[file-icons](https://github.com/file-icons) (ISC / MIT).
