//! CRM (Contact Relationship Management) for moltis.
//!
//! Provides domain types and persistent stores for tracking people and entities
//! that interact with the system across any channel.
//!
//! # Core types
//!
//! - [`Contact`] — a person or entity, optionally with PII fields (`email`, `phone`)
//! - [`ContactChannel`] — a channel identity linking a contact to an external platform
//! - [`Matter`] — a legal case or work item linked to a contact
//! - [`Interaction`] — a recorded touchpoint between the system and a contact
//!
//! # Enums
//!
//! - [`ContactStage`] — contact lifecycle stage
//! - [`MatterStatus`] / [`MatterPhase`] — matter lifecycle state and phase
//! - [`PracticeArea`] — legal practice area
//! - [`InteractionKind`] — type of interaction

pub mod error;
pub mod store;
pub mod store_memory;
pub mod store_sqlite;
pub mod types;

pub use {
    error::{Error, Result},
    store::{CrmStore, MemoryCrmStore, SqliteCrmStore},
    types::{
        Contact, ContactChannel, ContactStage, ContactWithChannels, Interaction, InteractionKind,
        Matter, MatterPhase, MatterStatus, PracticeArea,
    },
};

/// Run database migrations for the CRM crate.
///
/// Creates and updates the `crm_contacts`, `crm_matters`, `crm_interactions`,
/// and `crm_contact_channels` tables with indexes. Should be called at
/// application startup before using [`SqliteCrmStore`].
pub async fn run_migrations(pool: &sqlx::SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await?;
    Ok(())
}
