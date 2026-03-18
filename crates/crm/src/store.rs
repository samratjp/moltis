use async_trait::async_trait;

use crate::{
    Result,
    types::{
        Contact, ContactChannel, ContactStage, ContactWithChannels, Interaction, Matter,
        PracticeArea,
    },
};

/// PII-safe summary of a contact that has not been interacted with recently.
///
/// Contains name, stage, last interaction timestamp, and latest matter title.
/// Intentionally omits email and phone to prevent PII leakage into LLM prompts.
#[derive(Debug, Clone)]
pub struct StaleContactInfo {
    /// The contact's unique identifier.
    pub contact_id: String,
    /// The contact's display name.
    pub contact_name: String,
    /// The contact's current lifecycle stage.
    pub stage: ContactStage,
    /// Epoch milliseconds of the most recent interaction, or `None` if there
    /// has never been an interaction with this contact.
    pub last_interaction_at: Option<u64>,
    /// Title of the most recently updated matter for this contact, if any.
    pub matter_title: Option<String>,
}

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
    async fn list_matters(&self) -> Result<Vec<Matter>>;

    /// Return matters matching optional filters.
    ///
    /// Both filters are optional and can be combined:
    /// - `contact_id` — if set, only matters for this contact are returned.
    /// - `practice_area` — if set, only matters with this practice area are returned.
    async fn list_matters_filtered(
        &self,
        contact_id: Option<&str>,
        practice_area: Option<PracticeArea>,
    ) -> Result<Vec<Matter>>;

    /// Return all matters for a contact, ordered by most-recently-updated first.
    async fn list_matters_by_contact(&self, contact_id: &str) -> Result<Vec<Matter>>;

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

    // ── Follow-up queries ─────────────────────────────────────────────────────

    /// Return contacts that have had no interaction in the last `stale_days` days,
    /// along with their most recent matter title (if any). Results are ordered by
    /// `last_interaction_at` ascending (most stale first) and capped at `limit`.
    ///
    /// Contacts with no interactions at all are always included.
    ///
    /// The default implementation performs N+1 queries and is suitable for the
    /// in-memory store used in tests. Production [`SqliteCrmStore`] overrides this
    /// with a single efficient JOIN query.
    async fn contacts_needing_followup(
        &self,
        stale_days: u64,
        limit: usize,
    ) -> Result<Vec<StaleContactInfo>> {
        use time::OffsetDateTime;
        let now = OffsetDateTime::now_utc();
        let cutoff = now - time::Duration::days(stale_days as i64);
        let cutoff_ms = cutoff.unix_timestamp() * 1_000;

        let contacts = self.list().await?;
        let mut results: Vec<StaleContactInfo> = Vec::new();

        for contact in contacts {
            let interactions = self.list_interactions_by_contact(&contact.id).await?;
            let last_interaction_at = interactions.iter().map(|i| i.created_at).max();

            let is_stale = match last_interaction_at {
                Some(ts_ms) => (ts_ms as i64) < cutoff_ms,
                None => true,
            };

            if !is_stale {
                continue;
            }

            let matters = self.list_matters_by_contact(&contact.id).await?;
            let matter_title = matters.into_iter().next().map(|m| m.title);

            results.push(StaleContactInfo {
                contact_id: contact.id,
                contact_name: contact.name,
                stage: contact.stage,
                last_interaction_at,
                matter_title,
            });
        }

        results.sort_by_key(|c| c.last_interaction_at);
        results.truncate(limit);
        Ok(results)
    }
}

pub use crate::{store_memory::MemoryCrmStore, store_sqlite::SqliteCrmStore};
