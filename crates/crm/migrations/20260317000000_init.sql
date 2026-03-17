-- CRM tables
-- Owned by: moltis-crm crate

-- ── crm_contacts ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS crm_contacts (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    source      TEXT,
    external_id TEXT,
    metadata    TEXT,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_crm_contacts_updated_at
    ON crm_contacts(updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_crm_contacts_source_external_id
    ON crm_contacts(source, external_id)
    WHERE source IS NOT NULL AND external_id IS NOT NULL;

-- ── crm_matters ─────────────────────────────────────────────────────────────
-- Matters represent cases, deals, or topics being tracked for a contact.
-- contact_id is nullable: a matter may not be tied to a single contact.
CREATE TABLE IF NOT EXISTS crm_matters (
    id         TEXT    PRIMARY KEY,
    title      TEXT    NOT NULL,
    status     TEXT    NOT NULL DEFAULT 'open',
    contact_id TEXT    REFERENCES crm_contacts(id) ON DELETE SET NULL,
    metadata   TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_crm_matters_updated_at
    ON crm_matters(updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_crm_matters_contact_id
    ON crm_matters(contact_id)
    WHERE contact_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_matters_status
    ON crm_matters(status);

-- ── crm_interactions ────────────────────────────────────────────────────────
-- Logged interactions (messages, calls, meetings, notes) with a contact.
-- contact_id is required; matter_id is optional.
CREATE TABLE IF NOT EXISTS crm_interactions (
    id         TEXT    PRIMARY KEY,
    contact_id TEXT    NOT NULL REFERENCES crm_contacts(id) ON DELETE CASCADE,
    matter_id  TEXT    REFERENCES crm_matters(id) ON DELETE SET NULL,
    kind       TEXT    NOT NULL,
    body       TEXT,
    metadata   TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_crm_interactions_contact_id
    ON crm_interactions(contact_id);

CREATE INDEX IF NOT EXISTS idx_crm_interactions_matter_id
    ON crm_interactions(matter_id)
    WHERE matter_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_interactions_created_at
    ON crm_interactions(created_at DESC);

-- ── crm_contact_channels ────────────────────────────────────────────────────
-- Maps contacts to communication channels (Telegram, WhatsApp, Slack, …).
-- Each (channel_type, channel_identifier) pair belongs to exactly one contact.
CREATE TABLE IF NOT EXISTS crm_contact_channels (
    id                 TEXT    PRIMARY KEY,
    contact_id         TEXT    NOT NULL REFERENCES crm_contacts(id) ON DELETE CASCADE,
    channel_type       TEXT    NOT NULL,
    channel_identifier TEXT    NOT NULL,
    verified           INTEGER NOT NULL DEFAULT 0,
    created_at         INTEGER NOT NULL,
    UNIQUE(channel_type, channel_identifier)
);

CREATE INDEX IF NOT EXISTS idx_crm_contact_channels_contact_id
    ON crm_contact_channels(contact_id);
