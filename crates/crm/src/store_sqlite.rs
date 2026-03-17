use {async_trait::async_trait, sqlx::SqlitePool};

use crate::{
    Result,
    store::CrmStore,
    types::{Contact, ContactChannel, Interaction, InteractionKind, Matter, MatterStatus},
};

/// SQLite-backed implementation of [`CrmStore`].
pub struct SqliteCrmStore {
    pool: SqlitePool,
}

impl SqliteCrmStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

// ── Row types ─────────────────────────────────────────────────────────────────

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

#[derive(sqlx::FromRow)]
struct MatterRow {
    id: String,
    title: String,
    status: String,
    contact_id: Option<String>,
    metadata: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<MatterRow> for Matter {
    type Error = crate::Error;

    fn try_from(r: MatterRow) -> Result<Self> {
        Ok(Matter {
            id: r.id,
            title: r.title,
            status: MatterStatus::try_from(r.status.as_str())?,
            contact_id: r.contact_id,
            metadata: serde_json::from_str(r.metadata.as_deref().unwrap_or("{}"))?,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
        })
    }
}

#[derive(sqlx::FromRow)]
struct InteractionRow {
    id: String,
    contact_id: String,
    matter_id: Option<String>,
    kind: String,
    body: Option<String>,
    metadata: Option<String>,
    created_at: i64,
}

impl TryFrom<InteractionRow> for Interaction {
    type Error = crate::Error;

    fn try_from(r: InteractionRow) -> Result<Self> {
        Ok(Interaction {
            id: r.id,
            contact_id: r.contact_id,
            matter_id: r.matter_id,
            kind: InteractionKind::try_from(r.kind.as_str())?,
            body: r.body,
            metadata: serde_json::from_str(r.metadata.as_deref().unwrap_or("{}"))?,
            created_at: r.created_at as u64,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ContactChannelRow {
    id: String,
    contact_id: String,
    channel_type: String,
    channel_identifier: String,
    verified: i64,
    created_at: i64,
}

impl From<ContactChannelRow> for ContactChannel {
    fn from(r: ContactChannelRow) -> Self {
        ContactChannel {
            id: r.id,
            contact_id: r.contact_id,
            channel_type: r.channel_type,
            channel_identifier: r.channel_identifier,
            verified: r.verified != 0,
            created_at: r.created_at as u64,
        }
    }
}

// ── CrmStore impl ─────────────────────────────────────────────────────────────

#[async_trait]
impl CrmStore for SqliteCrmStore {
    // ── Contacts ──────────────────────────────────────────────────────────────

    async fn list(&self) -> Result<Vec<Contact>> {
        let rows = sqlx::query_as::<_, ContactRow>(
            "SELECT id, name, source, external_id, metadata, created_at, updated_at \
             FROM crm_contacts ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Contact::try_from).collect()
    }

    async fn get(&self, id: &str) -> Result<Option<Contact>> {
        let row = sqlx::query_as::<_, ContactRow>(
            "SELECT id, name, source, external_id, metadata, created_at, updated_at \
             FROM crm_contacts WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Contact::try_from).transpose()
    }

    async fn get_by_external(&self, source: &str, external_id: &str) -> Result<Option<Contact>> {
        let row = sqlx::query_as::<_, ContactRow>(
            "SELECT id, name, source, external_id, metadata, created_at, updated_at \
             FROM crm_contacts WHERE source = ? AND external_id = ?",
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
            r#"INSERT INTO crm_contacts (id, name, source, external_id, metadata, created_at, updated_at)
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
        sqlx::query("DELETE FROM crm_contacts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Matters ───────────────────────────────────────────────────────────────

    async fn list_matters(&self) -> Result<Vec<Matter>> {
        let rows = sqlx::query_as::<_, MatterRow>(
            "SELECT id, title, status, contact_id, metadata, created_at, updated_at \
             FROM crm_matters ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Matter::try_from).collect()
    }

    async fn list_matters_by_contact(&self, contact_id: &str) -> Result<Vec<Matter>> {
        let rows = sqlx::query_as::<_, MatterRow>(
            "SELECT id, title, status, contact_id, metadata, created_at, updated_at \
             FROM crm_matters WHERE contact_id = ? ORDER BY updated_at DESC",
        )
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Matter::try_from).collect()
    }

    async fn get_matter(&self, id: &str) -> Result<Option<Matter>> {
        let row = sqlx::query_as::<_, MatterRow>(
            "SELECT id, title, status, contact_id, metadata, created_at, updated_at \
             FROM crm_matters WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Matter::try_from).transpose()
    }

    async fn upsert_matter(&self, matter: Matter) -> Result<()> {
        let metadata = serde_json::to_string(&matter.metadata)?;
        sqlx::query(
            r#"INSERT INTO crm_matters (id, title, status, contact_id, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 title      = excluded.title,
                 status     = excluded.status,
                 contact_id = excluded.contact_id,
                 metadata   = excluded.metadata,
                 updated_at = excluded.updated_at"#,
        )
        .bind(&matter.id)
        .bind(&matter.title)
        .bind(matter.status.as_str())
        .bind(&matter.contact_id)
        .bind(&metadata)
        .bind(matter.created_at as i64)
        .bind(matter.updated_at as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_matter(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM crm_matters WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Interactions ──────────────────────────────────────────────────────────

    async fn list_interactions_by_contact(&self, contact_id: &str) -> Result<Vec<Interaction>> {
        let rows = sqlx::query_as::<_, InteractionRow>(
            "SELECT id, contact_id, matter_id, kind, body, metadata, created_at \
             FROM crm_interactions WHERE contact_id = ? ORDER BY created_at DESC",
        )
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Interaction::try_from).collect()
    }

    async fn list_interactions_by_matter(&self, matter_id: &str) -> Result<Vec<Interaction>> {
        let rows = sqlx::query_as::<_, InteractionRow>(
            "SELECT id, contact_id, matter_id, kind, body, metadata, created_at \
             FROM crm_interactions WHERE matter_id = ? ORDER BY created_at DESC",
        )
        .bind(matter_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Interaction::try_from).collect()
    }

    async fn create_interaction(&self, interaction: Interaction) -> Result<()> {
        let metadata = serde_json::to_string(&interaction.metadata)?;
        sqlx::query(
            "INSERT INTO crm_interactions \
             (id, contact_id, matter_id, kind, body, metadata, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&interaction.id)
        .bind(&interaction.contact_id)
        .bind(&interaction.matter_id)
        .bind(interaction.kind.as_str())
        .bind(&interaction.body)
        .bind(&metadata)
        .bind(interaction.created_at as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_interaction(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM crm_interactions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Contact channels ──────────────────────────────────────────────────────

    async fn list_channels_by_contact(&self, contact_id: &str) -> Result<Vec<ContactChannel>> {
        let rows = sqlx::query_as::<_, ContactChannelRow>(
            "SELECT id, contact_id, channel_type, channel_identifier, verified, created_at \
             FROM crm_contact_channels WHERE contact_id = ? ORDER BY created_at ASC",
        )
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ContactChannel::from).collect())
    }

    async fn get_channel_by_type_and_id(
        &self,
        channel_type: &str,
        channel_identifier: &str,
    ) -> Result<Option<ContactChannel>> {
        let row = sqlx::query_as::<_, ContactChannelRow>(
            "SELECT id, contact_id, channel_type, channel_identifier, verified, created_at \
             FROM crm_contact_channels \
             WHERE channel_type = ? AND channel_identifier = ?",
        )
        .bind(channel_type)
        .bind(channel_identifier)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ContactChannel::from))
    }

    async fn upsert_channel(&self, channel: ContactChannel) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO crm_contact_channels
               (id, contact_id, channel_type, channel_identifier, verified, created_at)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 contact_id         = excluded.contact_id,
                 channel_type       = excluded.channel_type,
                 channel_identifier = excluded.channel_identifier,
                 verified           = excluded.verified"#,
        )
        .bind(&channel.id)
        .bind(&channel.contact_id)
        .bind(&channel.channel_type)
        .bind(&channel.channel_identifier)
        .bind(channel.verified as i64)
        .bind(channel.created_at as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_channel(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM crm_contact_channels WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {
        super::*,
        crate::types::{Contact, ContactChannel, Interaction, InteractionKind, Matter},
    };

    async fn test_store() -> SqliteCrmStore {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::run_migrations(&pool).await.unwrap();
        SqliteCrmStore::new(pool)
    }

    // ── Contact tests ─────────────────────────────────────────────────────────

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

    // ── Matter tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_matter() {
        let store = test_store().await;
        let m = Matter::new("Test deal");
        let id = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        let found = store.get_matter(&id).await.unwrap().unwrap();
        assert_eq!(found.title, "Test deal");
        assert_eq!(found.status, MatterStatus::Open);
    }

    #[tokio::test]
    async fn list_matters_by_contact_filters_correctly() {
        let store = test_store().await;
        let contact = Contact::new("Eve");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let m1 = Matter::for_contact("Deal A", &cid);
        let m2 = Matter::new("Unrelated deal");
        store.upsert_matter(m1.clone()).await.unwrap();
        store.upsert_matter(m2).await.unwrap();

        let by_contact = store.list_matters_by_contact(&cid).await.unwrap();
        assert_eq!(by_contact.len(), 1);
        assert_eq!(by_contact[0].id, m1.id);
    }

    #[tokio::test]
    async fn delete_matter_is_idempotent() {
        let store = test_store().await;
        let m = Matter::new("Ephemeral");
        let id = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        store.delete_matter(&id).await.unwrap();
        store.delete_matter(&id).await.unwrap();
        assert!(store.get_matter(&id).await.unwrap().is_none());
    }

    // ── Interaction tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_list_interaction_by_contact() {
        let store = test_store().await;
        let contact = Contact::new("Irene");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let i = Interaction::new(&cid, InteractionKind::Message, Some("Hello".into()));
        store.create_interaction(i.clone()).await.unwrap();

        let interactions = store.list_interactions_by_contact(&cid).await.unwrap();
        assert_eq!(interactions.len(), 1);
        assert_eq!(interactions[0].id, i.id);
        assert_eq!(interactions[0].kind, InteractionKind::Message);
        assert_eq!(interactions[0].body.as_deref(), Some("Hello"));
    }

    #[tokio::test]
    async fn list_interactions_by_matter() {
        let store = test_store().await;
        let contact = Contact::new("Jack");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let matter = Matter::for_contact("Project X", &cid);
        let mid = matter.id.clone();
        store.upsert_matter(matter).await.unwrap();

        let mut i = Interaction::new(&cid, InteractionKind::Note, Some("Note 1".into()));
        i.matter_id = Some(mid.clone());
        store.create_interaction(i.clone()).await.unwrap();

        // Another interaction without matter_id
        store
            .create_interaction(Interaction::new(&cid, InteractionKind::Call, None))
            .await
            .unwrap();

        let by_matter = store.list_interactions_by_matter(&mid).await.unwrap();
        assert_eq!(by_matter.len(), 1);
        assert_eq!(by_matter[0].id, i.id);
    }

    #[tokio::test]
    async fn delete_interaction_is_idempotent() {
        let store = test_store().await;
        let contact = Contact::new("Kate");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let i = Interaction::new(&cid, InteractionKind::Meeting, None);
        let iid = i.id.clone();
        store.create_interaction(i).await.unwrap();
        store.delete_interaction(&iid).await.unwrap();
        store.delete_interaction(&iid).await.unwrap();
        assert!(
            store
                .list_interactions_by_contact(&cid)
                .await
                .unwrap()
                .is_empty()
        );
    }

    // ── ContactChannel tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_list_channels() {
        let store = test_store().await;
        let contact = Contact::new("Leo");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let ch = ContactChannel::new(&cid, "telegram", "123456");
        store.upsert_channel(ch.clone()).await.unwrap();

        let channels = store.list_channels_by_contact(&cid).await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].channel_type, "telegram");
        assert_eq!(channels[0].channel_identifier, "123456");
        assert!(!channels[0].verified);
    }

    #[tokio::test]
    async fn get_channel_by_type_and_id_finds_channel() {
        let store = test_store().await;
        let contact = Contact::new("Mia");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let ch = ContactChannel::new(&cid, "whatsapp", "+1555000");
        store.upsert_channel(ch.clone()).await.unwrap();

        let found = store
            .get_channel_by_type_and_id("whatsapp", "+1555000")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, ch.id);
        assert_eq!(found.contact_id, cid);
    }

    #[tokio::test]
    async fn get_channel_returns_none_for_unknown() {
        let store = test_store().await;
        assert!(
            store
                .get_channel_by_type_and_id("telegram", "does-not-exist")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn delete_channel_is_idempotent() {
        let store = test_store().await;
        let contact = Contact::new("Nick");
        let cid = contact.id.clone();
        store.upsert(contact).await.unwrap();

        let ch = ContactChannel::new(&cid, "slack", "U999");
        let chid = ch.id.clone();
        store.upsert_channel(ch).await.unwrap();
        store.delete_channel(&chid).await.unwrap();
        store.delete_channel(&chid).await.unwrap();
        assert!(
            store
                .list_channels_by_contact(&cid)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
