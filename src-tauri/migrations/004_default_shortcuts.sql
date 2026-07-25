-- Los atajos globales pasaron a ser Alt+Inicio y Alt+Fin.
--
-- Cambiar los valores predeterminados solo alcanza para las instalaciones
-- nuevas: las que ya existen tienen la seccion guardada y seguirian con
-- Ctrl+Alt+Espacio y Ctrl+Alt+0 para siempre.
--
-- El reemplazo incluye el `action` para que solo aplique al atajo correcto, y
-- exige que el acelerador siga siendo exactamente el viejo predeterminado: si
-- el usuario eligio el suyo, el texto no coincide y no se toca nada. Si algun
-- dia cambia el orden de los campos al serializar, tampoco coincide y esta
-- migracion no hace nada, que es la forma correcta de fallar.
UPDATE settings
   SET value_json = replace(
           value_json,
           '{"action":"toggle_overlay","accelerator":"Ctrl+Alt+Space"',
           '{"action":"toggle_overlay","accelerator":"Alt+Home"'
       )
 WHERE section = 'shortcuts';

UPDATE settings
   SET value_json = replace(
           value_json,
           '{"action":"stop_all","accelerator":"Ctrl+Alt+0"',
           '{"action":"stop_all","accelerator":"Alt+End"'
       )
 WHERE section = 'shortcuts';
