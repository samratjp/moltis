-- Add indexes on crm_matters columns used by list_matters_filtered.
-- phase and practice_area are queried as equality predicates; an index
-- on each column speeds up filtered listing without the cost of a composite
-- index (individual indexes let SQLite choose the tightest one per query).
-- Owned by: moltis-crm crate

CREATE INDEX IF NOT EXISTS idx_crm_matters_phase
    ON crm_matters(phase);

CREATE INDEX IF NOT EXISTS idx_crm_matters_practice_area
    ON crm_matters(practice_area);
