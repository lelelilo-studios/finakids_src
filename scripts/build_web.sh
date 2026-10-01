#!/usr/bin/env bash
# Builds the web version into dist/web (WebGPU with WebGL2 fallback).
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=${1:-dist/web}
cargo build --lib --release --target wasm32-unknown-unknown
mkdir -p "$OUT/pkg"
wasm-bindgen --target web --no-typescript --out-dir "$OUT/pkg" target/wasm32-unknown-unknown/release/finakids.wasm
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O2 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
    "$OUT/pkg/finakids_bg.wasm" -o "$OUT/pkg/finakids_bg.wasm" || echo "wasm-opt failed, keeping unoptimized wasm"
fi
cp web/index.html web/worker.js web/manifest.webmanifest "$OUT/"
cp -r web/icons "$OUT/"
touch "$OUT/.nojekyll"
ls -la "$OUT" "$OUT/pkg"
