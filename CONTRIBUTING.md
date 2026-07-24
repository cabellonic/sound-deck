# Contribuir a Sound Deck

## Antes de empezar

```bash
pnpm install
pnpm app:dev
```

## Antes de abrir un PR

Ejecutá la verificación completa. Debe pasar entera:

```bash
pnpm check:all
```

Equivale a: Prettier → ESLint → `tsc --noEmit` → Vitest → `cargo fmt --check` →
`cargo clippy -D warnings` → `cargo test`.

Es lo mismo que corre CI en cada push y cada PR, con Rust compilado en Windows y
en Linux. Correrlo antes ahorra el viaje de ida y vuelta.

## Publicar una versión

Las releases las arma GitHub Actions; no se suben binarios a mano.

1. Actualizá la versión en `package.json` **y** en `src-tauri/tauri.conf.json`.
   Tienen que coincidir: el workflow verifica que la etiqueta también coincida y
   falla antes de compilar si no.
2. Commiteá el cambio de versión y mergealo a `main`.
3. Etiquetá y empujá:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

4. El workflow compila en Windows y Linux y deja una **release en borrador** con
   el `.exe`, el `.deb` y el `.AppImage`.
5. Revisá que estén los artefactos de los dos sistemas y publicala desde la
   interfaz de GitHub.

Se publica en borrador a propósito: si un runner falla a mitad de camino, nadie
se descarga una release incompleta mientras tanto.

## Reglas del proyecto

**Rust**

- Nada de `unwrap()` en código de producción. Usá `AppError` con un mensaje accionable
  en español y `with_technical(...)` para el detalle que va a los logs.
- Los mensajes al usuario nunca incluyen rutas crudas, stack traces ni API keys.
- El hilo principal de Tauri no se bloquea: descarga, decodificación, hashing y copia
  de archivos van a `spawn_blocking` o a un comando `async`.
- Todo lo que dependa del sistema operativo se encapsula en `src/platform`. No hay
  llamadas Win32 fuera de ese módulo.
- Un archivo entra a la biblioteca **solo** por `library::ingest`. Si agregás una vía
  nueva de importación, reusala en vez de duplicar la validación.

**TypeScript**

- Nada de `any`. El único borde donde se acepta `unknown` es `normalizeError` en
  `lib/ipc.ts`, y está documentado.
- Los comandos IPC se agregan en `lib/ipc.ts`, nunca con `invoke` suelto en un
  componente.
- Los datos persistentes se consultan con TanStack Query. Zustand es solo para estado
  de interfaz: toasts, pestañas, texto de búsqueda, indicadores.
- Botones reales (`<button>`), no `div` clickeables. Todo control necesita nombre
  accesible.
- Las máquinas de estado con más de dos valores se modelan como unión discriminada, no
  como booleanos sueltos (ver `DialogState` en `windows/main/App.tsx`).

## Agregar un proveedor online

1. Implementá `SoundProvider` en `src-tauri/src/providers/tu_proveedor.rs`.
2. Declará sus hosts en `allowed_hosts()`: toda URL se valida contra esa lista.
3. Agregá un fixture JSON o HTML en `src-tauri/tests/fixtures/` y probá el parseo sin
   tocar la red.
4. Registralo en `ProviderRegistry::new`.
5. Si no tiene API pública documentada, marcá `unofficial: true` y dejalo desactivado
   por defecto. No evadas robots.txt, CAPTCHA, autenticación ni rate limits, y no
   descargues catálogos completos.

## Agregar una migración

1. Creá `src-tauri/migrations/00N_descripcion.sql`.
2. Sumá la entrada en `MIGRATIONS` (`src-tauri/src/database/migrations.rs`).
3. Nunca edites una migración ya publicada: agregá una nueva.
4. Verificá que `aplica_migraciones_y_es_idempotente` siga pasando.

Si la migración reconstruye una tabla que otras referencian con claves foráneas,
marcala con `rebuilds_tables: true`. Sin eso, el `DROP TABLE` dispara los
`ON DELETE` de las tablas hijas: reconstruir `sounds` sin esa marca le vacía la
botonera a todos los usuarios.

## Tests

- Rust: unitarios junto al código, integración en `src-tauri/tests/`.
- Frontend: `src/tests/`, con Vitest y Testing Library.
- Ningún test puede depender de Internet. Para HTTP usá el servidor `tiny_http` local
  como en `src-tauri/tests/integration.rs`.
- Un test que falla porque la implementación es demasiado permisiva se arregla
  endureciendo la implementación, no relajando el test.
