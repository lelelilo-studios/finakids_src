#!/usr/bin/env bash
# Compila el APK de Android (arm64) en local con cargo-apk, firmado con el keystore de publicación.
# usage: build_android.sh [out_dir]   (por defecto dist/downloads)
#
# Variables:
#   CARGO_APK_RELEASE_KEYSTORE           ruta del keystore PKCS12
#                                        (por defecto ~/.local/share/finakids/finakids-release.p12)
#   CARGO_APK_RELEASE_KEYSTORE_PASSWORD  contraseña del keystore (obligatoria)
#   FINAKIDS_KEYSTORE_PASS_FILE          alternativa: archivo (chmod 600) con la contraseña
#                                        (por defecto ~/.local/share/finakids/keystore.pass, si existe)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)
source scripts/toolchains.env
OUT=${1:-dist/downloads}
TARGET=aarch64-linux-android

fail() { echo "error: $*" >&2; exit 1; }
command -v cargo-apk >/dev/null 2>&1 || fail "falta cargo-apk: cargo install cargo-apk --locked"
command -v java >/dev/null 2>&1 || fail "no se encontró Java 17 (JAVA_HOME; ver README)"
[ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME" ] || fail "no se encontró el SDK de Android (ANDROID_HOME; ver README)"
[ -n "${ANDROID_NDK_ROOT:-}" ] && [ -d "$ANDROID_NDK_ROOT" ] || fail "no se encontró el NDK de Android (ANDROID_NDK_ROOT; ver README)"
rustup target list --installed | grep -qx "$TARGET" || rustup target add "$TARGET"

export CARGO_APK_RELEASE_KEYSTORE=${CARGO_APK_RELEASE_KEYSTORE:-$HOME/.local/share/finakids/finakids-release.p12}
PASS_FILE=${FINAKIDS_KEYSTORE_PASS_FILE:-$HOME/.local/share/finakids/keystore.pass}
if [ -z "${CARGO_APK_RELEASE_KEYSTORE_PASSWORD:-}" ] && [ -f "$PASS_FILE" ]; then
  CARGO_APK_RELEASE_KEYSTORE_PASSWORD=$(head -n1 "$PASS_FILE")
fi
if [ -z "${CARGO_APK_RELEASE_KEYSTORE_PASSWORD:-}" ]; then
  cat >&2 <<MSG
error: falta la contraseña del keystore de publicación.

El APK debe firmarse con la misma clave que el APK ya publicado; si no, Android
rechaza la actualización y habría que desinstalar el juego (perdiendo la partida).

Define la variable y vuelve a ejecutar:
  CARGO_APK_RELEASE_KEYSTORE_PASSWORD='<contraseña>' ./scripts/build_android.sh
o guarda la contraseña en $PASS_FILE (chmod 600).
La contraseña es la del secreto ANDROID_KEYSTORE_PASSWORD de los repos de GitHub.
Keystore: $CARGO_APK_RELEASE_KEYSTORE
MSG
  exit 2
fi
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD
[ -f "$CARGO_APK_RELEASE_KEYSTORE" ] || fail "no existe el keystore $CARGO_APK_RELEASE_KEYSTORE"

export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$ROOT/target-android}
cargo apk build --release --lib

APK=$(find "$CARGO_TARGET_DIR/release/apk" -maxdepth 1 -name 'finakids.apk' 2>/dev/null | head -1)
[ -n "$APK" ] && [ -f "$APK" ] || fail "no se generó el APK"
mkdir -p "$OUT"
cp "$APK" "$OUT/Finakids-android-arm64.apk"
echo "Listo: $OUT/Finakids-android-arm64.apk"
if [ -n "${FINAKIDS_BUILD_TOOLS:-}" ] && [ -x "$FINAKIDS_BUILD_TOOLS/apksigner" ]; then
  "$FINAKIDS_BUILD_TOOLS/apksigner" verify --print-certs "$OUT/Finakids-android-arm64.apk" | grep -E "SHA-256|DN" || true
  "$FINAKIDS_BUILD_TOOLS/aapt" dump badging "$OUT/Finakids-android-arm64.apk" | grep -E "^package|^sdkVersion|^native-code" || true
fi
