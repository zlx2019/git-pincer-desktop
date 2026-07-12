#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

V="${1:-}"
if [[ ! "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "usage: scripts/bump.sh <semver>    e.g. scripts/bump.sh 0.2.0" >&2
  exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree not clean — commit or stash first" >&2
  exit 1
fi
if git rev-parse -q --verify "refs/tags/v$V" >/dev/null; then
  echo "error: tag v$V already exists" >&2
  exit 1
fi

python3 - "$V" <<'EOF'
import re, sys

v = sys.argv[1]
s = open('package.json').read()
s, n = re.subn(r'("version"\s*:\s*")[^"]+(")', rf'\g<1>{v}\g<2>', s, count=1)
assert n == 1, 'package.json: version field not found'
open('package.json', 'w').write(s)

s = open('src-tauri/Cargo.toml').read()
s, n = re.subn(r'(?m)^version = "[^"]+"$', f'version = "{v}"', s, count=1)
assert n == 1, 'Cargo.toml: version field not found'
open('src-tauri/Cargo.toml', 'w').write(s)
EOF

# Cargo.lock 里本包的版本条目跟上(metadata 会顺手同步 lockfile, 无需编译)
(cd src-tauri && cargo metadata --format-version 1 --offline >/dev/null 2>&1) ||
  (cd src-tauri && cargo metadata --format-version 1 >/dev/null)

git cliff --tag "v$V" -o CHANGELOG.md

git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md
git commit -m "chore(release): v$V"
git tag "v$V"

echo
echo "✔ v$V committed and tagged. To publish:"
echo "    git push && git push --tags"
