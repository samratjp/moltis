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

// ── Matter ────────────────────────────────────────────────────────────────────

/// A matter in the CRM system.
///
/// Matters represent cases, deals, or topics being tracked. A matter may be
/// optionally linked to a contact; matters that span multiple contacts can
/// leave `contact_id` as `None`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Matter {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Lifecycle status of the matter.
    pub status: MatterStatus,
    /// Optional contact this matter is primarily associated with.
    #[serde(default)]
    pub contact_id: Option<String>,
    /// Arbitrary structured metadata stored as a JSON object.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Unix timestamp (milliseconds) when the matter was created.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) when the matter was last updated.
    pub updated_at: u64,
}

impl Matter {
    /// Create a new open matter with the given title and auto-generated UUID.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        let now = now_ms();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            status: MatterStatus::Open,
            contact_id: None,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new matter linked to a contact.
    #[must_use]
    pub fn for_contact(title: impl Into<String>, contact_id: impl Into<String>) -> Self {
        let mut m = Self::new(title);
        m.contact_id = Some(contact_id.into());
        m
    }
}

/// Lifecycle status of a [`Matter`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatterStatus {
    /// Newly created; not yet being actively worked.
    #[default]
    Open,
    /// Currently being worked.
    InProgress,
    /// Resolved or no longer active.
    Closed,
}

impl MatterStatus {
    /// String representation stored in the database.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Closed => "closed",
        }
    }
}

impl TryFrom<&str> for MatterStatus {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "open" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "closed" => Ok(Self::Closed),
            other => Err(crate::Error::invalid_field("status", other)),
        }
    }
}

// ── Interaction ───────────────────────────────────────────────────────────────

/// A logged interaction with a contact.
///
/// Interactions record communications or activities: a message received, a call
/// made, a meeting held, or a note added. Each interaction is tied to a contact
/// and may optionally be linked to a [`Matter`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Contact this interaction is with.
    pub contact_id: String,
    /// Optional matter this interaction belongs to.
    #[serde(default)]
    pub matter_id: Option<String>,
    /// The type of interaction.
    pub kind: InteractionKind,
    /// Free-text body (e.g. message content, call summary, note).
    #[serde(default)]
    pub body: Option<String>,
    /// Arbitrary structured metadata stored as a JSON object.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Unix timestamp (milliseconds) when the interaction occurred / was recorded.
    pub created_at: u64,
}

impl Interaction {
    /// Create a new interaction with auto-generated UUID.
    #[must_use]
    pub fn new(
        contact_id: impl Into<String>,
        kind: InteractionKind,
        body: impl Into<Option<String>>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            contact_id: contact_id.into(),
            matter_id: None,
            kind,
            body: body.into(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            created_at: now_ms(),
        }
    }
}

/// The type of a logged [`Interaction`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    /// An inbound or outbound message (chat, email, SMS).
    Message,
    /// A phone or video call.
    Call,
    /// An in-person or virtual meeting.
    Meeting,
    /// A manually written note.
    Note,
}

impl InteractionKind {
    /// String representation stored in the database.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Call => "call",
            Self::Meeting => "meeting",
            Self::Note => "note",
        }
    }
}

impl TryFrom<&str> for InteractionKind {
    type Error = crate::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "message" => Ok(Self::Message),
            "call" => Ok(Self::Call),
            "meeting" => Ok(Self::Meeting),
            "note" => Ok(Self::Note),
            other => Err(crate::Error::invalid_field("kind", other)),
        }
    }
}

// ── ContactChannel ────────────────────────────────────────────────────────────

/// A communication channel associated with a contact.
///
/// Each channel record maps a (`channel_type`, `channel_identifier`) pair — e.g.
/// `("telegram", "123456789")` — to a single contact. The unique constraint on
/// the pair is enforced at the database level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactChannel {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Contact this channel belongs to.
    pub contact_id: String,
    /// Channel type key (e.g. `"telegram"`, `"whatsapp"`, `"slack"`, `"email"`).
    pub channel_type: String,
    /// Channel-specific identifier (e.g. Telegram user ID, email address).
    pub channel_identifier: String,
    /// Whether the contact's ownership of this channel has been verified.
    pub verified: bool,
    /// Unix timestamp (milliseconds) when the channel was registered.
    pub created_at: u64,
}

impl ContactChannel {
    /// Create a new unverified channel for a contact.
    #[must_use]
    pub fn new(
        contact_id: impl Into<String>,
        channel_type: impl Into<String>,
        channel_identifier: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            contact_id: contact_id.into(),
            channel_type: channel_type.into(),
            channel_identifier: channel_identifier.into(),
            verified: false,
            created_at: now_ms(),
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

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

    #[test]
    fn matter_status_roundtrip() {
        for (s, expected) in [
            ("open", MatterStatus::Open),
            ("in_progress", MatterStatus::InProgress),
            ("closed", MatterStatus::Closed),
        ] {
            let parsed = MatterStatus::try_from(s).unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), s);
        }
    }

    #[test]
    fn matter_status_invalid() {
        assert!(MatterStatus::try_from("unknown").is_err());
    }

    #[test]
    fn interaction_kind_roundtrip() {
        for (s, expected) in [
            ("message", InteractionKind::Message),
            ("call", InteractionKind::Call),
            ("meeting", InteractionKind::Meeting),
            ("note", InteractionKind::Note),
        ] {
            let parsed = InteractionKind::try_from(s).unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), s);
        }
    }

    #[test]
    fn interaction_kind_invalid() {
        assert!(InteractionKind::try_from("sms").is_err());
    }

    #[test]
    fn new_matter_defaults() {
        let m = Matter::new("Test matter");
        assert!(!m.id.is_empty());
        assert_eq!(m.title, "Test matter");
        assert_eq!(m.status, MatterStatus::Open);
        assert!(m.contact_id.is_none());
        assert!(m.created_at > 0);
        assert_eq!(m.created_at, m.updated_at);
    }

    #[test]
    fn matter_for_contact() {
        let m = Matter::for_contact("A deal", "contact-uuid");
        assert_eq!(m.contact_id.as_deref(), Some("contact-uuid"));
    }

    #[test]
    fn new_interaction_defaults() {
        let i = Interaction::new(
            "contact-uuid",
            InteractionKind::Message,
            Some("hello".into()),
        );
        assert!(!i.id.is_empty());
        assert_eq!(i.contact_id, "contact-uuid");
        assert_eq!(i.kind, InteractionKind::Message);
        assert_eq!(i.body.as_deref(), Some("hello"));
        assert!(i.matter_id.is_none());
    }

    #[test]
    fn new_channel_defaults() {
        let ch = ContactChannel::new("contact-uuid", "telegram", "99999");
        assert!(!ch.id.is_empty());
        assert_eq!(ch.contact_id, "contact-uuid");
        assert_eq!(ch.channel_type, "telegram");
        assert_eq!(ch.channel_identifier, "99999");
        assert!(!ch.verified);
    }
}
