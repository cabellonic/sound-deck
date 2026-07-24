-- Esquema inicial de Sound Deck.
-- Los binarios de audio viven en AppData/sounds; aqui solo guardamos metadata.

CREATE TABLE sounds (
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
    custom_volume          REAL NOT NULL DEFAULT 1.0 CHECK (custom_volume >= 0.0 AND custom_volume <= 1.0),
    play_count             INTEGER NOT NULL DEFAULT 0,
    last_played_at         TEXT,
    -- Texto normalizado (minusculas, sin acentos) para la busqueda local.
    -- Se recalcula en cada escritura que toque nombre, tags o categorias.
    search_index           TEXT NOT NULL DEFAULT '',
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL
);

CREATE INDEX idx_sounds_name ON sounds (name COLLATE NOCASE);
CREATE INDEX idx_sounds_search ON sounds (search_index);
CREATE INDEX idx_sounds_category ON sounds (normalized_category);
CREATE INDEX idx_sounds_provider ON sounds (provider_id);
CREATE INDEX idx_sounds_created_at ON sounds (created_at DESC);
CREATE INDEX idx_sounds_play_count ON sounds (play_count DESC);
CREATE INDEX idx_sounds_last_played ON sounds (last_played_at DESC);
CREATE UNIQUE INDEX idx_sounds_remote ON sounds (provider_id, remote_id)
    WHERE provider_id IS NOT NULL AND remote_id IS NOT NULL;

CREATE TABLE sound_tags (
    sound_id TEXT NOT NULL,
    tag      TEXT NOT NULL,
    PRIMARY KEY (sound_id, tag),
    FOREIGN KEY (sound_id) REFERENCES sounds (id) ON DELETE CASCADE
);

CREATE INDEX idx_sound_tags_tag ON sound_tags (tag);

CREATE TABLE pages (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Indice unico diferido: reordenar paginas hace varios UPDATE dentro de una
-- transaccion y las posiciones intermedias pueden colisionar.
CREATE UNIQUE INDEX idx_pages_position ON pages (position);

CREATE TABLE slots (
    page_id       TEXT NOT NULL,
    slot_number   INTEGER NOT NULL CHECK (slot_number BETWEEN 1 AND 9),
    sound_id      TEXT,
    custom_label  TEXT,
    custom_volume REAL CHECK (custom_volume IS NULL OR (custom_volume >= 0.0 AND custom_volume <= 1.0)),
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    PRIMARY KEY (page_id, slot_number),
    FOREIGN KEY (page_id) REFERENCES pages (id) ON DELETE CASCADE,
    FOREIGN KEY (sound_id) REFERENCES sounds (id) ON DELETE SET NULL
);

CREATE INDEX idx_slots_sound ON slots (sound_id);

-- Configuracion: una fila por seccion, con JSON validado contra los structs de Rust.
CREATE TABLE settings (
    section    TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE provider_settings (
    provider_id   TEXT PRIMARY KEY,
    enabled       INTEGER NOT NULL DEFAULT 0,
    settings_json TEXT NOT NULL DEFAULT '{}',
    updated_at    TEXT NOT NULL
);
