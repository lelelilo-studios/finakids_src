#!/usr/bin/env bash
# Builds an unsigned iOS .ipa (device) and a simulator .app zip. Run on macOS.
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
OUT=${1:-dist/ios}
mkdir -p "$OUT"
bundle() {
  local target=$1 platform=$2 dest=$3
  cargo build --release --bin finakids --target "$target"
  local app="$dest/Finakids.app"
  rm -rf "$app" && mkdir -p "$app"
  cp "target/$target/release/finakids" "$app/finakids"
  sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/${GITHUB_RUN_NUMBER:-1}/" -e "s/__PLATFORM__/$platform/" packaging/ios/Info.plist > "$app/Info.plist"
  cp packaging/icons/icon_120.png "$app/AppIcon60x60@2x.png"
  cp packaging/icons/icon_180.png "$app/AppIcon60x60@3x.png"
  cp packaging/icons/icon_152.png "$app/AppIcon76x76@2x~ipad.png"
  cp packaging/icons/icon_167.png "$app/AppIcon83.5x83.5@2x~ipad.png"
  printf 'APPL????' > "$app/PkgInfo"
}
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
TMP=$(mktemp -d)
mkdir -p "$TMP/device/Payload" "$TMP/sim"
bundle aarch64-apple-ios iPhoneOS "$TMP/device/Payload"
(cd "$TMP/device" && zip -qr "$OLDPWD/$OUT/Finakids-ios-unsigned.ipa" Payload)
bundle aarch64-apple-ios-sim iPhoneSimulator "$TMP/sim"
codesign --force --sign - "$TMP/sim/Finakids.app" || true
(cd "$TMP/sim" && zip -qr "$OLDPWD/$OUT/Finakids-ios-simulator.zip" Finakids.app)
ls -la "$OUT"
