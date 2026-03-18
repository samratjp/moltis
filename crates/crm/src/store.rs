use async_trait::async_trait;

use crate::{
    Result,
    types::{
        Contact, ContactChannel, ContactStage, ContactWithChannels, Interaction, Matter,
        MatterPhase, MatterStatus, PracticeArea,
    },
};

/// Persistence trait for CRM data.
///
/// Provides CRUD operations for [`Contact`], [`Matter`], [`Interaction`], and
/// [`ContactChannel`] records. Implementations include [`SqliteCrmStore`] for
/// production use and [`MemoryCrmStore`] for testing.
#[async_trait]
pub trait CrmStore: Send + Sync {
    // ── Contacts ──────────────────────────────────────────────────────────────

    /// Return all contacts ordered by most-recently-updated first.
    async fn list(&self) -> Result<Vec<Contact>>;

    /// Return contacts matching optional filters, with pagination.
    ///
    /// - `stage` — if set, only contacts in this stage are returned.
    /// - `search` — if set, filters by a case-insensitive substring match
    ///   against `name`, `email`, and `phone`.
    /// - `offset` / `limit` — pagination (default: 0 / 50).
    async fn list_filtered(
        &self,
        stage: Option<ContactStage>,
        search: Option<&str>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Contact>>;

    /// Return a single contact by ID, or `None` if not found.
    async fn get(&self, id: &str) -> Result<Option<Contact>>;

    /// Look up a contact by source channel and external ID.
    async fn get_by_external(&self, source: &str, external_id: &str) -> Result<Option<Contact>>;

    /// Insert or update a contact (upsert by ID).
    async fn upsert(&self, contact: Contact) -> Result<()>;

    /// Delete a contact by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete(&self, id: &str) -> Result<()>;

    /// Return a contact together with all its channel identities.
    ///
    /// Provides the common view of a contact and its communication channels in
    /// one call rather than two. Default implementation calls [`Self::get`] and
    /// [`Self::list_channels_for_contact`].
    async fn get_with_channels(&self, id: &str) -> Result<Option<ContactWithChannels>> {
        let Some(contact) = self.get(id).await? else {
            return Ok(None);
        };
        let channels = self.list_channels_for_contact(id).await?;
        Ok(Some(ContactWithChannels { contact, channels }))
    }

    // ── Contact channels ──────────────────────────────────────────────────────

    /// Return all channel identities for a contact.
    async fn list_channels_for_contact(&self, contact_id: &str) -> Result<Vec<ContactChannel>>;

    /// Look up a channel identity by channel type and channel-native ID.
    async fn get_channel_by_external(
        &self,
        channel_type: &str,
        channel_id: &str,
    ) -> Result<Option<ContactChannel>>;

    /// Insert or update a channel identity (upsert by ID).
    async fn upsert_channel(&self, channel: ContactChannel) -> Result<()>;

    /// Delete a channel identity by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete_channel(&self, id: &str) -> Result<()>;

    // ── Matters ───────────────────────────────────────────────────────────────

    /// Return all matters ordered by most-recently-updated first.
    ///
    /// Default implementation delegates to [`Self::list_matters_filtered`] with
    /// no filters and an unbounded result set.
    async fn list_matters(&self) -> Result<Vec<Matter>> {
        self.list_matters_filtered(None, None, None, None, None, 0, u64::MAX)
            .await
    }

    /// Return matters matching optional filters, with pagination.
    ///
    /// All parameters are optional and can be combined with AND logic:
    /// - `contact_id` — only matters linked to this contact.
    /// - `status` — only matters in this lifecycle status.
    /// - `phase` — only matters in this phase.
    /// - `practice_area` — only matters with this practice area.
    /// - `search` — case-insensitive substring match against `title` and `description`.
    /// - `offset` / `limit` — pagination (default: 0 / all).
    async fn list_matters_filtered(
        &self,
        contact_id: Option<&str>,
        status: Option<MatterStatus>,
        phase: Option<MatterPhase>,
        practice_area: Option<PracticeArea>,
        search: Option<&str>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Matter>>;

    /// Return all matters for a contact, ordered by most-recently-updated first.
    ///
    /// Default implementation delegates to [`Self::list_matters_filtered`].
    async fn list_matters_by_contact(&self, contact_id: &str) -> Result<Vec<Matter>> {
        self.list_matters_filtered(Some(contact_id), None, None, None, None, 0, u64::MAX)
            .await
    }

    /// Return a single matter by ID, or `None` if not found.
    async fn get_matter(&self, id: &str) -> Result<Option<Matter>>;

    /// Insert or update a matter (upsert by ID).
    async fn upsert_matter(&self, matter: Matter) -> Result<()>;

    /// Delete a matter by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete_matter(&self, id: &str) -> Result<()>;

    // ── Interactions ──────────────────────────────────────────────────────────

    /// Return all interactions for a contact, ordered by most-recently-updated first.
    async fn list_interactions_by_contact(&self, contact_id: &str) -> Result<Vec<Interaction>>;

    /// Return all interactions for a matter, ordered by most-recently-updated first.
    async fn list_interactions_by_matter(&self, matter_id: &str) -> Result<Vec<Interaction>>;

    /// Return a single interaction by ID, or `None` if not found.
    async fn get_interaction(&self, id: &str) -> Result<Option<Interaction>>;

    /// Insert or update an interaction (upsert by ID).
    async fn upsert_interaction(&self, interaction: Interaction) -> Result<()>;

    /// Delete an interaction by ID. Returns `Ok(())` if not found (idempotent).
    async fn delete_interaction(&self, id: &str) -> Result<()>;

    /// Delete all interactions whose `created_at` (epoch milliseconds) is older than
    /// `cutoff_epoch_ms`. Returns the number of rows deleted.
    #[must_use]
    async fn delete_interactions_before(&self, cutoff_epoch_ms: i64) -> Result<u64>;
}

pub use crate::{store_memory::MemoryCrmStore, store_sqlite::SqliteCrmStore};
