#!/usr/bin/env bash
# Compila Finakids para Windows x64 desde Linux, sin sudo:
# Rust (x86_64-pc-windows-gnu) + Zig como enlazador, vía cargo-zigbuild.
# usage: build_windows.sh [out_dir]   (por defecto dist/downloads)
# Deja Finakids-windows-x64.zip con Finakids/finakids.exe (icono incluido por build.rs).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)
source scripts/toolchains.env
OUT=${1:-dist/downloads}
TARGET=x86_64-pc-windows-gnu

fail() { echo "error: $*" >&2; exit 1; }
command -v zig >/dev/null 2>&1 || fail "no se encontró zig (instálalo en ~/.local/opt/zig-*; ver README)"
command -v cargo-zigbuild >/dev/null 2>&1 || fail "falta cargo-zigbuild: cargo install cargo-zigbuild"
rustup target list --installed | grep -qx "$TARGET" || rustup target add "$TARGET"

export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target-win}
cargo zigbuild --release --bin finakids --target "$TARGET"

EXE="$CARGO_TARGET_DIR/$TARGET/release/finakids.exe"
[ -f "$EXE" ] || fail "no se generó $EXE"
PKG="$CARGO_TARGET_DIR/pkg"
rm -rf "$PKG" && mkdir -p "$PKG/Finakids"
cp "$EXE" "$PKG/Finakids/"
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
rm -f "$OUT/Finakids-windows-x64.zip"
(cd "$PKG" && zip -q -r -D -X "$OUT/Finakids-windows-x64.zip" Finakids)
echo "Listo: $OUT/Finakids-windows-x64.zip"
unzip -l "$OUT/Finakids-windows-x64.zip"
