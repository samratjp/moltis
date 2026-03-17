use serde::{Deserialize, Serialize};

/// A contact in the CRM system.
///
/// Contacts represent people or entities that interact with the system through
/// any channel (Telegram, WhatsApp, Slack, etc.) or are imported from external
/// sources. The `source` field records which channel or import path created
/// the contact; a typed channel reference can be added in a later phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contact {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Origin channel or import source (e.g. `"telegram"`, `"whatsapp"`, `"csv-import"`).
    #[serde(default)]
    pub source: Option<String>,
    /// Identifier assigned by the external channel (e.g. Telegram user ID).
    #[serde(default)]
    pub external_id: Option<String>,
    /// Arbitrary structured metadata stored as a JSON object.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Unix timestamp (milliseconds) when the contact was created.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) when the contact was last updated.
    pub updated_at: u64,
}

impl Contact {
    /// Create a new contact with the given name and auto-generated UUID.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = now_ms();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            source: None,
            external_id: None,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new contact with explicit source and external ID.
    #[must_use]
    pub fn with_source(
        name: impl Into<String>,
        source: impl Into<String>,
        external_id: impl Into<String>,
    ) -> Self {
        let mut c = Self::new(name);
        c.source = Some(source.into());
        c.external_id = Some(external_id.into());
        c
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn new_contact_has_uuid() {
        let c = Contact::new("Alice");
        assert!(!c.id.is_empty());
        assert_eq!(c.name, "Alice");
        assert!(c.source.is_none());
        assert!(c.external_id.is_none());
        assert!(c.created_at > 0);
        assert_eq!(c.created_at, c.updated_at);
    }

    #[test]
    fn with_source_sets_fields() {
        let c = Contact::with_source("Bob", "telegram", "12345");
        assert_eq!(c.source.as_deref(), Some("telegram"));
        assert_eq!(c.external_id.as_deref(), Some("12345"));
    }

    #[test]
    fn new_contacts_have_distinct_ids() {
        let a = Contact::new("A");
        let b = Contact::new("B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn roundtrip_serde() {
        let c = Contact::with_source("Carol", "slack", "U123");
        let json = serde_json::to_string(&c).unwrap();
        let c2: Contact = serde_json::from_str(&json).unwrap();
        assert_eq!(c, c2);
    }
}
