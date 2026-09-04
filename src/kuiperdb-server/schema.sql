-- Reference schema. The executable migration is `SCHEMA` in kuiperdb-core/src/store.rs.
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS records (
    id TEXT PRIMARY KEY,
    payload TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vector_spaces (
    name TEXT PRIMARY KEY,
    dimensions INTEGER NOT NULL CHECK (dimensions > 0),
    distance_metric TEXT NOT NULL,
    normalization TEXT NOT NULL,
    index_config TEXT NOT NULL,
    generation INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS vectors (
    record_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    space TEXT NOT NULL REFERENCES vector_spaces(name) ON DELETE CASCADE,
    values_blob BLOB NOT NULL,
    PRIMARY KEY (record_id, space)
);

CREATE TABLE IF NOT EXISTS record_metadata (
    record_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value_json TEXT NOT NULL,
    PRIMARY KEY (record_id, key)
);

CREATE TABLE IF NOT EXISTS relations (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL REFERENCES records(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vectors_space ON vectors(space);
CREATE INDEX IF NOT EXISTS idx_record_metadata_lookup ON record_metadata(key, value_json, record_id);
CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id);
CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id);

CREATE TRIGGER IF NOT EXISTS vectors_generation_insert AFTER INSERT ON vectors BEGIN
    UPDATE vector_spaces SET generation = generation + 1 WHERE name = NEW.space;
END;
CREATE TRIGGER IF NOT EXISTS vectors_generation_update AFTER UPDATE ON vectors BEGIN
    UPDATE vector_spaces SET generation = generation + 1 WHERE name = NEW.space;
    UPDATE vector_spaces SET generation = generation + 1 WHERE name = OLD.space AND OLD.space <> NEW.space;
END;
CREATE TRIGGER IF NOT EXISTS vectors_generation_delete AFTER DELETE ON vectors BEGIN
    UPDATE vector_spaces SET generation = generation + 1 WHERE name = OLD.space;
END;
