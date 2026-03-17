-- Enrich crm_* tables with additional columns for the full CRM type model.
-- Owned by: moltis-crm crate
--
-- This migration runs after 20260317000000_init.sql which created the base
-- crm_contacts, crm_matters, crm_interactions, and crm_contact_channels tables.
-- SQLite supports ADD COLUMN and RENAME COLUMN (≥3.25.0) but not DROP COLUMN
-- in older versions; new columns use nullable or DEFAULT-bearing types.

-- ── crm_contacts: add PII and stage columns ───────────────────────────────────
ALTER TABLE crm_contacts ADD COLUMN email TEXT;
ALTER TABLE crm_contacts ADD COLUMN phone TEXT;
ALTER TABLE crm_contacts ADD COLUMN stage TEXT NOT NULL DEFAULT 'lead';

-- ── crm_contact_channels: rename identifier field, add display_name + updated_at ─
-- channel_identifier → channel_id (aligns with domain type field name)
ALTER TABLE crm_contact_channels RENAME COLUMN channel_identifier TO channel_id;
ALTER TABLE crm_contact_channels ADD COLUMN display_name TEXT;
ALTER TABLE crm_contact_channels ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;

-- Rebuild unique index with new column name.
DROP INDEX IF EXISTS idx_crm_contact_channels_contact_id;
CREATE INDEX IF NOT EXISTS idx_crm_contact_channels_contact_id
    ON crm_contact_channels(contact_id);

-- The UNIQUE(channel_type, channel_identifier) constraint was declared inline on
-- the table; SQLite preserves it through RENAME COLUMN automatically.

-- ── crm_matters: add description, phase, practice_area ────────────────────────
ALTER TABLE crm_matters ADD COLUMN description TEXT;
ALTER TABLE crm_matters ADD COLUMN phase         TEXT NOT NULL DEFAULT 'intake';
ALTER TABLE crm_matters ADD COLUMN practice_area TEXT NOT NULL DEFAULT 'other';

-- ── crm_interactions: rename body → summary, add channel + updated_at ─────────
ALTER TABLE crm_interactions RENAME COLUMN body TO summary;
ALTER TABLE crm_interactions ADD COLUMN channel    TEXT;
ALTER TABLE crm_interactions ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;

-- Rebuild interaction indexes after schema change.
DROP INDEX IF EXISTS idx_crm_interactions_contact_id;
DROP INDEX IF EXISTS idx_crm_interactions_matter_id;
DROP INDEX IF EXISTS idx_crm_interactions_created_at;

CREATE INDEX IF NOT EXISTS idx_crm_interactions_contact_id
    ON crm_interactions(contact_id);

CREATE INDEX IF NOT EXISTS idx_crm_interactions_matter_id
    ON crm_interactions(matter_id)
    WHERE matter_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_interactions_updated_at
    ON crm_interactions(updated_at DESC);
