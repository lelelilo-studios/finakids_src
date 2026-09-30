# Finakids — código fuente

Videojuego 3D de simulación de vida y educación financiera para jóvenes (10–17 años), escrito en **Rust + wgpu**.
El jugador vive ocho semanas en la vida de Sofía: recibe dinero, ahorra, compra, usa crédito, trabaja, emprende e invierte, y vive las consecuencias de cada decisión.

- Juego publicado (web + descargas): https://github.com/lelelilo-studios/Finakids
- Plataformas: Web (WebGPU / WebGL2), Windows, macOS, Linux, Android, iOS.

## Arquitectura

| Módulo | Contenido |
|---|---|
| `src/gfx` | Renderer forward HDR sobre wgpu: PBR con patrones procedurales (madera, telas, piel, ojos, cabello…), sombras PCF, cielo dinámico, MSAA, bloom, tonemapping ACES, viñeta, grano, letterbox, vidrio esmerilado para la UI. |
| `src/sdf.rs` | Modelado por campos de distancia con *surface nets* disperso. |
| `src/character` | Personajes procedurales: cuerpo, ropa, cabeza, manos y peinados generados por SDF; esqueleto de 29 huesos con *skinning* en GPU; animador procedural (pasos con pies plantados, IK de dos huesos, respiración, cambio de peso, mirada con sacadas, parpadeo, expresiones faciales, boca animada, física secundaria de coleta/cabello/ropa) y biblioteca de acciones (anticipación → movimiento → contacto → acción → recuperación). |
| `src/world` | Habitación, casa y plaza del barrio con mobiliario procedural, luces y ventanas con haces de luz. |
| `src/ui` | UI inmediata propia: SDF redondeados, texto (fontdue + Inter), íconos vectoriales, vidrio esmerilado, sliders. |
| `src/game` | Estado, director de escenas (`script.rs`), guion (`story.rs`), finanzas (`finance.rs`), tienda, negocio, minijuego del café, HUD/teléfono (`hud.rs`), guardado. |

## Compilar y ejecutar

```bash
cargo run --release                 # escritorio
./scripts/build_web.sh dist/web     # web (requiere wasm-bindgen-cli 0.2.129)
python3 -m http.server -d dist/web  # probar en http://localhost:8000
cargo apk build --release --lib     # Android (cargo-apk + NDK)
./scripts/package_macos.sh          # macOS .app universal (en macOS)
./scripts/package_ios.sh            # iOS .ipa sin firmar + simulador (en macOS)
```

### Opciones de depuración

```bash
finakids --shot out.png --size 1280x720 --frames 120 [--scene bedroom|home|plaza] [--hour 19]
         [--beat choice|phone|shop|summary|biz|cafe|reflect|goals|week3|plaza] [--cam face|body|lab_head|lab_body]
         [--autoplay] [--fresh] [--no-ui] [--icon]
```

Prueba automática de la historia completa (juega las 8 semanas solo y registra cada decisión y el estado financiero):

```bash
finakids --shot fin.png --frames 100000 --bot-policy first|last|wise|random:SEMILLA --save-dir /tmp/finakids-bot
```

En la web, las mismas opciones funcionan como parámetros de URL (`?hour=19&scene=plaza`).

## Publicación

Las GitHub Actions de los repos privados de la cuenta no arrancan (límite de facturación), así que la compilación vive en el repo público:

- `lelelilo-studios/Finakids` tiene `.github/workflows/build.yml` (copia canónica en `packaging/public-ci/build.yml`). Clona este código con una clave de despliegue de solo lectura (`SRC_DEPLOY_KEY`), compila Web, Windows, macOS (universal), Linux, Android (APK arm64) e iOS (IPA sin firmar + simulador) y publica solo los binarios en GitHub Pages.
- Para publicar una versión nueva: `gh workflow run build.yml -R lelelilo-studios/Finakids -f ref=main`.
- Despliegue local (web + Linux, conservando las demás descargas del sitio): `./scripts/build_web.sh`, empaquetar el tarball Linux en `dist/downloads` junto a las descargas actuales, `./scripts/assemble_site.sh dist/web dist/downloads dist/site` y `GIT_SSH_COMMAND="ssh -i <clave>" ./scripts/deploy_site.sh dist/site`.
- El APK se firma con un keystore estable guardado en los secretos `ANDROID_KEYSTORE_B64` / `ANDROID_KEYSTORE_PASSWORD`.
- `ci.yml` y `release.yml` de este repo quedan solo con ejecución manual, por si se reactivan las Actions privadas.

Fuente tipográfica: Inter (SIL Open Font License, ver `assets/fonts/Inter-LICENSE.txt`).
