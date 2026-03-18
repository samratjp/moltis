//! Live CRM service backed by the SQLite store.
//!
//! [`LiveCrmService`] implements [`moltis_service_traits::CrmService`] by
//! delegating to [`moltis_crm::SqliteCrmStore`].  Each RPC method deserialises
//! its `serde_json::Value` params, calls the appropriate store operation, and
//! serialises the result back.
//!
//! PII fields (email, phone) are exposed in responses — callers reach this
//! service only through the auth-gated WebSocket layer.

use std::sync::Arc;

use {
    async_trait::async_trait,
    moltis_crm::{
        Contact, ContactChannel, Interaction, Matter, SqliteCrmStore,
        store::CrmStore,
        types::{ContactStage, InteractionKind, MatterPhase, MatterStatus, PracticeArea},
    },
    moltis_service_traits::{CrmService, ServiceError, ServiceResult},
    secrecy::{ExposeSecret, Secret},
    serde_json::Value,
};

/// Live CRM service backed by a SQLite store.
pub struct LiveCrmService {
    store: Arc<SqliteCrmStore>,
}

impl LiveCrmService {
    #[must_use]
    pub fn new(store: Arc<SqliteCrmStore>) -> Self {
        Self { store }
    }
}

// ── Serialisation helpers ─────────────────────────────────────────────────────

fn contact_to_json(c: Contact) -> Value {
    serde_json::json!({
        "id":         c.id,
        "name":       c.name,
        "source":     c.source,
        "externalId": c.external_id,
        "email":      c.email.as_ref().map(|s| s.expose_secret()),
        "phone":      c.phone.as_ref().map(|s| s.expose_secret()),
        "stage":      c.stage.as_str(),
        "metadata":   c.metadata,
        "createdAt":  c.created_at,
        "updatedAt":  c.updated_at,
    })
}

fn channel_to_json(ch: ContactChannel) -> Value {
    serde_json::json!({
        "id":          ch.id,
        "contactId":   ch.contact_id,
        "channelType": ch.channel_type,
        "channelId":   ch.channel_id,
        "displayName": ch.display_name,
        "verified":    ch.verified,
        "createdAt":   ch.created_at,
        "updatedAt":   ch.updated_at,
    })
}

fn matter_to_json(m: Matter) -> Value {
    serde_json::json!({
        "id":           m.id,
        "contactId":    m.contact_id,
        "title":        m.title,
        "description":  m.description,
        "status":       m.status.as_str(),
        "phase":        m.phase.as_str(),
        "practiceArea": m.practice_area.as_str(),
        "createdAt":    m.created_at,
        "updatedAt":    m.updated_at,
    })
}

fn interaction_to_json(i: Interaction) -> Value {
    serde_json::json!({
        "id":        i.id,
        "contactId": i.contact_id,
        "matterId":  i.matter_id,
        "kind":      i.kind.as_str(),
        "summary":   i.summary,
        "channel":   i.channel,
        "createdAt": i.created_at,
        "updatedAt": i.updated_at,
    })
}

fn store_err(e: moltis_crm::Error) -> ServiceError {
    ServiceError::message(e.to_string())
}

// ── CrmService impl ───────────────────────────────────────────────────────────

#[async_trait]
impl CrmService for LiveCrmService {
    // ── Contacts ──────────────────────────────────────────────────────────────

    async fn list_contacts(&self, params: Value) -> ServiceResult {
        // If any filter params are present, use list_filtered; otherwise list all.
        let stage = params
            .get("stage")
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<ContactStage>().map_err(ServiceError::message))
            .transpose()?;
        let search = params.get("search").and_then(|v| v.as_str());
        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(0);

        let contacts = if stage.is_some() || search.is_some() || offset > 0 || limit > 0 {
            let effective_limit = if limit == 0 {
                50
            } else {
                limit
            };
            self.store
                .list_filtered(stage, search, offset, effective_limit)
                .await
                .map_err(store_err)?
        } else {
            self.store.list().await.map_err(store_err)?
        };
        Ok(Value::Array(
            contacts.into_iter().map(contact_to_json).collect(),
        ))
    }

    async fn get_contact(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        let contact = self.store.get(id).await.map_err(store_err)?;
        Ok(contact.map(contact_to_json).unwrap_or(Value::Null))
    }

    async fn get_contact_by_external(&self, params: Value) -> ServiceResult {
        let source = require_str(&params, "source")?;
        let external_id = require_str(&params, "externalId")?;
        let contact = self
            .store
            .get_by_external(source, external_id)
            .await
            .map_err(store_err)?;
        Ok(contact.map(contact_to_json).unwrap_or(Value::Null))
    }

    async fn get_contact_with_channels(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        let result = self.store.get_with_channels(id).await.map_err(store_err)?;
        match result {
            None => Ok(Value::Null),
            Some(cwc) => Ok(serde_json::json!({
                "contact":  contact_to_json(cwc.contact),
                "channels": cwc.channels.into_iter().map(channel_to_json).collect::<Vec<_>>(),
            })),
        }
    }

    async fn upsert_contact(&self, params: Value) -> ServiceResult {
        let contact = parse_contact(params)?;
        self.store.upsert(contact).await.map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn delete_contact(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        self.store.delete(id).await.map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    // ── Contact channels ──────────────────────────────────────────────────────

    async fn list_channels(&self, params: Value) -> ServiceResult {
        let contact_id = require_str(&params, "contactId")?;
        let channels = self
            .store
            .list_channels_for_contact(contact_id)
            .await
            .map_err(store_err)?;
        Ok(Value::Array(
            channels.into_iter().map(channel_to_json).collect(),
        ))
    }

    async fn upsert_channel(&self, params: Value) -> ServiceResult {
        let channel = parse_channel(params)?;
        self.store
            .upsert_channel(channel)
            .await
            .map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn delete_channel(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        self.store.delete_channel(id).await.map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    // ── Matters ───────────────────────────────────────────────────────────────

    async fn list_matters(&self, params: Value) -> ServiceResult {
        let has_filters = params.is_object() && params.as_object().is_some_and(|o| !o.is_empty());
        let matters = if has_filters {
            let contact_id = params
                .get("contactId")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
            let status = params
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<MatterStatus>().map_err(ServiceError::message))
                .transpose()?;
            let phase = params
                .get("phase")
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<MatterPhase>().map_err(ServiceError::message))
                .transpose()?;
            let practice_area = params
                .get("practiceArea")
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<PracticeArea>().map_err(ServiceError::message))
                .transpose()?;
            let search = params
                .get("search")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);
            self.store
                .list_matters_filtered(
                    contact_id.as_deref(),
                    status,
                    phase,
                    practice_area,
                    search.as_deref(),
                    offset,
                    limit,
                )
                .await
                .map_err(store_err)?
        } else {
            self.store.list_matters().await.map_err(store_err)?
        };
        Ok(Value::Array(
            matters.into_iter().map(matter_to_json).collect(),
        ))
    }

    async fn get_matter(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        let matter = self.store.get_matter(id).await.map_err(store_err)?;
        Ok(matter.map(matter_to_json).unwrap_or(Value::Null))
    }

    async fn upsert_matter(&self, params: Value) -> ServiceResult {
        let matter = parse_matter(params)?;
        self.store.upsert_matter(matter).await.map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn delete_matter(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        self.store.delete_matter(id).await.map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    // ── Interactions ──────────────────────────────────────────────────────────

    async fn list_interactions(&self, params: Value) -> ServiceResult {
        if let Some(contact_id) = params.get("contactId").and_then(|v| v.as_str()) {
            let items = self
                .store
                .list_interactions_by_contact(contact_id)
                .await
                .map_err(store_err)?;
            return Ok(Value::Array(
                items.into_iter().map(interaction_to_json).collect(),
            ));
        }
        if let Some(matter_id) = params.get("matterId").and_then(|v| v.as_str()) {
            let items = self
                .store
                .list_interactions_by_matter(matter_id)
                .await
                .map_err(store_err)?;
            return Ok(Value::Array(
                items.into_iter().map(interaction_to_json).collect(),
            ));
        }
        Err(ServiceError::message(
            "list_interactions requires contactId or matterId",
        ))
    }

    async fn get_interaction(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        let item = self.store.get_interaction(id).await.map_err(store_err)?;
        Ok(item.map(interaction_to_json).unwrap_or(Value::Null))
    }

    async fn upsert_interaction(&self, params: Value) -> ServiceResult {
        let interaction = parse_interaction(params)?;
        self.store
            .upsert_interaction(interaction)
            .await
            .map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }

    async fn delete_interaction(&self, params: Value) -> ServiceResult {
        let id = require_str(&params, "id")?;
        self.store.delete_interaction(id).await.map_err(store_err)?;
        Ok(serde_json::json!({ "ok": true }))
    }
}

// ── Parse helpers ─────────────────────────────────────────────────────────────

fn require_str<'a>(params: &'a Value, field: &'static str) -> Result<&'a str, ServiceError> {
    params
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServiceError::message(format!("missing field: {field}")))
}

fn parse_contact(p: Value) -> Result<Contact, ServiceError> {
    let id = require_str(&p, "id")?.to_owned();
    let name = require_str(&p, "name")?.to_owned();
    let stage = p
        .get("stage")
        .and_then(|v| v.as_str())
        .unwrap_or("lead")
        .parse::<ContactStage>()
        .map_err(ServiceError::message)?;
    let now = now_ms();
    Ok(Contact {
        id,
        name,
        source: p
            .get("source")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        external_id: p
            .get("externalId")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        email: p
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| Secret::new(s.to_owned())),
        phone: p
            .get("phone")
            .and_then(|v| v.as_str())
            .map(|s| Secret::new(s.to_owned())),
        stage,
        metadata: p
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
        created_at: p.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(now),
        updated_at: now,
    })
}

fn parse_channel(p: Value) -> Result<ContactChannel, ServiceError> {
    let id = require_str(&p, "id")?.to_owned();
    let contact_id = require_str(&p, "contactId")?.to_owned();
    let channel_type = require_str(&p, "channelType")?.to_owned();
    let channel_id = require_str(&p, "channelId")?.to_owned();
    let now = now_ms();
    Ok(ContactChannel {
        id,
        contact_id,
        channel_type,
        channel_id,
        display_name: p
            .get("displayName")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        verified: p.get("verified").and_then(|v| v.as_bool()).unwrap_or(false),
        created_at: p.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(now),
        updated_at: now,
    })
}

fn parse_matter(p: Value) -> Result<Matter, ServiceError> {
    let id = require_str(&p, "id")?.to_owned();
    let title = require_str(&p, "title")?.to_owned();
    let status = p
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("open")
        .parse::<MatterStatus>()
        .map_err(ServiceError::message)?;
    let phase = p
        .get("phase")
        .and_then(|v| v.as_str())
        .unwrap_or("intake")
        .parse::<MatterPhase>()
        .map_err(ServiceError::message)?;
    let practice_area = p
        .get("practiceArea")
        .and_then(|v| v.as_str())
        .unwrap_or("other")
        .parse::<PracticeArea>()
        .map_err(ServiceError::message)?;
    let now = now_ms();
    Ok(Matter {
        id,
        contact_id: p
            .get("contactId")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        title,
        description: p
            .get("description")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        status,
        phase,
        practice_area,
        created_at: p.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(now),
        updated_at: now,
    })
}

fn parse_interaction(p: Value) -> Result<Interaction, ServiceError> {
    let id = require_str(&p, "id")?.to_owned();
    let contact_id = require_str(&p, "contactId")?.to_owned();
    let kind = require_str(&p, "kind")?
        .parse::<InteractionKind>()
        .map_err(ServiceError::message)?;
    let summary = require_str(&p, "summary")?.to_owned();
    let now = now_ms();
    Ok(Interaction {
        id,
        contact_id,
        matter_id: p
            .get("matterId")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        kind,
        summary,
        channel: p
            .get("channel")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        created_at: p.get("createdAt").and_then(|v| v.as_u64()).unwrap_or(now),
        updated_at: now,
    })
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {super::*, sqlx::SqlitePool};

    async fn make_service() -> LiveCrmService {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        moltis_crm::run_migrations(&pool).await.unwrap();
        LiveCrmService::new(Arc::new(SqliteCrmStore::new(pool)))
    }

    // ── Contact CRUD ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_contacts_empty() {
        let svc = make_service().await;
        let result = svc.list_contacts(serde_json::json!({})).await.unwrap();
        assert_eq!(result, serde_json::json!([]));
    }

    #[tokio::test]
    async fn upsert_and_get_contact() {
        let svc = make_service().await;
        let id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": id,
            "name": "Alice",
            "stage": "lead",
        }))
        .await
        .unwrap();
        let result = svc
            .get_contact(serde_json::json!({ "id": id }))
            .await
            .unwrap();
        assert_eq!(result["name"], "Alice");
        assert_eq!(result["stage"], "lead");
    }

    #[tokio::test]
    async fn list_contacts_returns_upserted() {
        let svc = make_service().await;
        let id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({ "id": id, "name": "Bob", "stage": "prospect" }))
            .await
            .unwrap();
        let result = svc.list_contacts(serde_json::json!({})).await.unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Bob");
    }

    #[tokio::test]
    async fn delete_contact_removes_it() {
        let svc = make_service().await;
        let id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({ "id": id, "name": "Carol", "stage": "active" }))
            .await
            .unwrap();
        svc.delete_contact(serde_json::json!({ "id": id }))
            .await
            .unwrap();
        let result = svc
            .get_contact(serde_json::json!({ "id": id }))
            .await
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[tokio::test]
    async fn get_contact_missing_id_errors() {
        let svc = make_service().await;
        assert!(svc.get_contact(serde_json::json!({})).await.is_err());
    }

    // ── Channels ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_list_channels() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "Chan",
            "stage": "lead",
        }))
        .await
        .unwrap();
        let ch_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_channel(serde_json::json!({
            "id": ch_id,
            "contactId": contact_id,
            "channelType": "telegram",
            "channelId": "tg-999",
        }))
        .await
        .unwrap();
        let result = svc
            .list_channels(serde_json::json!({ "contactId": contact_id }))
            .await
            .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["channelType"], "telegram");
    }

    #[tokio::test]
    async fn delete_channel_removes_it() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "Del",
            "stage": "lead",
        }))
        .await
        .unwrap();
        let ch_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_channel(serde_json::json!({
            "id": ch_id,
            "contactId": contact_id,
            "channelType": "slack",
            "channelId": "U-1",
        }))
        .await
        .unwrap();
        svc.delete_channel(serde_json::json!({ "id": ch_id }))
            .await
            .unwrap();
        let result = svc
            .list_channels(serde_json::json!({ "contactId": contact_id }))
            .await
            .unwrap();
        assert_eq!(result.as_array().unwrap().len(), 0);
    }

    // ── Matters ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_matter() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "MatterTest",
            "stage": "active",
        }))
        .await
        .unwrap();
        let mid = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": mid,
            "contactId": contact_id,
            "title": "Contract Review",
            "status": "open",
            "phase": "intake",
            "practiceArea": "corporate",
        }))
        .await
        .unwrap();
        let result = svc
            .get_matter(serde_json::json!({ "id": mid }))
            .await
            .unwrap();
        assert_eq!(result["title"], "Contract Review");
        assert_eq!(result["practiceArea"], "corporate");
    }

    #[tokio::test]
    async fn delete_matter_removes_it() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "DelMatter",
            "stage": "active",
        }))
        .await
        .unwrap();
        let mid = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": mid,
            "contactId": contact_id,
            "title": "To Delete",
            "status": "open",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        svc.delete_matter(serde_json::json!({ "id": mid }))
            .await
            .unwrap();
        let result = svc
            .get_matter(serde_json::json!({ "id": mid }))
            .await
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[tokio::test]
    async fn list_matters_empty_returns_empty_array() {
        let svc = make_service().await;
        let result = svc.list_matters(serde_json::json!(null)).await.unwrap();
        assert_eq!(result, serde_json::json!([]));
    }

    #[tokio::test]
    async fn list_matters_no_params_returns_all() {
        let svc = make_service().await;
        let mid = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": mid,
            "title": "All Matters Test",
            "status": "open",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        let result = svc.list_matters(serde_json::json!(null)).await.unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_matters_filtered_by_status() {
        let svc = make_service().await;
        let open_id = uuid::Uuid::new_v4().to_string();
        let closed_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": open_id,
            "title": "Open Matter",
            "status": "open",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        svc.upsert_matter(serde_json::json!({
            "id": closed_id,
            "title": "Closed Matter",
            "status": "closed",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        let result = svc
            .list_matters(serde_json::json!({ "status": "open" }))
            .await
            .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["status"], "open");
    }

    #[tokio::test]
    async fn list_matters_filtered_by_search() {
        let svc = make_service().await;
        let match_id = uuid::Uuid::new_v4().to_string();
        let nomatch_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": match_id,
            "title": "Patent Dispute",
            "status": "open",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        svc.upsert_matter(serde_json::json!({
            "id": nomatch_id,
            "title": "Lease Agreement",
            "status": "open",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        let result = svc
            .list_matters(serde_json::json!({ "search": "patent" }))
            .await
            .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "Patent Dispute");
    }

    #[tokio::test]
    async fn list_matters_filtered_pagination() {
        let svc = make_service().await;
        for i in 0..5_u32 {
            let mid = uuid::Uuid::new_v4().to_string();
            svc.upsert_matter(serde_json::json!({
                "id": mid,
                "title": format!("Matter {i}"),
                "status": "open",
                "phase": "intake",
                "practiceArea": "other",
            }))
            .await
            .unwrap();
        }
        let result = svc
            .list_matters(serde_json::json!({ "offset": 0, "limit": 2 }))
            .await
            .unwrap();
        assert_eq!(result.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn list_matters_invalid_status_returns_error() {
        let svc = make_service().await;
        let result = svc
            .list_matters(serde_json::json!({ "status": "not_a_real_status" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_matters_empty_search_ignored() {
        let svc = make_service().await;
        let mid = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": mid,
            "title": "Visible Matter",
            "status": "open",
            "phase": "intake",
            "practiceArea": "other",
        }))
        .await
        .unwrap();
        // Empty search string should be treated as no filter
        let result = svc
            .list_matters(serde_json::json!({ "search": "" }))
            .await
            .unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1);
    }

    // ── Interactions ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_interaction() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "IntTest",
            "stage": "active",
        }))
        .await
        .unwrap();
        let iid = uuid::Uuid::new_v4().to_string();
        svc.upsert_interaction(serde_json::json!({
            "id": iid,
            "contactId": contact_id,
            "kind": "call",
            "summary": "Intake call",
        }))
        .await
        .unwrap();
        let result = svc
            .get_interaction(serde_json::json!({ "id": iid }))
            .await
            .unwrap();
        assert_eq!(result["summary"], "Intake call");
        assert_eq!(result["kind"], "call");
    }

    #[tokio::test]
    async fn list_interactions_by_contact() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "IntList",
            "stage": "active",
        }))
        .await
        .unwrap();
        for kind in &["call", "email"] {
            let iid = uuid::Uuid::new_v4().to_string();
            svc.upsert_interaction(serde_json::json!({
                "id": iid,
                "contactId": contact_id,
                "kind": kind,
                "summary": format!("{} interaction", kind),
            }))
            .await
            .unwrap();
        }
        let result = svc
            .list_interactions(serde_json::json!({ "contactId": contact_id }))
            .await
            .unwrap();
        assert_eq!(result.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn list_interactions_requires_contact_or_matter_id() {
        let svc = make_service().await;
        assert!(svc.list_interactions(serde_json::json!({})).await.is_err());
    }

    #[tokio::test]
    async fn delete_interaction_removes_it() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "IntDel",
            "stage": "lead",
        }))
        .await
        .unwrap();
        let iid = uuid::Uuid::new_v4().to_string();
        svc.upsert_interaction(serde_json::json!({
            "id": iid,
            "contactId": contact_id,
            "kind": "note",
            "summary": "A note",
        }))
        .await
        .unwrap();
        svc.delete_interaction(serde_json::json!({ "id": iid }))
            .await
            .unwrap();
        let result = svc
            .get_interaction(serde_json::json!({ "id": iid }))
            .await
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    // ── list_contacts filter/pagination ───────────────────────────────────────

    #[tokio::test]
    async fn list_contacts_no_params_backward_compat() {
        let svc = make_service().await;
        let id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({ "id": id, "name": "Filter", "stage": "lead" }))
            .await
            .unwrap();
        // Empty params → returns all
        let result = svc.list_contacts(serde_json::json!({})).await.unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_contacts_filter_by_stage() {
        let svc = make_service().await;
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({ "id": id1, "name": "Lead One", "stage": "lead" }))
            .await
            .unwrap();
        svc.upsert_contact(
            serde_json::json!({ "id": id2, "name": "Active One", "stage": "active" }),
        )
        .await
        .unwrap();
        let result = svc
            .list_contacts(serde_json::json!({ "stage": "lead" }))
            .await
            .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Lead One");
    }

    #[tokio::test]
    async fn list_contacts_filter_by_search() {
        let svc = make_service().await;
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(
            serde_json::json!({ "id": id1, "name": "Alice Smith", "stage": "lead" }),
        )
        .await
        .unwrap();
        svc.upsert_contact(serde_json::json!({ "id": id2, "name": "Bob Jones", "stage": "lead" }))
            .await
            .unwrap();
        let result = svc
            .list_contacts(serde_json::json!({ "search": "alice" }))
            .await
            .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "Alice Smith");
    }

    #[tokio::test]
    async fn list_contacts_pagination() {
        let svc = make_service().await;
        for i in 0..5u32 {
            let id = uuid::Uuid::new_v4().to_string();
            svc.upsert_contact(
                serde_json::json!({ "id": id, "name": format!("Contact {i}"), "stage": "lead" }),
            )
            .await
            .unwrap();
        }
        let page1 = svc
            .list_contacts(serde_json::json!({ "offset": 0, "limit": 2 }))
            .await
            .unwrap();
        assert_eq!(page1.as_array().unwrap().len(), 2);

        let page2 = svc
            .list_contacts(serde_json::json!({ "offset": 2, "limit": 2 }))
            .await
            .unwrap();
        assert_eq!(page2.as_array().unwrap().len(), 2);

        // Pages must be disjoint
        let p1_ids: Vec<&str> = page1
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["id"].as_str().unwrap())
            .collect();
        let p2_ids: Vec<&str> = page2
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["id"].as_str().unwrap())
            .collect();
        assert!(p1_ids.iter().all(|id| !p2_ids.contains(id)));
    }

    #[tokio::test]
    async fn list_contacts_invalid_stage_errors() {
        let svc = make_service().await;
        let result = svc
            .list_contacts(serde_json::json!({ "stage": "not_a_valid_stage" }))
            .await;
        assert!(result.is_err());
    }

    // ── get_contact_by_external ───────────────────────────────────────────────

    #[tokio::test]
    async fn get_contact_by_external_found() {
        let svc = make_service().await;
        let id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": id,
            "name": "Ext User",
            "stage": "lead",
            "source": "telegram",
            "externalId": "tg-42",
        }))
        .await
        .unwrap();
        let result = svc
            .get_contact_by_external(serde_json::json!({
                "source": "telegram",
                "externalId": "tg-42",
            }))
            .await
            .unwrap();
        assert_eq!(result["name"], "Ext User");
        assert_eq!(result["externalId"], "tg-42");
    }

    #[tokio::test]
    async fn get_contact_by_external_not_found() {
        let svc = make_service().await;
        let result = svc
            .get_contact_by_external(serde_json::json!({
                "source": "telegram",
                "externalId": "no-such-id",
            }))
            .await
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[tokio::test]
    async fn get_contact_by_external_missing_params() {
        let svc = make_service().await;
        // missing externalId
        assert!(
            svc.get_contact_by_external(serde_json::json!({ "source": "telegram" }))
                .await
                .is_err()
        );
        // missing source
        assert!(
            svc.get_contact_by_external(serde_json::json!({ "externalId": "tg-1" }))
                .await
                .is_err()
        );
    }

    // ── get_contact_with_channels ─────────────────────────────────────────────

    #[tokio::test]
    async fn get_contact_with_channels_returns_contact_and_channels() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "With Channels",
            "stage": "active",
        }))
        .await
        .unwrap();
        let ch_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_channel(serde_json::json!({
            "id": ch_id,
            "contactId": contact_id,
            "channelType": "telegram",
            "channelId": "tg-100",
        }))
        .await
        .unwrap();

        let result = svc
            .get_contact_with_channels(serde_json::json!({ "id": contact_id }))
            .await
            .unwrap();
        assert_eq!(result["contact"]["name"], "With Channels");
        let channels = result["channels"].as_array().unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0]["channelType"], "telegram");
    }

    #[tokio::test]
    async fn get_contact_with_channels_no_channels() {
        let svc = make_service().await;
        let id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({ "id": id, "name": "No Channels", "stage": "lead" }))
            .await
            .unwrap();
        let result = svc
            .get_contact_with_channels(serde_json::json!({ "id": id }))
            .await
            .unwrap();
        assert_eq!(result["contact"]["name"], "No Channels");
        assert_eq!(result["channels"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn get_contact_with_channels_not_found() {
        let svc = make_service().await;
        let result = svc
            .get_contact_with_channels(serde_json::json!({ "id": "nonexistent-id" }))
            .await
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[tokio::test]
    async fn get_contact_with_channels_missing_id() {
        let svc = make_service().await;
        assert!(
            svc.get_contact_with_channels(serde_json::json!({}))
                .await
                .is_err()
        );
    }

    // ── Interaction tests (CHA-18) ───────────────────────────────────────────

    #[tokio::test]
    async fn list_interactions_by_matter() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "IntMatter",
            "stage": "active",
        }))
        .await
        .unwrap();
        let matter_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_matter(serde_json::json!({
            "id": matter_id,
            "contactId": contact_id,
            "title": "Contract",
            "status": "open",
            "phase": "intake",
            "practiceArea": "corporate",
        }))
        .await
        .unwrap();
        // One interaction linked to the matter, one without.
        let iid_linked = uuid::Uuid::new_v4().to_string();
        svc.upsert_interaction(serde_json::json!({
            "id": iid_linked,
            "contactId": contact_id,
            "matterId": matter_id,
            "kind": "meeting",
            "summary": "Kickoff meeting",
        }))
        .await
        .unwrap();
        let iid_other = uuid::Uuid::new_v4().to_string();
        svc.upsert_interaction(serde_json::json!({
            "id": iid_other,
            "contactId": contact_id,
            "kind": "note",
            "summary": "Unrelated note",
        }))
        .await
        .unwrap();
        let result = svc
            .list_interactions(serde_json::json!({ "matterId": matter_id }))
            .await
            .unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], iid_linked);
        assert_eq!(arr[0]["matterId"], matter_id);
    }

    #[tokio::test]
    async fn upsert_interaction_overwrites_existing() {
        let svc = make_service().await;
        let contact_id = uuid::Uuid::new_v4().to_string();
        svc.upsert_contact(serde_json::json!({
            "id": contact_id,
            "name": "IntOverwrite",
            "stage": "active",
        }))
        .await
        .unwrap();
        let iid = uuid::Uuid::new_v4().to_string();
        svc.upsert_interaction(serde_json::json!({
            "id": iid,
            "contactId": contact_id,
            "kind": "call",
            "summary": "Original summary",
        }))
        .await
        .unwrap();
        // Upsert same ID with updated summary and kind.
        svc.upsert_interaction(serde_json::json!({
            "id": iid,
            "contactId": contact_id,
            "kind": "email",
            "summary": "Updated summary",
        }))
        .await
        .unwrap();
        let result = svc
            .get_interaction(serde_json::json!({ "id": iid }))
            .await
            .unwrap();
        assert_eq!(result["summary"], "Updated summary");
        assert_eq!(result["kind"], "email");
    }
}
