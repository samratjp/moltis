use async_trait::async_trait;

use crate::{
    Result,
    types::{Contact, ContactChannel, Interaction, Matter},
};

/// Persistence trait for CRM entities.
///
/// Covers contacts, matters, interactions, and contact channels.
/// Implementations include [`SqliteCrmStore`] for production use and
/// [`MemoryCrmStore`] for testing.
#[async_trait]
pub trait CrmStore: Send + Sync {
    // ── Contacts ──────────────────────────────────────────────────────────────

    /// Return all contacts ordered by most-recently-updated first.
    async fn list(&self) -> Result<Vec<Contact>>;

    /// Return a single contact by ID, or `None` if not found.
    async fn get(&self, id: &str) -> Result<Option<Contact>>;

    /// Look up a contact by source channel and external ID.
    async fn get_by_external(&self, source: &str, external_id: &str) -> Result<Option<Contact>>;

    /// Insert or update a contact (upsert by ID).
    async fn upsert(&self, contact: Contact) -> Result<()>;

    /// Delete a contact by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete(&self, id: &str) -> Result<()>;

    // ── Matters ───────────────────────────────────────────────────────────────

    /// Return all matters ordered by most-recently-updated first.
    async fn list_matters(&self) -> Result<Vec<Matter>>;

    /// Return matters associated with a contact, most-recently-updated first.
    async fn list_matters_by_contact(&self, contact_id: &str) -> Result<Vec<Matter>>;

    /// Return a single matter by ID, or `None` if not found.
    async fn get_matter(&self, id: &str) -> Result<Option<Matter>>;

    /// Insert or update a matter (upsert by ID).
    async fn upsert_matter(&self, matter: Matter) -> Result<()>;

    /// Delete a matter by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete_matter(&self, id: &str) -> Result<()>;

    // ── Interactions ──────────────────────────────────────────────────────────

    /// Return interactions for a contact, most-recently-created first.
    async fn list_interactions_by_contact(&self, contact_id: &str) -> Result<Vec<Interaction>>;

    /// Return interactions for a matter, most-recently-created first.
    async fn list_interactions_by_matter(&self, matter_id: &str) -> Result<Vec<Interaction>>;

    /// Insert a new interaction record.
    async fn create_interaction(&self, interaction: Interaction) -> Result<()>;

    /// Delete an interaction by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete_interaction(&self, id: &str) -> Result<()>;

    // ── Contact channels ──────────────────────────────────────────────────────

    /// Return all channels registered for a contact.
    async fn list_channels_by_contact(&self, contact_id: &str) -> Result<Vec<ContactChannel>>;

    /// Look up a channel by its type and identifier, or `None` if not found.
    async fn get_channel_by_type_and_id(
        &self,
        channel_type: &str,
        channel_identifier: &str,
    ) -> Result<Option<ContactChannel>>;

    /// Insert or update a contact channel (upsert by ID).
    async fn upsert_channel(&self, channel: ContactChannel) -> Result<()>;

    /// Delete a contact channel by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete_channel(&self, id: &str) -> Result<()>;
}

pub use crate::{store_memory::MemoryCrmStore, store_sqlite::SqliteCrmStore};
