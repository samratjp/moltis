use async_trait::async_trait;

use crate::{Result, types::Contact};

/// Persistence trait for CRM contacts.
///
/// Implementations include [`SqliteCrmStore`] for production use and
/// [`MemoryCrmStore`] for testing.
#[async_trait]
pub trait CrmStore: Send + Sync {
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
}

pub use crate::{store_memory::MemoryCrmStore, store_sqlite::SqliteCrmStore};
