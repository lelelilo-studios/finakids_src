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

En la web, las mismas opciones funcionan como parámetros de URL (`?hour=19&scene=plaza`).

## Publicación

- `ci.yml`: en cada push a `main` compila la versión web y la publica en el repo público (conserva las descargas).
- `release.yml`: con un tag `v*` o manualmente, compila Windows, macOS (universal), Linux, Android (APK arm64), iOS (IPA sin firmar + simulador) y Web, y publica todo en el repo público (GitHub Pages).
- Requiere el secreto `SITE_DEPLOY_KEY` (clave SSH de despliegue con escritura en `lelelilo-studios/Finakids`).

Fuente tipográfica: Inter (SIL Open Font License, ver `assets/fonts/Inter-LICENSE.txt`).
