use {async_trait::async_trait, secrecy::Secret, sqlx::SqlitePool};

use crate::{
    Error, Result,
    store::CrmStore,
    types::{
        Contact, ContactChannel, ContactStage, Interaction, InteractionKind, Matter, MatterPhase,
        MatterStatus, PracticeArea,
    },
};

const SELECT_CONTACTS: &str = "SELECT id, name, source, external_id, email, phone, stage, metadata, created_at, updated_at FROM crm_contacts";
const SELECT_MATTERS: &str = "SELECT id, contact_id, title, description, status, phase, practice_area, created_at, updated_at FROM crm_matters";
const SELECT_INTERACTIONS: &str = "SELECT id, contact_id, matter_id, kind, summary, channel, created_at, updated_at FROM crm_interactions";
const SELECT_CHANNELS: &str = "SELECT id, contact_id, channel_type, channel_id, display_name, verified, created_at, updated_at FROM crm_contact_channels";

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
    email: Option<String>,
    phone: Option<String>,
    stage: String,
    metadata: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<ContactRow> for Contact {
    type Error = Error;

    fn try_from(r: ContactRow) -> Result<Self> {
        let stage = r
            .stage
            .parse::<ContactStage>()
            .map_err(|_| Error::parse("contact.stage", &r.stage))?;
        Ok(Contact {
            id: r.id,
            name: r.name,
            source: r.source,
            external_id: r.external_id,
            email: r.email.map(Secret::new),
            phone: r.phone.map(Secret::new),
            stage,
            metadata: serde_json::from_str(r.metadata.as_deref().unwrap_or("{}"))?,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ContactChannelRow {
    id: String,
    contact_id: String,
    channel_type: String,
    channel_id: String,
    display_name: Option<String>,
    verified: i64,
    created_at: i64,
    updated_at: i64,
}

impl From<ContactChannelRow> for ContactChannel {
    fn from(r: ContactChannelRow) -> Self {
        Self {
            id: r.id,
            contact_id: r.contact_id,
            channel_type: r.channel_type,
            channel_id: r.channel_id,
            display_name: r.display_name,
            verified: r.verified != 0,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
        }
    }
}

#[derive(sqlx::FromRow)]
struct MatterRow {
    id: String,
    contact_id: Option<String>,
    title: String,
    description: Option<String>,
    status: String,
    phase: String,
    practice_area: String,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<MatterRow> for Matter {
    type Error = Error;

    fn try_from(r: MatterRow) -> Result<Self> {
        let status = r
            .status
            .parse::<MatterStatus>()
            .map_err(|_| Error::parse("matter.status", &r.status))?;
        let phase = r
            .phase
            .parse::<MatterPhase>()
            .map_err(|_| Error::parse("matter.phase", &r.phase))?;
        let practice_area = r
            .practice_area
            .parse::<PracticeArea>()
            .map_err(|_| Error::parse("matter.practice_area", &r.practice_area))?;
        Ok(Matter {
            id: r.id,
            contact_id: r.contact_id,
            title: r.title,
            description: r.description,
            status,
            phase,
            practice_area,
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
    summary: String,
    channel: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl TryFrom<InteractionRow> for Interaction {
    type Error = Error;

    fn try_from(r: InteractionRow) -> Result<Self> {
        let kind = r
            .kind
            .parse::<InteractionKind>()
            .map_err(|_| Error::parse("interaction.kind", &r.kind))?;
        Ok(Interaction {
            id: r.id,
            contact_id: r.contact_id,
            matter_id: r.matter_id,
            kind,
            summary: r.summary,
            channel: r.channel,
            created_at: r.created_at as u64,
            updated_at: r.updated_at as u64,
        })
    }
}

// ── CrmStore implementation ───────────────────────────────────────────────────

#[async_trait]
impl CrmStore for SqliteCrmStore {
    // ── Contacts ──────────────────────────────────────────────────────────────

    async fn list(&self) -> Result<Vec<Contact>> {
        let rows =
            sqlx::query_as::<_, ContactRow>(&format!("{SELECT_CONTACTS} ORDER BY updated_at DESC"))
                .fetch_all(&self.pool)
                .await?;

        rows.into_iter().map(Contact::try_from).collect()
    }

    async fn list_filtered(
        &self,
        stage: Option<ContactStage>,
        search: Option<&str>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Contact>> {
        let mut sql = format!("{SELECT_CONTACTS} WHERE 1=1");
        if stage.is_some() {
            sql.push_str(" AND stage = ?");
        }
        if search.is_some() {
            sql.push_str(" AND (name LIKE ? OR email LIKE ? OR phone LIKE ?)");
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, ContactRow>(&sql);
        if let Some(s) = stage {
            q = q.bind(s.as_str());
        }
        if let Some(term) = search {
            let pattern = format!("%{term}%");
            q = q.bind(pattern.clone()).bind(pattern.clone()).bind(pattern);
        }
        q = q.bind(limit as i64).bind(offset as i64);

        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(Contact::try_from).collect()
    }

    async fn get(&self, id: &str) -> Result<Option<Contact>> {
        let row = sqlx::query_as::<_, ContactRow>(&format!("{SELECT_CONTACTS} WHERE id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(Contact::try_from).transpose()
    }

    async fn get_by_external(&self, source: &str, external_id: &str) -> Result<Option<Contact>> {
        let row = sqlx::query_as::<_, ContactRow>(&format!(
            "{SELECT_CONTACTS} WHERE source = ? AND external_id = ?"
        ))
        .bind(source)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(Contact::try_from).transpose()
    }

    async fn upsert(&self, contact: Contact) -> Result<()> {
        use secrecy::ExposeSecret;
        let metadata = serde_json::to_string(&contact.metadata)?;
        sqlx::query(
            r#"INSERT INTO crm_contacts
               (id, name, source, external_id, email, phone, stage, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 name        = excluded.name,
                 source      = excluded.source,
                 external_id = excluded.external_id,
                 email       = excluded.email,
                 phone       = excluded.phone,
                 stage       = excluded.stage,
                 metadata    = excluded.metadata,
                 updated_at  = excluded.updated_at"#,
        )
        .bind(&contact.id)
        .bind(&contact.name)
        .bind(&contact.source)
        .bind(&contact.external_id)
        .bind(contact.email.as_ref().map(|s| s.expose_secret().clone()))
        .bind(contact.phone.as_ref().map(|s| s.expose_secret().clone()))
        .bind(contact.stage.as_str())
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

    // ── Contact channels ──────────────────────────────────────────────────────

    async fn list_channels_for_contact(&self, contact_id: &str) -> Result<Vec<ContactChannel>> {
        let rows = sqlx::query_as::<_, ContactChannelRow>(&format!(
            "{SELECT_CHANNELS} WHERE contact_id = ? ORDER BY updated_at DESC"
        ))
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(ContactChannel::from).collect())
    }

    async fn get_channel_by_external(
        &self,
        channel_type: &str,
        channel_id: &str,
    ) -> Result<Option<ContactChannel>> {
        let row = sqlx::query_as::<_, ContactChannelRow>(&format!(
            "{SELECT_CHANNELS} WHERE channel_type = ? AND channel_id = ?"
        ))
        .bind(channel_type)
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(ContactChannel::from))
    }

    async fn upsert_channel(&self, channel: ContactChannel) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO crm_contact_channels
               (id, contact_id, channel_type, channel_id, display_name, verified, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 contact_id   = excluded.contact_id,
                 channel_type = excluded.channel_type,
                 channel_id   = excluded.channel_id,
                 display_name = excluded.display_name,
                 verified     = excluded.verified,
                 updated_at   = excluded.updated_at"#,
        )
        .bind(&channel.id)
        .bind(&channel.contact_id)
        .bind(&channel.channel_type)
        .bind(&channel.channel_id)
        .bind(&channel.display_name)
        .bind(channel.verified as i64)
        .bind(channel.created_at as i64)
        .bind(channel.updated_at as i64)
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

    // ── Matters ───────────────────────────────────────────────────────────────

    async fn list_matters(&self) -> Result<Vec<Matter>> {
        let rows =
            sqlx::query_as::<_, MatterRow>(&format!("{SELECT_MATTERS} ORDER BY updated_at DESC"))
                .fetch_all(&self.pool)
                .await?;

        rows.into_iter().map(Matter::try_from).collect()
    }

    async fn list_matters_filtered(
        &self,
        contact_id: Option<&str>,
        practice_area: Option<PracticeArea>,
    ) -> Result<Vec<Matter>> {
        let mut sql = format!("{SELECT_MATTERS} WHERE 1=1");
        if contact_id.is_some() {
            sql.push_str(" AND contact_id = ?");
        }
        if practice_area.is_some() {
            sql.push_str(" AND practice_area = ?");
        }
        sql.push_str(" ORDER BY updated_at DESC");

        let mut q = sqlx::query_as::<_, MatterRow>(&sql);
        if let Some(cid) = contact_id {
            q = q.bind(cid);
        }
        if let Some(pa) = practice_area {
            q = q.bind(pa.as_str());
        }
        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(Matter::try_from).collect()
    }

    async fn list_matters_by_contact(&self, contact_id: &str) -> Result<Vec<Matter>> {
        let rows = sqlx::query_as::<_, MatterRow>(&format!(
            "{SELECT_MATTERS} WHERE contact_id = ? ORDER BY updated_at DESC"
        ))
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Matter::try_from).collect()
    }

    async fn get_matter(&self, id: &str) -> Result<Option<Matter>> {
        let row = sqlx::query_as::<_, MatterRow>(&format!("{SELECT_MATTERS} WHERE id = ?"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(Matter::try_from).transpose()
    }

    async fn upsert_matter(&self, matter: Matter) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO crm_matters
               (id, contact_id, title, description, status, phase, practice_area, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 contact_id    = excluded.contact_id,
                 title         = excluded.title,
                 description   = excluded.description,
                 status        = excluded.status,
                 phase         = excluded.phase,
                 practice_area = excluded.practice_area,
                 updated_at    = excluded.updated_at"#,
        )
        .bind(&matter.id)
        .bind(&matter.contact_id)
        .bind(&matter.title)
        .bind(&matter.description)
        .bind(matter.status.as_str())
        .bind(matter.phase.as_str())
        .bind(matter.practice_area.as_str())
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
        let rows = sqlx::query_as::<_, InteractionRow>(&format!(
            "{SELECT_INTERACTIONS} WHERE contact_id = ? ORDER BY updated_at DESC"
        ))
        .bind(contact_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Interaction::try_from).collect()
    }

    async fn list_interactions_by_matter(&self, matter_id: &str) -> Result<Vec<Interaction>> {
        let rows = sqlx::query_as::<_, InteractionRow>(&format!(
            "{SELECT_INTERACTIONS} WHERE matter_id = ? ORDER BY updated_at DESC"
        ))
        .bind(matter_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Interaction::try_from).collect()
    }

    async fn get_interaction(&self, id: &str) -> Result<Option<Interaction>> {
        let row =
            sqlx::query_as::<_, InteractionRow>(&format!("{SELECT_INTERACTIONS} WHERE id = ?"))
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(Interaction::try_from).transpose()
    }

    async fn upsert_interaction(&self, interaction: Interaction) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO crm_interactions
               (id, contact_id, matter_id, kind, summary, channel, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 contact_id = excluded.contact_id,
                 matter_id  = excluded.matter_id,
                 kind       = excluded.kind,
                 summary    = excluded.summary,
                 channel    = excluded.channel,
                 updated_at = excluded.updated_at"#,
        )
        .bind(&interaction.id)
        .bind(&interaction.contact_id)
        .bind(&interaction.matter_id)
        .bind(interaction.kind.as_str())
        .bind(&interaction.summary)
        .bind(&interaction.channel)
        .bind(interaction.created_at as i64)
        .bind(interaction.updated_at as i64)
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

    async fn delete_interactions_before(&self, cutoff_epoch_ms: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM crm_interactions WHERE created_at < ?")
            .bind(cutoff_epoch_ms)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {
        super::*,
        crate::types::{
            Contact, ContactChannel, ContactStage, Interaction, InteractionKind, Matter,
            PracticeArea,
        },
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

    #[tokio::test]
    async fn contact_pii_roundtrips_through_sqlite() {
        use secrecy::ExposeSecret;
        let store = test_store().await;
        let mut c = Contact::new("PiiTest");
        c.email = Some(Secret::new("pii@test.com".to_owned()));
        c.phone = Some(Secret::new("+10000000001".to_owned()));
        let id = c.id.clone();
        store.upsert(c).await.unwrap();
        let found = store.get(&id).await.unwrap().unwrap();
        assert_eq!(
            found.email.as_ref().unwrap().expose_secret(),
            "pii@test.com"
        );
        assert_eq!(
            found.phone.as_ref().unwrap().expose_secret(),
            "+10000000001"
        );
    }

    // ── ContactChannel tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_list_channels() {
        let store = test_store().await;
        let c = Contact::new("Chan");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let ch = ContactChannel::new(&contact_id, "telegram", "tg-1");
        store.upsert_channel(ch.clone()).await.unwrap();
        let channels = store.list_channels_for_contact(&contact_id).await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].channel_type, "telegram");
    }

    #[tokio::test]
    async fn get_channel_by_external_finds_channel() {
        let store = test_store().await;
        let c = Contact::new("Chan2");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let ch = ContactChannel::new(&contact_id, "slack", "U-999");
        store.upsert_channel(ch).await.unwrap();
        let found = store
            .get_channel_by_external("slack", "U-999")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.contact_id, contact_id);
    }

    #[tokio::test]
    async fn delete_channel_is_idempotent() {
        let store = test_store().await;
        let c = Contact::new("ChanDel");
        store.upsert(c.clone()).await.unwrap();
        let ch = ContactChannel::new(&c.id, "whatsapp", "wa-1");
        let ch_id = ch.id.clone();
        store.upsert_channel(ch).await.unwrap();
        store.delete_channel(&ch_id).await.unwrap();
        store.delete_channel(&ch_id).await.unwrap();
        assert!(
            store
                .get_channel_by_external("whatsapp", "wa-1")
                .await
                .unwrap()
                .is_none()
        );
    }

    // ── Matter tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_matter() {
        let store = test_store().await;
        let c = Contact::new("MatterContact");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let m = Matter::new(&contact_id, "Contract Review", PracticeArea::Corporate);
        let mid = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        let found = store.get_matter(&mid).await.unwrap().unwrap();
        assert_eq!(found.title, "Contract Review");
        assert_eq!(found.practice_area, PracticeArea::Corporate);
    }

    #[tokio::test]
    async fn list_matters_by_contact() {
        let store = test_store().await;
        let c = Contact::new("Multi");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        store
            .upsert_matter(Matter::new(&contact_id, "M1", PracticeArea::Tax))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&contact_id, "M2", PracticeArea::Litigation))
            .await
            .unwrap();
        let matters = store.list_matters_by_contact(&contact_id).await.unwrap();
        assert_eq!(matters.len(), 2);
    }

    #[tokio::test]
    async fn delete_matter_is_idempotent() {
        let store = test_store().await;
        let c = Contact::new("MatterDel");
        store.upsert(c.clone()).await.unwrap();
        let m = Matter::new(&c.id, "Delete Me", PracticeArea::Other);
        let mid = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        store.delete_matter(&mid).await.unwrap();
        store.delete_matter(&mid).await.unwrap();
        assert!(store.get_matter(&mid).await.unwrap().is_none());
    }

    // ── Interaction tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_interaction() {
        let store = test_store().await;
        let c = Contact::new("IntContact");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let i = Interaction::new(&contact_id, InteractionKind::Call, "Initial intake call");
        let iid = i.id.clone();
        store.upsert_interaction(i).await.unwrap();
        let found = store.get_interaction(&iid).await.unwrap().unwrap();
        assert_eq!(found.summary, "Initial intake call");
        assert_eq!(found.kind, InteractionKind::Call);
    }

    #[tokio::test]
    async fn list_interactions_by_contact() {
        let store = test_store().await;
        let c = Contact::new("IntMulti");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        store
            .upsert_interaction(Interaction::new(
                &contact_id,
                InteractionKind::Email,
                "Email 1",
            ))
            .await
            .unwrap();
        store
            .upsert_interaction(Interaction::new(
                &contact_id,
                InteractionKind::Note,
                "Note 1",
            ))
            .await
            .unwrap();
        let interactions = store
            .list_interactions_by_contact(&contact_id)
            .await
            .unwrap();
        assert_eq!(interactions.len(), 2);
    }

    #[tokio::test]
    async fn interaction_linked_to_matter() {
        let store = test_store().await;
        let c = Contact::new("IntMatter");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let m = Matter::new(&contact_id, "Linked Matter", PracticeArea::Employment);
        let matter_id = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        let mut i = Interaction::new(&contact_id, InteractionKind::Meeting, "Strategy meeting");
        i.matter_id = Some(matter_id.clone());
        store.upsert_interaction(i).await.unwrap();
        let by_matter = store.list_interactions_by_matter(&matter_id).await.unwrap();
        assert_eq!(by_matter.len(), 1);
        assert_eq!(by_matter[0].summary, "Strategy meeting");
    }

    #[tokio::test]
    async fn delete_interaction_is_idempotent() {
        let store = test_store().await;
        let c = Contact::new("IntDel");
        store.upsert(c.clone()).await.unwrap();
        let i = Interaction::new(&c.id, InteractionKind::Document, "Doc");
        let iid = i.id.clone();
        store.upsert_interaction(i).await.unwrap();
        store.delete_interaction(&iid).await.unwrap();
        store.delete_interaction(&iid).await.unwrap();
        assert!(store.get_interaction(&iid).await.unwrap().is_none());
    }

    // ── list_filtered tests ───────────────────────────────────────────────────

    #[tokio::test]
    async fn list_filtered_no_filters_returns_all() {
        let store = test_store().await;
        store.upsert(Contact::new("A")).await.unwrap();
        store.upsert(Contact::new("B")).await.unwrap();
        let all = store.list_filtered(None, None, 0, 50).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn list_filtered_by_stage() {
        let store = test_store().await;
        let mut active = Contact::new("ActiveOne");
        active.stage = ContactStage::Active;
        let lead = Contact::new("LeadOne"); // default stage = Lead
        store.upsert(active).await.unwrap();
        store.upsert(lead).await.unwrap();
        let results = store
            .list_filtered(Some(ContactStage::Active), None, 0, 50)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "ActiveOne");
    }

    #[tokio::test]
    async fn list_filtered_by_name_search() {
        let store = test_store().await;
        store.upsert(Contact::new("Alice Smith")).await.unwrap();
        store.upsert(Contact::new("Bob Jones")).await.unwrap();
        let results = store
            .list_filtered(None, Some("alice"), 0, 50)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice Smith");
    }

    #[tokio::test]
    async fn list_filtered_by_email_search() {
        use secrecy::Secret;
        let store = test_store().await;
        let mut c = Contact::new("EmailUser");
        c.email = Some(Secret::new("hello@example.com".to_owned()));
        store.upsert(c).await.unwrap();
        store.upsert(Contact::new("NoEmail")).await.unwrap();
        let results = store
            .list_filtered(None, Some("example.com"), 0, 50)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "EmailUser");
    }

    #[tokio::test]
    async fn list_filtered_pagination() {
        let store = test_store().await;
        for i in 0..5 {
            store
                .upsert(Contact::new(format!("Contact{i}")))
                .await
                .unwrap();
        }
        let page1 = store.list_filtered(None, None, 0, 2).await.unwrap();
        let page2 = store.list_filtered(None, None, 2, 2).await.unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        // Pages should not overlap.
        let ids1: std::collections::HashSet<_> = page1.iter().map(|c| &c.id).collect();
        let ids2: std::collections::HashSet<_> = page2.iter().map(|c| &c.id).collect();
        assert!(ids1.is_disjoint(&ids2));
    }

    #[tokio::test]
    async fn list_filtered_combined_stage_and_search() {
        use secrecy::Secret;
        let store = test_store().await;
        let mut c1 = Contact::new("Alice Active");
        c1.stage = ContactStage::Active;
        c1.email = Some(Secret::new("alice@active.com".to_owned()));
        let mut c2 = Contact::new("Alice Lead");
        c2.email = Some(Secret::new("alice@lead.com".to_owned()));
        store.upsert(c1).await.unwrap();
        store.upsert(c2).await.unwrap();
        let results = store
            .list_filtered(Some(ContactStage::Active), Some("alice"), 0, 50)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Alice Active");
    }

    // ── get_with_channels tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn get_with_channels_returns_contact_and_channels() {
        let store = test_store().await;
        let c = Contact::new("ChanUser");
        let cid = c.id.clone();
        store.upsert(c).await.unwrap();
        store
            .upsert_channel(ContactChannel::new(&cid, "telegram", "tg-42"))
            .await
            .unwrap();
        let result = store.get_with_channels(&cid).await.unwrap().unwrap();
        assert_eq!(result.contact.name, "ChanUser");
        assert_eq!(result.channels.len(), 1);
        assert_eq!(result.channels[0].channel_type, "telegram");
    }

    #[tokio::test]
    async fn get_with_channels_returns_none_for_missing_contact() {
        let store = test_store().await;
        assert!(
            store
                .get_with_channels("no-such-id")
                .await
                .unwrap()
                .is_none()
        );
    }

    // ── list_matters_filtered tests ───────────────────────────────────────────

    #[tokio::test]
    async fn list_matters_filtered_by_contact_id() {
        let store = test_store().await;
        let c1 = Contact::new("C1");
        let c2 = Contact::new("C2");
        store.upsert(c1.clone()).await.unwrap();
        store.upsert(c2.clone()).await.unwrap();
        store
            .upsert_matter(Matter::new(&c1.id, "M1", PracticeArea::Corporate))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c2.id, "M2", PracticeArea::Tax))
            .await
            .unwrap();
        let results = store
            .list_matters_filtered(Some(&c1.id), None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "M1");
    }

    #[tokio::test]
    async fn list_matters_filtered_by_practice_area() {
        let store = test_store().await;
        let c = Contact::new("PracticeC");
        store.upsert(c.clone()).await.unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "Corporate", PracticeArea::Corporate))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "Tax", PracticeArea::Tax))
            .await
            .unwrap();
        let results = store
            .list_matters_filtered(None, Some(PracticeArea::Tax))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Tax");
    }

    #[tokio::test]
    async fn list_matters_filtered_combined() {
        let store = test_store().await;
        let c1 = Contact::new("F1");
        let c2 = Contact::new("F2");
        store.upsert(c1.clone()).await.unwrap();
        store.upsert(c2.clone()).await.unwrap();
        store
            .upsert_matter(Matter::new(&c1.id, "C1-Corp", PracticeArea::Corporate))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c1.id, "C1-Tax", PracticeArea::Tax))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c2.id, "C2-Corp", PracticeArea::Corporate))
            .await
            .unwrap();
        // Both contact_id and practice_area filter.
        let results = store
            .list_matters_filtered(Some(&c1.id), Some(PracticeArea::Corporate))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "C1-Corp");
    }
}
