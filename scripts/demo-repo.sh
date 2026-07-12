#!/usr/bin/env bash
# 造一个带现成 merge 冲突的演示仓库(默认 /tmp/pincer-demo), 30 秒体验 PINCER:
# 文本冲突 ×2 (hello.rs / notes.txt) + 二进制冲突 ×1 (logo.bin, 走 pick-one)。
# 仅用本地 git 配置的最小覆盖保证确定性, 不碰你的全局配置。
set -euo pipefail

DIR="${1:-/tmp/pincer-demo}"

if [ -e "$DIR" ]; then
  echo "error: $DIR already exists — remove it first: rm -rf $DIR" >&2
  exit 1
fi

git init -q -b main "$DIR"
cd "$DIR"
git config user.name "PINCER Demo"
git config user.email "demo@pincer.local"
git config commit.gpgsign false
git config core.hooksPath /dev/null
git config rerere.enabled false

# --- base ---------------------------------------------------------------
cat > hello.rs <<'EOF'
//! Demo file for PINCER.

/// Greeting entry.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

/// Config file location.
pub fn config_path() -> &'static str {
    "/etc/demo.toml"
}
EOF
cat > notes.txt <<'EOF'
release checklist
- run tests
- tag version
EOF
printf 'BIN\x00v1' > logo.bin
git add . && git commit -qm "chore: base"

# --- feature/merge: 对同一批行做改动 --------------------------------------
git switch -qc feature/merge
sed -i.bak 's/Hello, {name}!/Hey {name}, ready to merge?/' hello.rs
sed -i.bak 's|/etc/demo.toml|/etc/demo/config.toml|' hello.rs
sed -i.bak 's/- run tests/- run tests on every platform/' notes.txt
rm -f hello.rs.bak notes.txt.bak
printf 'BIN\x00v2-theirs' > logo.bin
git commit -aqm "feat: rework greeting and config path"

# --- main: 对同一批行做不同的改动 -----------------------------------------
git switch -q main
sed -i.bak 's/Hello, {name}!/Good morning, {name}./' hello.rs
sed -i.bak 's|/etc/demo.toml|/opt/demo.toml|' hello.rs
sed -i.bak 's/- run tests/- run unit and integration tests/' notes.txt
rm -f hello.rs.bak notes.txt.bak
printf 'BIN\x00v2-ours' > logo.bin
git commit -aqm "feat: morning greeting and /opt config"

echo "demo repo ready: $DIR"
echo "  1) open it in PINCER"
echo "  2) launch 合并分支/Merge branch → feature/merge from the palette"
echo "     (or run: cd $DIR && git merge feature/merge)"
