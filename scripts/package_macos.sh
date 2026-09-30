#!/usr/bin/env bash
# Builds a universal macOS .app (arm64 + x86_64) and zips it. Run on macOS.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
OUT=${1:-dist/macos}
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --bin finakids --target aarch64-apple-darwin
cargo build --release --bin finakids --target x86_64-apple-darwin
APP="$OUT/Finakids.app"
rm -rf "$APP" && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/finakids" \
  target/aarch64-apple-darwin/release/finakids target/x86_64-apple-darwin/release/finakids
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/${GITHUB_RUN_NUMBER:-1}/" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
ICONSET=$(mktemp -d)/AppIcon.iconset && mkdir -p "$ICONSET"
cp packaging/icons/icon_16.png "$ICONSET/icon_16x16.png"
cp packaging/icons/icon_32.png "$ICONSET/icon_16x16@2x.png"
cp packaging/icons/icon_32.png "$ICONSET/icon_32x32.png"
cp packaging/icons/icon_64.png "$ICONSET/icon_32x32@2x.png"
cp packaging/icons/icon_128.png "$ICONSET/icon_128x128.png"
cp packaging/icons/icon_256.png "$ICONSET/icon_128x128@2x.png"
cp packaging/icons/icon_256.png "$ICONSET/icon_256x256.png"
cp packaging/icons/icon_512.png "$ICONSET/icon_256x256@2x.png"
cp packaging/icons/icon_512.png "$ICONSET/icon_512x512.png"
cp packaging/icons/icon_1024.png "$ICONSET/icon_512x512@2x.png"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
# ad-hoc signature so Apple Silicon accepts the binary
codesign --force --deep --sign - "$APP" || true
(cd "$OUT" && ditto -c -k --sequesterRsrc --keepParent Finakids.app Finakids-macos-universal.zip)
ls -la "$OUT"
