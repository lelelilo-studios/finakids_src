#!/usr/bin/env bash
# Assembles the public site: web build at the root plus downloadable builds.
# usage: assemble_site.sh <web_dir> <downloads_dir|-> <out_dir>
set -euo pipefail
WEB=$1; DL=$2; OUT=$3
rm -rf "$OUT" && mkdir -p "$OUT"
cp -r "$WEB"/. "$OUT"/
touch "$OUT/.nojekyll"
# keep the build pipeline that lives in the public repo
mkdir -p "$OUT/.github/workflows"
cp "$(dirname "$0")/../packaging/public-ci/build.yml" "$OUT/.github/workflows/build.yml"
if [ "$DL" != "-" ] && [ -d "$DL" ]; then
  mkdir -p "$OUT/downloads"
  cp "$DL"/* "$OUT/downloads/" 2>/dev/null || true
fi
VERSION=$(grep -m1 '^version' "$(dirname "$0")/../Cargo.toml" | cut -d'"' -f2)
BASE="https://lelelilo-studios.github.io/Finakids"
# STALE: descargas que no se pudieron recompilar en esta publicación (se conservan de la anterior)
STALE=" ${STALE:-} "
link() {
  if [ -f "$OUT/downloads/$1" ]; then
    case "$STALE" in *" $1 "*) echo "[$2]($BASE/downloads/$1) _(versión anterior)_" ;; *) echo "[$2]($BASE/downloads/$1)" ;; esac
  else echo "_(próximamente)_"; fi
}
cat > "$OUT/README.md" <<MD
# Finakids

**Simulación de vida en 3D para aprender a manejar el dinero tomando decisiones.**
Vive ocho semanas en la vida de Sofía: recibe dinero, ahorra, compra, usa crédito, trabaja, emprende e invierte... y vive las consecuencias.

## ▶ Jugar ahora en el navegador

**$BASE/**

Funciona en Chrome, Edge, Safari y Firefox recientes (WebGPU, con respaldo WebGL2), también en celulares y tablets: gira el teléfono en horizontal. Con sonido (música y efectos; se ajustan en el menú de pausa).

## Descargas (versión $VERSION)

| Plataforma | Descarga | Notas |
|---|---|---|
| Windows 10/11 (64 bits) | $(link Finakids-windows-x64.zip "Finakids-windows-x64.zip") | Descomprime y abre \`finakids.exe\`. Si SmartScreen avisa: *Más información → Ejecutar de todas formas*. |
| macOS 11+ (Apple Silicon e Intel) | $(link Finakids-macos-universal.zip "Finakids-macos-universal.zip") | App sin firma de Apple: clic derecho → *Abrir*, o \`xattr -dr com.apple.quarantine Finakids.app\`. |
| Linux x86_64 | $(link Finakids-linux-x64.tar.gz "Finakids-linux-x64.tar.gz") | Requiere drivers Vulkan u OpenGL. \`./finakids\` |
| Android 8+ (arm64) | $(link Finakids-android-arm64.apk "Finakids-android-arm64.apk") | Permite *instalar apps desconocidas* para tu navegador y abre el APK. |
| iOS / iPadOS 15+ | $(link Finakids-ios-unsigned.ipa "Finakids-ios-unsigned.ipa") | IPA sin firmar: instálala firmándola con tu Apple ID (AltStore / Sideloadly). También: $(link Finakids-ios-simulator.zip "build para Simulador"). |

## Controles

| | Teclado y ratón | Pantalla táctil |
|---|---|---|
| **Mover** | WASD / flechas, o clic en el suelo | Toca el suelo |
| **Interactuar / hablar** | E, o clic en el objeto o personaje | Toca el objeto o personaje |
| **Teléfono** (banco, metas, crédito, inversiones...) | TAB o el botón inferior derecho | Botón inferior derecho |
| **Cámara** | Arrastrar, Q / R, rueda | Arrastra; pellizca para acercar |
| **Pausa** (volumen, calidad gráfica) | Esc o el botón ⏸ | Botón ⏸ |
| **Pantalla completa** | F | Automática al primer toque (Android) |

---
Hecho con Rust + wgpu. Todo el mundo 3D, los personajes y sus animaciones se generan proceduralmente en tiempo real.
MD
ls -la "$OUT" "$OUT/downloads" 2>/dev/null || true
