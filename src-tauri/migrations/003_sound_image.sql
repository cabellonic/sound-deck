-- Imagen opcional de un audio, para que la botonera la muestre en vez de una
-- caja de texto vacia.
--
-- La imagen es del audio, no del boton: si el audio se asigna a otro slot la
-- imagen viaja con el, y un boton con un audio sin imagen se ve sin imagen.
--
-- Guardamos la ruta absoluta del archivo ya copiado a la carpeta administrada
-- (`AppData/images/<uuid>.<ext>`), igual que `file_path` para el audio. NULL
-- significa que el audio no tiene imagen, que es el estado por defecto.
ALTER TABLE sounds ADD COLUMN image_path TEXT;
