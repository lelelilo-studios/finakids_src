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
| `src/character` | Personajes procedurales: cuerpo, ropa, cabeza, manos y peinados generados por SDF en segundo plano (`factory.rs`: hilos en nativo, Web Workers en la web) con tres niveles de detalle (alto, bajo y móvil); esqueleto de 39 huesos (dedos incluidos) con *skinning* en GPU; animador procedural (pasos con pies plantados, IK de dos huesos, respiración, cambio de peso, mirada con sacadas, parpadeo, expresiones faciales, boca animada, física secundaria de coleta/cabello/ropa) y biblioteca de acciones (anticipación → movimiento → contacto → acción → recuperación). |
| `src/world` | Habitación, casa y plaza del barrio con mobiliario procedural, luces y ventanas con haces de luz. |
| `src/ui` | UI inmediata propia: SDF redondeados, texto (fontdue + Inter), íconos vectoriales, vidrio esmerilado, sliders. |
| `src/audio` | Sonido procedural: música generativa por lugar y hora, ambiente y efectos (cpal en nativo, WebAudio en la web). |
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

Para previsualizar un celular en el escritorio: `--size 1477x683 --density 1.75 --quality mobile` (la densidad activa los diseños compactos; `--quality high|low|mobile` fija el detalle de los personajes). `--perf` registra el costo de CPU por frame y las llamadas de dibujo; `--beat pause` muestra el menú de pausa.

En la web, las mismas opciones funcionan como parámetros de URL (`?hour=19&scene=plaza`); además `?quality=low|high` fuerza o desactiva el modo móvil y `?perf=1` escribe el rendimiento en la consola.

### Celulares (web y nativo)

- El lienzo web sigue el tamaño de la página y se limita a 1,75 píxeles por píxel CSS (~1,25 MP); la escena 3D usa resolución dinámica según el tiempo de frame (`app.rs`, `Perf`). En el menú de pausa se puede fijar *Rendimiento* o *Calidad*.
- Sin MSAA, sombras de 1024 con 4 muestras y 4 luces puntuales por píxel; personajes con detalle móvil (~28 mil vértices).
- La interfaz nunca baja de 0,8 píxeles CSS por unidad: en pantallas bajas (`Ui::compact`) los paneles usan diseños compactos con scroll.

### Sonido

Todo el audio es procedural (`src/audio/`): música generativa por momento del día y lugar, ambiente (pájaros, grillos, plaza, fuente) y efectos sintetizados. No hay archivos de sonido. En nativo sale por cpal (en Linux hace falta `libasound2-dev` para compilar); en la web, por WebAudio (empieza con el primer toque o clic). El volumen se ajusta en la pausa.

```bash
finakids --no-audio                 # sin sonido (los modos --shot y --bot ya lo desactivan)
finakids --audio-dump DIR           # escribe cada música, ambiente y efecto en WAV e imprime niveles y costo
finakids --audio-test 10            # reproduce una escena de prueba 10 s e informa cortes (underruns)
finakids --shot x.png --bot-policy wise --audio-trace   # registra cada efecto y cambio de música
```

## Publicación

Las GitHub Actions de los repos privados de la cuenta no arrancan (límite de facturación), así que la compilación vive en el repo público:

- `lelelilo-studios/Finakids` tiene `.github/workflows/build.yml` (copia canónica en `packaging/public-ci/build.yml`). Clona este código con una clave de despliegue de solo lectura (`SRC_DEPLOY_KEY`), compila Web, Windows, macOS (universal), Linux, Android (APK arm64) e iOS (IPA sin firmar + simulador) y publica solo los binarios en GitHub Pages.
- Para publicar una versión nueva: `gh workflow run build.yml -R lelelilo-studios/Finakids -f ref=main`.
- **Despliegue local (el que se usa):** `SITE_KEY=<clave de despliegue> ./scripts/publish_local.sh` compila Web, Linux, Windows y (si está la contraseña del keystore) Android, conserva las descargas que no se pueden generar aquí marcándolas como «versión anterior», arma el sitio y lo publica. Con `--no-deploy` solo lo arma en `dist/site`.
- Windows y Android también se compilan en local desde Linux, sin sudo (dejan el archivo en `dist/downloads`):
  - `./scripts/build_windows.sh` → `Finakids-windows-x64.zip` (Rust `x86_64-pc-windows-gnu` + Zig como enlazador vía `cargo-zigbuild`; el icono lo incrusta `build.rs`).
  - `CARGO_APK_RELEASE_KEYSTORE_PASSWORD='…' ./scripts/build_android.sh` → `Finakids-android-arm64.apk` (`cargo-apk`; keystore por defecto en `~/.local/share/finakids/finakids-release.p12`, o `CARGO_APK_RELEASE_KEYSTORE`). Sin la contraseña el script se detiene: el APK debe firmarse con la misma clave publicada para que se actualice encima.
  - Herramientas esperadas en `~/.local/opt` (las localiza `scripts/toolchains.env`): Zig (`zig-*`), Temurin JDK 17 (`jdk-17*`) y el SDK de Android (`android-sdk` con `platforms;android-34`, `build-tools;34.0.0` y un NDK r27), más `cargo install cargo-zigbuild cargo-apk` y `rustup target add x86_64-pc-windows-gnu aarch64-linux-android`.
  - macOS e iOS siguen necesitando una Mac (`scripts/package_macos.sh`, `scripts/package_ios.sh`).
- El APK se firma con un keystore estable guardado en los secretos `ANDROID_KEYSTORE_B64` / `ANDROID_KEYSTORE_PASSWORD`.
- `ci.yml` y `release.yml` de este repo quedan solo con ejecución manual, por si se reactivan las Actions privadas.

Fuente tipográfica: Inter (SIL Open Font License, ver `assets/fonts/Inter-LICENSE.txt`).
