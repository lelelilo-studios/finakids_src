#!/usr/bin/env bash
# Compila en esta máquina todo lo que se puede (Web, Linux, Windows y Android)
# y publica el sitio en el repo público, conservando las descargas que aquí no
# se pueden generar (macOS e iOS necesitan una Mac).
#
# uso: publish_local.sh [--no-deploy]
#   SITE_KEY=<clave SSH de despliegue con escritura en el repo público>
#   CARGO_APK_RELEASE_KEYSTORE_PASSWORD=<contraseña del keystore> (opcional:
#     sin ella se conserva el APK ya publicado)
#   STALE_KEEP="archivo ..." descargas ya marcadas como versión anterior en una
#     publicación previa (se siguen marcando mientras no se recompilen)
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/toolchains.env
REPO=${PUBLIC_REPO:-lelelilo-studios/Finakids}
DEPLOY=1; [ "${1:-}" = "--no-deploy" ] && DEPLOY=0
if [ -n "${SITE_KEY:-}" ]; then
  export GIT_SSH_COMMAND="ssh -i $SITE_KEY -o IdentitiesOnly=yes"
fi

echo "== Web"
./scripts/build_web.sh dist/web >/dev/null

echo "== Descargas actuales del sitio"
rm -rf dist/downloads dist/current
mkdir -p dist/downloads
# todo lo que venga del sitio es "versión anterior" hasta que se recompile aquí
STALE=""
if git clone -q --depth 1 "git@github.com:$REPO.git" dist/current 2>/dev/null && [ -d dist/current/downloads ]; then
  cp dist/current/downloads/* dist/downloads/
  for f in dist/downloads/*; do STALE="$STALE $(basename "$f")"; done
fi
rm -rf dist/current
fresh() { STALE=" $STALE "; STALE="${STALE// $1 / }"; }

echo "== Linux"
cargo build --release --bin finakids
rm -rf dist/pkg && mkdir -p dist/pkg/Finakids
cp target/release/finakids dist/pkg/Finakids/
cp packaging/icons/icon_256.png dist/pkg/Finakids/finakids.png
tar -C dist/pkg -czf dist/downloads/Finakids-linux-x64.tar.gz Finakids
fresh Finakids-linux-x64.tar.gz

echo "== Windows"
if command -v zig >/dev/null 2>&1; then
  ./scripts/build_windows.sh dist/downloads
  fresh Finakids-windows-x64.zip
else
  echo "   (sin Zig: se conserva el zip publicado)"
fi

echo "== Android"
if [ -n "${CARGO_APK_RELEASE_KEYSTORE_PASSWORD:-}" ] || [ -f "$HOME/.local/share/finakids/keystore.pass" ]; then
  ./scripts/build_android.sh dist/downloads
  fresh Finakids-android-arm64.apk
else
  echo "   (sin contraseña del keystore: se conserva el APK publicado)"
fi

echo "== Sitio"
export STALE
echo "   versión anterior:${STALE:- (ninguna)}"
./scripts/assemble_site.sh dist/web dist/downloads dist/site >/dev/null
ls -la dist/site/downloads
if [ "$DEPLOY" = 1 ]; then
  ./scripts/deploy_site.sh dist/site
else
  echo "Sitio armado en dist/site (no publicado)."
fi
