-- Sonoridad medida al importar, para poder igualar el volumen entre audios.
--
-- `loudness_lufs` es la sonoridad integrada de EBU R128 y `peak_amplitude` el
-- pico maximo de muestra, que es lo que limita cuanto se puede amplificar sin
-- saturar. NULL en los dos significa "todavia sin medir": los audios que ya
-- estaban en la biblioteca arrancan asi y se miden cuando el usuario lo pida.
ALTER TABLE sounds ADD COLUMN loudness_lufs REAL;
ALTER TABLE sounds ADD COLUMN peak_amplitude REAL;
