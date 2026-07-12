<p align="center">
  <img src="assets/banner.svg" alt="PINCER — IDEA-style Git conflict resolver" />
</p>

# PINCER · git-pincer-desktop

**English** · [简体中文](README.zh.md)

[![CI](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/zlx2019/git-pincer-desktop/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/zlx2019/git-pincer-desktop?include_prereleases)](https://github.com/zlx2019/git-pincer-desktop/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-8a8f98)

An IDEA-style Git conflict resolver for the desktop. Sister project of
[git-pincer](https://github.com/zlx2019/git-pincer) (CLI/TUI) — same feature set,
independent implementation.

Tauri 2 · SvelteKit (Svelte 5) · CodeMirror 6 · merge engine in Rust (`similar`).

<!-- TODO(before first release): screenshots in assets/screenshots/ — compact palette + three-pane merge, ideally a main-flow GIF -->

## Features

- **Compact command palette** (420×640, works as a side widget): `pull / merge / rebase /
  cherry-pick / revert` with ⌘1–⌘5 shortcuts and terminal-style output; whether an operation
  starts in the palette or in your terminal, **the app takes over and switches to the large
  window the moment conflicts appear**
- **Conflicts list**: multi-select Accept Yours / Theirs, group by directory, delete-conflict
  handling, binary pick-one, `--continue` loop (multi-round rebase relays automatically), Abort
- **Three-pane merge editor**: four-color chunk banding + word-level emphasis, seam ribbons with
  `≫ / ≪ / ✕ / ⟲` buttons, batch-apply non-conflicting, F7/⇧F7 navigation, a shared horizontal
  scrollbar, free editing in the center pane (touching a chunk marks it resolved), ⌘⏎ Apply
- **Merge engine** (Rust, pure functions): two line-level Myers diffs + base-range collision
  chunking — reports more conflicts rather than silently mis-merging; degrades to a whole-file
  conflict beyond 2 MB / 500 ms

Privacy: **no network, no telemetry**; git operations invoke your local git binary, so
credentials / hooks / rerere all follow your existing configuration.

Layered UI language: large windows (Conflicts / three-pane) keep the original IDEA English
(the 1:1 restoration baseline), small-window helper copy defaults to Chinese; switch to full
English in Settings.

Design baselines live in `docs/IDEA_STYLE.md` and `docs/PLAN.md`.

## Install

Grab the artifact for your platform from
[Releases](https://github.com/zlx2019/git-pincer-desktop/releases):

| Platform | Artifacts |
|---|---|
| macOS | `.dmg` |
| Windows | `.exe` (NSIS) / `.msi` |
| Linux | `.AppImage` / `.deb` / `.rpm` (requires `webkit2gtk`) |

Current builds are **not code-signed**, so the first launch needs a manual bypass (nothing is
broken — there is simply no paid certificate):

- **macOS**: on "damaged / unidentified developer", right-click the app → Open; if that fails run
  `xattr -dr com.apple.quarantine /Applications/PINCER.app`
- **Windows**: when SmartScreen blocks, click "More info → Run anyway"
- **Linux**: `chmod +x` the AppImage before running

## Try it in 30 seconds

```bash
scripts/demo-repo.sh          # creates /tmp/pincer-demo with ready-made conflicts
```

Open `/tmp/pincer-demo` in the app and launch **Merge branch → feature/merge** from the
palette — the takeover flow, conflicts list and three-pane editor are all one step away.

## Develop

```bash
pnpm install
pnpm tauri dev
```

Test data can also come from the sister repo's playground (merge / two-round rebase /
cherry-pick / revert / binary scenarios):

```bash
cd ../git-pincer && cargo run --example playground
cd /tmp/git-pincer-playground && git merge feature/merge   # or launch from the app menu
```

## Checks

```bash
pnpm check && pnpm test && pnpm build                      # svelte-check + vitest + frontend build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run                                          # engine unit tests + git plumbing integration tests
```

## Package & Release

```bash
pnpm tauri build    # local bundle: dmg / nsis+msi / AppImage+deb+rpm (per platform)
```

An official release = push a `v*` tag to run the four-platform `release.yml` matrix; the process
is documented in [CONTRIBUTING](CONTRIBUTING.md).

## FAQ

**Why only the conflict flow, and not a full git client?**
Positioning. Log / commit / push are already served well by your shell and existing GUIs; the
one moment that still hurts is conflict resolution. PINCER does exactly that slice — launch an
operation, take over conflicts, resolve in three panes, continue — and nothing else.

**How does it relate to IDEA's built-in merge tool?**
IDEA's resolver is the interaction benchmark (the UI is a deliberate 1:1 restoration), but you
get it without opening an IDE: a lightweight (~5 MB) native window over your own git binary,
with the same take-order semantics as the git-pincer CLI.

**macOS says the app is damaged / from an unidentified developer.**
Expected for unsigned builds — see [Install](#install). Right-click → Open once, or clear the
quarantine attribute.

**What does Linux need at runtime?**
`webkit2gtk` (the Tauri webview). The `.deb` / `.rpm` declare it as a dependency; for the
AppImage install it from your distribution's repositories.
