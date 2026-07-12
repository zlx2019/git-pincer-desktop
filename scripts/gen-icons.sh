#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

command -v rsvg-convert >/dev/null || { echo "need librsvg: brew install librsvg" >&2; exit 1; }
command -v magick >/dev/null || { echo "need imagemagick: brew install imagemagick" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 全平台图标由 tauri icon 从满幅 1024 直出 (Windows/Linux 惯例即满幅)
rsvg-convert -w 1024 -h 1024 assets/icon.svg -o "$tmp/full.png"
pnpm tauri icon "$tmp/full.png"

# macOS icns 单独重做: Apple 网格要求图形 824/1024 居中留白, 满幅会在程序坞里大一圈
magick "$tmp/full.png" -resize 824x824 -background none -gravity center -extent 1024x1024 "$tmp/pad.png"
mkdir "$tmp/icon.iconset"
for s in 16 32 128 256 512; do
  magick "$tmp/pad.png" -resize "${s}x${s}" "$tmp/icon.iconset/icon_${s}x${s}.png"
  magick "$tmp/pad.png" -resize "$((s * 2))x$((s * 2))" "$tmp/icon.iconset/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$tmp/icon.iconset" -o src-tauri/icons/icon.icns

# macOS 托盘 template 剪影 (尺寸规范见 assets/tray.svg 注释)
rsvg-convert -w 44 -h 44 assets/tray.svg -o src-tauri/icons/tray.png

echo "✔ icons regenerated (icns padded to the Apple grid, tray 44px)"
