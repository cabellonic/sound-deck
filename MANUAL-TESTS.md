# Pruebas manuales reproducibles

Los tests automatizados no pueden cubrir el audio real, el foco de ventanas ni el
comportamiento sobre un juego. Esta es la lista para verificar eso a mano. Sigue el
mismo orden que los criterios de aceptación del MVP.

Antes de empezar, para una prueba desde cero:

```powershell
# Windows — borra la carpeta de datos de la aplicación
Remove-Item -Recurse -Force "$env:APPDATA\app.sounddeck.desktop"
pnpm app:build   # o: pnpm app:dev
```

Necesitás a mano dos archivos de audio propios (MP3 o WAV) y, opcionalmente, una API
key de Freesound.

---

## 1. Primer arranque

| #   | Paso                                    | Resultado esperado                                                          |
| --- | --------------------------------------- | --------------------------------------------------------------------------- |
| 1.1 | Abrir la aplicación                     | Se abre la ventana y aparece la introducción de bienvenida                  |
| 1.2 | Cerrar la introducción con **Empezar**  | No vuelve a aparecer al reiniciar                                           |
| 1.3 | Mirar la botonera                       | Existe una única página llamada "Principal" con 9 botones vacíos            |
| 1.4 | Abrir `%APPDATA%\app.sounddeck.desktop` | Están `database.sqlite`, `sounds/`, `images/`, `temp/`, `logs/`, `backups/` |
| 1.5 | Abrir el log del día                    | Registra versión, migraciones 1 a 3, página inicial, dispositivo y atajos   |

## 2. Importación y biblioteca

| #   | Paso                                                       | Resultado esperado                                                 |
| --- | ---------------------------------------------------------- | ------------------------------------------------------------------ |
| 2.1 | **Importar** → elegir un MP3                               | Toast "1 audio importado"; aparece en **Guardados**                |
| 2.2 | Volver a importar el mismo archivo                         | Toast indica que ya estaba en la biblioteca; no se duplica la fila |
| 2.3 | Arrastrar un audio desde el explorador sobre la biblioteca | Se importa igual que con el diálogo                                |
| 2.4 | Intentar importar un `.txt` renombrado a `.mp3`            | Toast de error con el nombre del archivo; no aparece en la lista   |
| 2.5 | Escribir parte del nombre en el buscador                   | Filtra al instante, sin distinguir mayúsculas ni acentos           |
| 2.6 | Pulsar ▶ en una fila                                       | Suena a volumen de previsualización (20 % por defecto)             |
| 2.7 | Pulsar ▶ en otra fila mientras suena                       | Se corta la anterior y empieza la nueva                            |

## 3. Botonera

| #   | Paso                                                | Resultado esperado                                                 |
| --- | --------------------------------------------------- | ------------------------------------------------------------------ |
| 3.1 | Arrastrar un audio de la biblioteca al botón 1      | El botón muestra nombre y duración; toast de confirmación          |
| 3.2 | Hacer clic en el botón 1                            | Suena a volumen general (35 % por defecto)                         |
| 3.3 | Presionar la tecla `1` con la ventana enfocada      | Suena lo mismo                                                     |
| 3.4 | Escribir `1` dentro del buscador                    | **No** suena nada; el `1` se escribe en el campo                   |
| 3.5 | Arrastrar el botón 1 sobre el botón 5               | Se intercambian los contenidos                                     |
| 3.6 | Clic derecho sobre un botón asignado                | Menú con nombre visible, volumen, metadata, ubicación y quitar     |
| 3.7 | Usar **Asignar a...** desde el menú `⋯` de una fila | Diálogo de página + botón; asigna sin usar el mouse para arrastrar |
| 3.8 | Navegar la botonera con las flechas y pulsar Enter  | El foco se mueve dentro de la grilla y Enter reproduce             |

## 4. Páginas

| #   | Paso                                          | Resultado esperado                                           |
| --- | --------------------------------------------- | ------------------------------------------------------------ |
| 4.1 | Crear una página "Discord"                    | Aparece en la barra con contador `0/9`                       |
| 4.2 | Asignarle un audio distinto                   | El contador pasa a `1/9`                                     |
| 4.3 | Arrastrar la pestaña de una página sobre otra | Cambia el orden                                              |
| 4.4 | Borrar una página con asignaciones            | Pide confirmación indicando cuántos botones tiene            |
| 4.5 | Confirmar el borrado                          | La página desaparece; **los audios siguen** en la biblioteca |
| 4.6 | Intentar borrar la última página que queda    | Se rechaza con un mensaje claro                              |

## 5. Persistencia

| #   | Paso                                                | Resultado esperado                                    |
| --- | --------------------------------------------------- | ----------------------------------------------------- |
| 5.1 | Cerrar la aplicación por completo (bandeja → Salir) | El proceso termina                                    |
| 5.2 | Volver a abrirla                                    | Páginas, asignaciones, nombres y volúmenes siguen ahí |
| 5.3 | Comprobar la página activa                          | Se abre en la última que estabas usando               |

## 6. Audio y dispositivos

| #   | Paso                                                          | Resultado esperado                                             |
| --- | ------------------------------------------------------------- | -------------------------------------------------------------- |
| 6.1 | Ajustes → Audio → **Probar**                                  | Suena un tono corto de 440 Hz                                  |
| 6.2 | Cambiar el dispositivo de salida                              | Toast con el nombre; **Probar** suena por el nuevo dispositivo |
| 6.3 | Mover el volumen general y reproducir un botón                | El volumen cambia de forma audible                             |
| 6.4 | Reiniciar la aplicación                                       | El dispositivo elegido sigue seleccionado                      |
| 6.5 | Desconectar el dispositivo elegido y reiniciar                | Aviso discreto y fallback al predeterminado, sin crash         |
| 6.6 | Cambiar a modo **Superponer** y disparar dos botones seguidos | Suenan a la vez                                                |
| 6.7 | Volver a **Interrumpir** y repetir                            | El segundo corta al primero                                    |

## 6b. Volumen propio de un audio

Lo que hay que oír acá es que un audio deslinkeado deja de moverse con el general.
Conviene hacerlo con un audio que suene fuerte.

| #    | Paso                                                                  | Resultado esperado                                                      |
| ---- | --------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| 6b.1 | Dejar el volumen general en 30 % y disparar un botón                  | Suena a 30 %                                                            |
| 6b.2 | Menú `⋯` del audio → **Ajustar volumen**                              | El interruptor "Seguir el volumen general" está activado y marca 30 %   |
| 6b.3 | Desactivar el interruptor, bajar a 15 % y guardar                     | El audio suena claramente más bajo; la fila muestra el ícono de volumen |
| 6b.4 | Subir el volumen general a 80 % y volver a disparar ese audio         | **Sigue sonando al 15 %**: el general ya no lo toca                     |
| 6b.5 | Disparar otro audio que quedó linkeado                                | Ese sí sube a 80 %                                                      |
| 6b.6 | Previsualizar con ▶ el audio deslinkeado                              | Se escucha a su 15 %, no al volumen de previsualización                 |
| 6b.7 | Volver a activar el interruptor y guardar                             | Vuelve a seguir el general; el ícono de volumen desaparece de la fila   |
| 6b.8 | Clic derecho en un botón → **Volumen del botón** → desactivar y fijar | Ese botón queda fijo aunque el audio suene distinto en otro botón       |

## 6c. Imagen de un audio

| #    | Paso                                                          | Resultado esperado                                                       |
| ---- | ------------------------------------------------------------- | ------------------------------------------------------------------------ |
| 6c.1 | Menú `⋯` de un audio → **Poner imagen** → elegir un PNG o JPG | El botón que lo tenga asignado muestra la imagen, con el nombre encima   |
| 6c.2 | Abrir `%APPDATA%\app.sounddeck.desktop\images`                | Hay un archivo `<uuid>.png`; el original que elegiste sigue donde estaba |
| 6c.3 | Abrir el overlay (`Ctrl + Alt + Espacio`)                     | El botón también muestra la imagen ahí                                   |
| 6c.4 | Arrastrar ese audio a otro botón                              | La imagen viaja con el audio                                             |
| 6c.5 | Asignar a ese botón otro audio sin imagen                     | El botón vuelve a verse sin imagen, sin restos del anterior              |
| 6c.6 | Intentar poner un `.txt` renombrado a `.png`                  | Toast de error; el audio conserva la imagen que tenía                    |
| 6c.7 | **Quitar imagen**                                             | El botón vuelve a verse sin imagen y el archivo desaparece de `images/`  |
| 6c.8 | Poner una imagen, borrarla a mano de `images/` y reiniciar    | El botón se ve sin imagen, sin ícono roto ni error                       |
| 6c.9 | Eliminar de la biblioteca un audio con imagen                 | Su archivo de `images/` también se borra                                 |

## 7. Bandeja e instancia única

| #   | Paso                                                       | Resultado esperado                                  |
| --- | ---------------------------------------------------------- | --------------------------------------------------- |
| 7.1 | Cerrar la ventana con la X (con "cerrar a bandeja" activo) | La ventana se oculta y el icono sigue en la bandeja |
| 7.2 | Clic izquierdo en el icono de la bandeja                   | Vuelve la ventana principal                         |
| 7.3 | Clic derecho en el icono                                   | Menú: abrir, overlay, detener, configuración, salir |
| 7.4 | Ejecutar el `.exe` una segunda vez                         | No abre otra ventana: enfoca la que ya estaba       |
| 7.5 | Bandeja → **Salir completamente**                          | El proceso termina de verdad                        |

## 8. Overlay (la parte que más hay que mirar a ojo)

| #    | Paso                                                            | Resultado esperado                                            |
| ---- | --------------------------------------------------------------- | ------------------------------------------------------------- |
| 8.1  | Con la ventana oculta, presionar `Ctrl + Alt + Espacio`         | Aparece el overlay, centrado y por encima de todo             |
| 8.2  | Presionar `1`                                                   | Suena el botón 1 y el overlay se cierra                       |
| 8.3  | Presionar el atajo de nuevo y luego `Escape`                    | El overlay se cierra sin reproducir                           |
| 8.4  | Con el overlay abierto, hacer clic en otra ventana              | El overlay se cierra al perder el foco                        |
| 8.5  | `Re Pág` / `Av Pág` dentro del overlay                          | Cambia de página; el indicador `n / total` se actualiza       |
| 8.6  | Abrir el Bloc de notas, escribir, abrir el overlay y pulsar `1` | Suena el audio y **no** se escribe un `1` en el Bloc de notas |
| 8.7  | Tras cerrarse el overlay, escribir                              | El foco volvió al Bloc de notas                               |
| 8.8  | Repetir 8.6 con un juego en **modo ventana**                    | Mismo comportamiento                                          |
| 8.9  | Repetir con un juego en **borderless fullscreen**               | Mismo comportamiento                                          |
| 8.10 | Repetir con un juego en **fullscreen exclusivo**                | Limitación conocida: el overlay puede no verse                |
| 8.11 | `Ctrl + Alt + 0` mientras suena algo                            | Todo se detiene                                               |

## 9. Atajos

| #   | Paso                                                | Resultado esperado                                           |
| --- | --------------------------------------------------- | ------------------------------------------------------------ |
| 9.1 | Ajustes → Atajos → cambiar "Abrir/cerrar overlay"   | Captura la combinación y la aplica de inmediato              |
| 9.2 | Asignar a dos acciones el mismo atajo               | Se rechaza indicando que ya está en uso                      |
| 9.3 | Asignar uno que ya use otra aplicación              | Error claro; **se conserva el anterior** y sigue funcionando |
| 9.4 | Asignar una tecla sin modificador a un atajo global | Se rechaza pidiendo un modificador                           |
| 9.5 | **Restaurar atajos predeterminados**                | Vuelven `Ctrl+Alt+Space` y `Ctrl+Alt+0`                      |

## 10. Búsqueda online

### 10.a MyInstants (no necesita API key)

| #      | Paso                                                                      | Resultado esperado                                                        |
| ------ | ------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| 10.a.1 | Pestaña **Internet** sin proveedores activos → **Configurar proveedores** | Se abre Ajustes **ya parado en la pestaña Proveedores**                   |
| 10.a.2 | Activar MyInstants                                                        | Muestra el aviso de "no oficial" y queda listo sin pedir clave            |
| 10.a.3 | Buscar "argentina" en la pestaña Internet                                 | Devuelve resultados con nombre y enlace de origen                         |
| 10.a.4 | Pulsar ▶ en un resultado                                                  | Se escucha sin guardarse en la biblioteca                                 |
| 10.a.5 | Descargar uno                                                             | Aparece en **Guardados**; en "Ver metadata" figura MyInstants y el origen |
| 10.a.6 | Comprobar la licencia en la metadata                                      | Queda vacía a propósito: el sitio no declara ninguna                      |
| 10.a.7 | Hacer varias búsquedas seguidas muy rápido                                | Responde espaciado (~1,2 s entre consultas), sin errores                  |
| 10.a.8 | Buscar algo muy común y mirar el pie de la lista                          | Si aparece **Cargar más resultados**, la página 2 trae audios distintos   |
| 10.a.9 | Desactivar MyInstants y buscar                                            | Deja de consultarse de inmediato                                          |

### 10.b Freesound (requiere API key)

| #     | Paso                                                       | Resultado esperado                                                |
| ----- | ---------------------------------------------------------- | ----------------------------------------------------------------- |
| 10.0  | Leer las instrucciones de la tarjeta de Freesound          | Aclaran que la URL de callback del formulario es solo para OAuth2 |
| 10.1  | Ajustes → Proveedores → activar Freesound y pegar la clave | **Probar conexión** responde "Conexión correcta"                  |
| 10.2  | Reabrir Ajustes                                            | La clave se muestra enmascarada, nunca completa                   |
| 10.3  | Buscar en el log de la aplicación la clave                 | **No aparece en ningún lado**                                     |
| 10.4  | Pestaña **Internet** → escribir una búsqueda               | Resultados tras ~300 ms, con licencia y duración                  |
| 10.5  | Escribir rápido varias palabras distintas                  | Solo se muestran los resultados de la última búsqueda             |
| 10.6  | Pulsar ▶ en un resultado                                   | Se escucha sin quedar guardado en la biblioteca                   |
| 10.7  | Pulsar descargar                                           | Barra de progreso y luego aparece en **Guardados**                |
| 10.8  | Arrastrar un resultado online a un botón vacío             | Muestra progreso en el botón y queda asignado al terminar         |
| 10.9  | Desconectar Internet y pulsar ese botón                    | **Sigue sonando**: el archivo quedó local                         |
| 10.10 | Con Internet desconectado, buscar en **Internet**          | Error claro del proveedor; el resto de la aplicación sigue usable |
| 10.11 | Poner una API key inválida y buscar                        | Mensaje que indica revisar la clave en Ajustes                    |

## 11. Casos borde

| #    | Paso                                                    | Resultado esperado                                                |
| ---- | ------------------------------------------------------- | ----------------------------------------------------------------- |
| 11.1 | Borrar a mano un archivo de `sounds/` y reiniciar       | La fila y el botón se marcan como "archivo faltante"              |
| 11.2 | Pulsar ese botón                                        | Error accionable, sin crash                                       |
| 11.3 | Ajustes → Biblioteca → **Eliminar registros huérfanos** | Se limpian los registros y se liberan los botones                 |
| 11.4 | Eliminar un audio asignado a varios botones             | Lista en qué páginas y botones está antes de confirmar            |
| 11.5 | Mantener presionada la tecla `1`                        | Suena una sola vez (no se repite)                                 |
| 11.6 | Doble clic rápido en un botón                           | Reinicia el sonido, no lo superpone (con los valores por defecto) |
| 11.7 | Cerrar la aplicación durante una descarga               | Al reabrir, `temp/` queda limpio                                  |
| 11.8 | Importar un archivo con nombre en Unicode (漢字, emoji) | Se importa; el nombre se conserva como metadata                   |

---

## Qué queda cubierto por tests automatizados

No hace falta verificar a mano lo siguiente, ya cubierto por `pnpm check:all`
(148 tests de Rust + 37 de frontend):

- Migraciones, creación de páginas y slots, persistencia entre conexiones.
- Deduplicación por hash y rollback ante archivo inválido.
- Validación de URLs (esquemas, hosts, SSRF) y de contenido de audio.
- Descarga contra un servidor HTTP local, incluidos límite de tamaño y errores.
- Normalización de atajos y detección de conflictos.
- Cálculo de volumen efectivo y mapeo de categorías.
- Parseo del proveedor Freesound contra fixtures.
- Render de la botonera, navegación por teclado y captura de `1`–`9`.
- Que escribir en un input no dispare los atajos numéricos.
