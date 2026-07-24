-- El volumen propio de un audio pasa de multiplicador a valor absoluto opcional.
--
-- Antes: `custom_volume` era NOT NULL con default 1.0 y se multiplicaba por el
-- volumen general (`master * custom_volume`). Un audio solo podia atenuarse, y
-- para dejarlo sonando al 15% con el general en 30% habia que escribir 50%.
--
-- Ahora: NULL significa "linkeado al volumen general" y cualquier otro valor es
-- el volumen absoluto de ese audio, sin importar donde este el general. Es la
-- misma convencion que ya usaba `slots.custom_volume`, asi que los tres niveles
-- se resuelven con una sola regla: slot ?? sound ?? master.
--
-- Conversion: los audios en 1.0 (el default, la enorme mayoria) quedan
-- linkeados. Los que tenian un valor propio `v` pasan a `master * v`, que es
-- exactamente lo que se escuchaba antes de migrar: nadie cambia de volumen al
-- actualizar.
--
-- SQLite no permite quitar un NOT NULL con ALTER TABLE, asi que reconstruimos
-- la tabla. El runner corre esta migracion con `PRAGMA foreign_keys = OFF`
-- (ver `rebuilds_tables` en migrations.rs): sin eso, el DROP TABLE dispararia
-- el `ON DELETE SET NULL` de `slots.sound_id` y vaciaria toda la botonera.

CREATE TABLE sounds_new (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL,
    original_name          TEXT,
    internal_filename      TEXT NOT NULL UNIQUE,
    file_path              TEXT NOT NULL,
    content_hash           TEXT NOT NULL UNIQUE,
    mime_type              TEXT,
    file_extension         TEXT,
    file_size_bytes        INTEGER,
    duration_ms            INTEGER,
    source_type            TEXT NOT NULL CHECK (source_type IN ('local_import', 'provider')),
    provider_id            TEXT,
    remote_id              TEXT,
    source_page_url        TEXT,
    download_url_reference TEXT,
    provider_category      TEXT,
    normalized_category    TEXT NOT NULL DEFAULT 'uncategorized',
    license_code           TEXT,
    license_name           TEXT,
    license_url            TEXT,
    attribution            TEXT,
    -- NULL = sigue el volumen general. Un valor = volumen absoluto propio.
    custom_volume          REAL CHECK (custom_volume IS NULL OR (custom_volume >= 0.0 AND custom_volume <= 1.0)),
    play_count             INTEGER NOT NULL DEFAULT 0,
    last_played_at         TEXT,
    search_index           TEXT NOT NULL DEFAULT '',
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL
);

INSERT INTO sounds_new (
    id, name, original_name, internal_filename, file_path, content_hash,
    mime_type, file_extension, file_size_bytes, duration_ms, source_type,
    provider_id, remote_id, source_page_url, download_url_reference,
    provider_category, normalized_category, license_code, license_name,
    license_url, attribution, custom_volume, play_count, last_played_at,
    search_index, created_at, updated_at
)
SELECT
    id, name, original_name, internal_filename, file_path, content_hash,
    mime_type, file_extension, file_size_bytes, duration_ms, source_type,
    provider_id, remote_id, source_page_url, download_url_reference,
    provider_category, normalized_category, license_code, license_name,
    license_url, attribution,
    CASE
        WHEN custom_volume IS NULL OR custom_volume = 1.0 THEN NULL
        ELSE MAX(0.0, MIN(1.0, custom_volume * COALESCE(
            (SELECT json_extract(value_json, '$.masterVolume')
               FROM settings WHERE section = 'audio'),
            0.35
        )))
    END,
    play_count, last_played_at, search_index, created_at, updated_at
FROM sounds;

DROP TABLE sounds;

ALTER TABLE sounds_new RENAME TO sounds;

CREATE INDEX idx_sounds_name ON sounds (name COLLATE NOCASE);
CREATE INDEX idx_sounds_search ON sounds (search_index);
CREATE INDEX idx_sounds_category ON sounds (normalized_category);
CREATE INDEX idx_sounds_provider ON sounds (provider_id);
CREATE INDEX idx_sounds_created_at ON sounds (created_at DESC);
CREATE INDEX idx_sounds_play_count ON sounds (play_count DESC);
CREATE INDEX idx_sounds_last_played ON sounds (last_played_at DESC);
CREATE UNIQUE INDEX idx_sounds_remote ON sounds (provider_id, remote_id)
    WHERE provider_id IS NOT NULL AND remote_id IS NOT NULL;
