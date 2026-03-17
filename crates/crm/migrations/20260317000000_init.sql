-- Contacts table schema
-- Owned by: moltis-crm crate

CREATE TABLE IF NOT EXISTS contacts (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    source      TEXT,
    external_id TEXT,
    metadata    TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contacts_updated_at
    ON contacts(updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_contacts_source_external_id
    ON contacts(source, external_id)
    WHERE source IS NOT NULL AND external_id IS NOT NULL;
