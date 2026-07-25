# Dependencias de terceros

Sound Deck se distribuye bajo licencia MIT. Este archivo lista las dependencias
directas y sus licencias. Para el árbol completo:

```bash
cargo install cargo-license && cargo license --manifest-path src-tauri/Cargo.toml
pnpm licenses list
```

## Rust (`src-tauri/Cargo.toml`)

| Crate                                               | Licencia                     | Para qué se usa                                         |
| --------------------------------------------------- | ---------------------------- | ------------------------------------------------------- |
| `tauri`, `tauri-build`                              | Apache-2.0 OR MIT            | Framework de la aplicación de escritorio                |
| `tauri-plugin-dialog`                               | Apache-2.0 OR MIT            | Diálogo nativo de selección de archivos                 |
| `tauri-plugin-opener`                               | Apache-2.0 OR MIT            | Abrir carpetas y URLs en el sistema                     |
| `tauri-plugin-global-shortcut`                      | Apache-2.0 OR MIT            | Atajos globales                                         |
| `tauri-plugin-single-instance`                      | Apache-2.0 OR MIT            | Una sola instancia de la aplicación                     |
| `tauri-plugin-autostart`                            | Apache-2.0 OR MIT            | Iniciar con el sistema                                  |
| `rusqlite` (feature `bundled`)                      | MIT                          | SQLite embebido. SQLite es de dominio público           |
| `rodio`                                             | MIT OR Apache-2.0            | Decodificación y mezcla de audio                        |
| `cpal` (vía `rodio::cpal`)                          | Apache-2.0                   | Enumeración y apertura de dispositivos                  |
| `ebur128`                                           | MIT                          | Medición de sonoridad EBU R128 para normalizar volumen  |
| `symphonia` (vía `rodio`)                           | MPL-2.0                      | Decodificadores MP3, FLAC, Vorbis, WAV                  |
| `reqwest`                                           | MIT OR Apache-2.0            | Cliente HTTP                                            |
| `tokio`, `futures-util`                             | MIT                          | Runtime asíncrono y streams                             |
| `serde`, `serde_json`                               | MIT OR Apache-2.0            | Serialización                                           |
| `uuid`                                              | MIT OR Apache-2.0            | Identificadores internos de archivos y entidades        |
| `thiserror`                                         | MIT OR Apache-2.0            | Errores de dominio                                      |
| `tracing`, `tracing-subscriber`, `tracing-appender` | MIT                          | Logging con rotación diaria                             |
| `sha2`                                              | MIT OR Apache-2.0            | Hash de contenido para deduplicación                    |
| `time`                                              | MIT OR Apache-2.0            | Marcas de tiempo RFC 3339                               |
| `parking_lot`                                       | MIT OR Apache-2.0            | Mutex de la base y del motor de audio                   |
| `dunce`                                             | CC0-1.0 OR MIT OR Apache-2.0 | Rutas canónicas legibles en Windows                     |
| `async-trait`                                       | MIT OR Apache-2.0            | Trait `SoundProvider` asíncrono                         |
| `scraper`                                           | ISC                          | Parseo HTML del proveedor no oficial MyInstants         |
| `windows`                                           | MIT OR Apache-2.0            | Restaurar el foco de la ventana anterior (solo Windows) |
| `tempfile`, `tiny_http`                             | MIT OR Apache-2.0            | Solo en tests                                           |

`symphonia` está bajo MPL-2.0: es una licencia de copyleft débil por archivo. Se usa
como biblioteca sin modificar, lo que es compatible con distribuir Sound Deck bajo MIT.
Si llegás a modificar su código, esos archivos deben publicarse bajo MPL-2.0.

## JavaScript (`package.json`)

| Paquete                                                                     | Licencia          | Para qué se usa                       |
| --------------------------------------------------------------------------- | ----------------- | ------------------------------------- |
| `react`, `react-dom`                                                        | MIT               | Interfaz                              |
| `@tauri-apps/api` y plugins                                                 | MIT OR Apache-2.0 | Puente con el backend                 |
| `@tanstack/react-query`                                                     | MIT               | Estado asíncrono y caché de búsquedas |
| `@tanstack/react-virtual`                                                   | MIT               | Lista virtualizada de la biblioteca   |
| `zustand`                                                                   | MIT               | Estado de interfaz                    |
| `@radix-ui/*`                                                               | MIT               | Primitivas accesibles                 |
| `lucide-react`                                                              | ISC               | Iconos                                |
| `tailwindcss`, `@tailwindcss/vite`                                          | MIT               | Estilos                               |
| `clsx`, `tailwind-merge`                                                    | MIT               | Composición de clases                 |
| `vite`, `@vitejs/plugin-react`                                              | MIT               | Build y desarrollo                    |
| `typescript`, `eslint`, `prettier`, `vitest`, `@testing-library/*`, `jsdom` | MIT               | Herramientas de desarrollo            |

## Contenido

El repositorio **no incluye archivos de audio de terceros**. El tono de prueba del
dispositivo se genera en memoria (onda senoidal de 440 Hz). El icono de la aplicación
se genera con `scripts/generate-icon.mjs`, sin dependencias externas.

Los audios descargados desde un proveedor conservan su licencia original, que Sound
Deck guarda junto con la atribución en la metadata de cada sonido.
