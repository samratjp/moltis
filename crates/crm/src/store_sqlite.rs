use {async_trait::async_trait, sqlx::SqlitePool};

use crate::{Result, store::CrmStore, types::Contact};

/// SQLite-backed implementation of [`CrmStore`].
pub struct SqliteCrmStore {
    pool: SqlitePool,
}

impl SqliteCrmStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Internal row type for sqlx mapping.
#[derive(sqlx::FromRow)]
struct ContactRow {
    id: String,
    name: String,
    source: Option<String>,
    external_id: Option<String>,
    metadata: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<ContactRow> for Contact {
    type Error = crate::Error;

    fn try_from(r: ContactRow) -> Result<Self> {
        Ok(Contact {
            id: r.id,
            name: r.name,
            source: r.source,
            external_id: r.external_id,
            metadata: serde_json::from_str(r.metadata.as_deref().unwrap_or("{}"))?,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
        })
    }
}

#[async_trait]
impl CrmStore for SqliteCrmStore {
    async fn list(&self) -> Result<Vec<Contact>> {
        let rows = sqlx::query_as::<_, ContactRow>(
            "SELECT id, name, source, external_id, metadata, created_at, updated_at \
             FROM contacts ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Contact::try_from).collect()
    }

    async fn get(&self, id: &str) -> Result<Option<Contact>> {
        let row = sqlx::query_as::<_, ContactRow>(
            "SELECT id, name, source, external_id, metadata, created_at, updated_at \
             FROM contacts WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Contact::try_from).transpose()
    }

    async fn get_by_external(&self, source: &str, external_id: &str) -> Result<Option<Contact>> {
        let row = sqlx::query_as::<_, ContactRow>(
            "SELECT id, name, source, external_id, metadata, created_at, updated_at \
             FROM contacts WHERE source = ? AND external_id = ?",
        )
        .bind(source)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Contact::try_from).transpose()
    }

    async fn upsert(&self, contact: Contact) -> Result<()> {
        let metadata = serde_json::to_string(&contact.metadata)?;
        sqlx::query(
            r#"INSERT INTO contacts (id, name, source, external_id, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 name        = excluded.name,
                 source      = excluded.source,
                 external_id = excluded.external_id,
                 metadata    = excluded.metadata,
                 updated_at  = excluded.updated_at"#,
        )
        .bind(&contact.id)
        .bind(&contact.name)
        .bind(&contact.source)
        .bind(&contact.external_id)
        .bind(&metadata)
        .bind(contact.created_at as i64)
        .bind(contact.updated_at as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM contacts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {super::*, crate::types::Contact};

    async fn test_store() -> SqliteCrmStore {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::run_migrations(&pool).await.unwrap();
        SqliteCrmStore::new(pool)
    }

    #[tokio::test]
    async fn upsert_and_get() {
        let store = test_store().await;
        let c = Contact::new("Alice");
        let id = c.id.clone();
        store.upsert(c).await.unwrap();
        let found = store.get(&id).await.unwrap().unwrap();
        assert_eq!(found.name, "Alice");
    }

    #[tokio::test]
    async fn upsert_updates_existing() {
        let store = test_store().await;
        let mut c = Contact::new("Alice");
        store.upsert(c.clone()).await.unwrap();
        c.name = "Alice Updated".into();
        store.upsert(c.clone()).await.unwrap();
        let found = store.get(&c.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Alice Updated");
    }

    #[tokio::test]
    async fn list_returns_all() {
        let store = test_store().await;
        store.upsert(Contact::new("A")).await.unwrap();
        store.upsert(Contact::new("B")).await.unwrap();
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let store = test_store().await;
        assert!(store.get("missing-id").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let store = test_store().await;
        let c = Contact::new("X");
        let id = c.id.clone();
        store.upsert(c).await.unwrap();
        store.delete(&id).await.unwrap();
        store.delete(&id).await.unwrap();
        assert!(store.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_by_external_finds_contact() {
        let store = test_store().await;
        let c = Contact::with_source("Bob", "telegram", "99999");
        store.upsert(c).await.unwrap();
        let found = store
            .get_by_external("telegram", "99999")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Bob");
    }

    #[tokio::test]
    async fn get_by_external_returns_none_for_wrong_source() {
        let store = test_store().await;
        let c = Contact::with_source("Bob", "telegram", "99999");
        store.upsert(c).await.unwrap();
        assert!(
            store
                .get_by_external("slack", "99999")
                .await
                .unwrap()
                .is_none()
        );
    }
}
