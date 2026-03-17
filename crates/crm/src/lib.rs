//! CRM (Contact Relationship Management) for moltis.
//!
//! Provides a `Contact` model and persistent store for tracking people and
//! entities that interact with the system across any channel. This crate is the
//! foundational scaffold; richer models (tags, notes, interactions) are
//! intended for subsequent phases.

pub mod error;
pub mod store;
pub mod store_memory;
pub mod store_sqlite;
pub mod types;

pub use {
    error::{Error, Result},
    store::{CrmStore, MemoryCrmStore, SqliteCrmStore},
    types::{Contact, ContactChannel, Interaction, InteractionKind, Matter, MatterStatus},
};

/// Run database migrations for the CRM crate.
///
/// Creates the `crm_contacts`, `crm_matters`, `crm_interactions`, and
/// `crm_contact_channels` tables with indexes. Should be called at application
/// startup before using [`SqliteCrmStore`].
pub async fn run_migrations(pool: &sqlx::SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await?;
    Ok(())
}
