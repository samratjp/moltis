use {
    secrecy::{ExposeSecret, Secret},
    serde::{Deserialize, Serialize},
};

// ── Serde helpers for Secret<String> ─────────────────────────────────────────
//
// `Secret<String>` requires the `SerializableSecret` marker for its Serialize
// impl as a security guardrail. We use explicit helpers instead.

fn serialize_option_secret<S>(
    secret: &Option<Secret<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match secret {
        Some(s) => serializer.serialize_some(s.expose_secret()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_option_secret<'de, D>(deserializer: D) -> Result<Option<Secret<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.map(Secret::new))
}

// ── Contact stage ─────────────────────────────────────────────────────────────

/// Lifecycle stage of a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContactStage {
    /// Initial outreach — not yet engaged.
    #[default]
    Lead,
    /// Engaged but no active matter.
    Prospect,
    /// Has an open matter.
    Active,
    /// Previously active, currently dormant.
    Inactive,
    /// Relationship closed.
    Closed,
}

impl ContactStage {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Prospect => "prospect",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Closed => "closed",
        }
    }
}

impl std::fmt::Display for ContactStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ContactStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lead" => Ok(Self::Lead),
            "prospect" => Ok(Self::Prospect),
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "closed" => Ok(Self::Closed),
            other => Err(format!("unknown ContactStage: {other:?}")),
        }
    }
}

// ── Matter status ─────────────────────────────────────────────────────────────

/// Lifecycle status of a matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatterStatus {
    /// Matter is actively being worked on.
    #[default]
    Open,
    /// Matter is temporarily paused.
    OnHold,
    /// Matter has been resolved.
    Closed,
    /// Matter is archived (read-only).
    Archived,
}

impl MatterStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::OnHold => "on_hold",
            Self::Closed => "closed",
            Self::Archived => "archived",
        }
    }
}

impl std::fmt::Display for MatterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MatterStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "on_hold" => Ok(Self::OnHold),
            "closed" => Ok(Self::Closed),
            "archived" => Ok(Self::Archived),
            other => Err(format!("unknown MatterStatus: {other:?}")),
        }
    }
}

// ── Matter phase ──────────────────────────────────────────────────────────────

/// Phase within the lifecycle of a matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatterPhase {
    /// Initial assessment and onboarding.
    #[default]
    Intake,
    /// Gathering facts, evidence, or documents.
    Discovery,
    /// Active negotiation or dispute.
    Negotiation,
    /// Working toward or implementing a resolution.
    Resolution,
    /// Post-resolution review or appeal period.
    Review,
    /// Matter is fully closed.
    Closed,
}

impl MatterPhase {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Intake => "intake",
            Self::Discovery => "discovery",
            Self::Negotiation => "negotiation",
            Self::Resolution => "resolution",
            Self::Review => "review",
            Self::Closed => "closed",
        }
    }
}

impl std::fmt::Display for MatterPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MatterPhase {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "intake" => Ok(Self::Intake),
            "discovery" => Ok(Self::Discovery),
            "negotiation" => Ok(Self::Negotiation),
            "resolution" => Ok(Self::Resolution),
            "review" => Ok(Self::Review),
            "closed" => Ok(Self::Closed),
            other => Err(format!("unknown MatterPhase: {other:?}")),
        }
    }
}

// ── Practice area ─────────────────────────────────────────────────────────────

/// Practice area or domain of a matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PracticeArea {
    /// Business formation, M&A, contracts.
    Corporate,
    /// Employment law and labour relations.
    Employment,
    /// Family law, divorce, custody.
    FamilyLaw,
    /// Immigration and visa matters.
    Immigration,
    /// Patents, trademarks, copyrights.
    IntellectualProperty,
    /// Litigation and dispute resolution.
    Litigation,
    /// Real estate transactions and disputes.
    RealEstate,
    /// Tax planning and disputes.
    Tax,
    /// Catch-all for uncategorised matters.
    #[default]
    Other,
}

impl PracticeArea {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Corporate => "corporate",
            Self::Employment => "employment",
            Self::FamilyLaw => "family_law",
            Self::Immigration => "immigration",
            Self::IntellectualProperty => "intellectual_property",
            Self::Litigation => "litigation",
            Self::RealEstate => "real_estate",
            Self::Tax => "tax",
            Self::Other => "other",
        }
    }
}

impl std::fmt::Display for PracticeArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PracticeArea {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "corporate" => Ok(Self::Corporate),
            "employment" => Ok(Self::Employment),
            "family_law" => Ok(Self::FamilyLaw),
            "immigration" => Ok(Self::Immigration),
            "intellectual_property" => Ok(Self::IntellectualProperty),
            "litigation" => Ok(Self::Litigation),
            "real_estate" => Ok(Self::RealEstate),
            "tax" => Ok(Self::Tax),
            "other" => Ok(Self::Other),
            other => Err(format!("unknown PracticeArea: {other:?}")),
        }
    }
}

// ── Interaction kind ──────────────────────────────────────────────────────────

/// The type of interaction recorded between the system and a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    /// Phone or video call.
    Call,
    /// Email exchange.
    Email,
    /// Chat message (any channel).
    Message,
    /// In-person or virtual meeting.
    Meeting,
    /// Internal note.
    Note,
    /// Document shared or received.
    Document,
}

impl InteractionKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Email => "email",
            Self::Message => "message",
            Self::Meeting => "meeting",
            Self::Note => "note",
            Self::Document => "document",
        }
    }
}

impl std::fmt::Display for InteractionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for InteractionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "call" => Ok(Self::Call),
            "email" => Ok(Self::Email),
            "message" => Ok(Self::Message),
            "meeting" => Ok(Self::Meeting),
            "note" => Ok(Self::Note),
            "document" => Ok(Self::Document),
            other => Err(format!("unknown InteractionKind: {other:?}")),
        }
    }
}

// ── Contact ───────────────────────────────────────────────────────────────────

/// A contact in the CRM system.
///
/// Contacts represent people or entities that interact with the system through
/// any channel (Telegram, WhatsApp, Slack, etc.) or are imported from external
/// sources.
///
/// PII fields (`email`, `phone`) are wrapped in [`Secret`] to prevent
/// accidental exposure in logs and debug output.
#[derive(Clone, Serialize, Deserialize)]
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
    /// Email address — PII, redacted in debug output.
    #[serde(
        default,
        serialize_with = "serialize_option_secret",
        deserialize_with = "deserialize_option_secret"
    )]
    pub email: Option<Secret<String>>,
    /// Phone number — PII, redacted in debug output.
    #[serde(
        default,
        serialize_with = "serialize_option_secret",
        deserialize_with = "deserialize_option_secret"
    )]
    pub phone: Option<Secret<String>>,
    /// Lifecycle stage of this contact.
    #[serde(default)]
    pub stage: ContactStage,
    /// Arbitrary structured metadata stored as a JSON object.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Unix timestamp (milliseconds) when the contact was created.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) when the contact was last updated.
    pub updated_at: u64,
}

impl std::fmt::Debug for Contact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Contact")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("source", &self.source)
            .field("external_id", &self.external_id)
            .field("email", &self.email.as_ref().map(|_| "[REDACTED]"))
            .field("phone", &self.phone.as_ref().map(|_| "[REDACTED]"))
            .field("stage", &self.stage)
            .field("metadata", &self.metadata)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl PartialEq for Contact {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.source == other.source
            && self.external_id == other.external_id
            && opt_secret_eq(&self.email, &other.email)
            && opt_secret_eq(&self.phone, &other.phone)
            && self.stage == other.stage
            && self.metadata == other.metadata
            && self.created_at == other.created_at
            && self.updated_at == other.updated_at
    }
}

fn opt_secret_eq(a: &Option<Secret<String>>, b: &Option<Secret<String>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x.expose_secret() == y.expose_secret(),
        _ => false,
    }
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
            email: None,
            phone: None,
            stage: ContactStage::default(),
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

// ── ContactChannel ────────────────────────────────────────────────────────────

/// A channel identity record linking a contact to an external platform.
///
/// A single contact can have multiple channel identities (e.g., the same
/// person reachable on Telegram, WhatsApp, and email). The `channel_type`
/// field holds the channel identifier string (e.g., `"telegram"`, `"slack"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContactChannel {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Contact this channel belongs to.
    pub contact_id: String,
    /// Channel type identifier (e.g. `"telegram"`, `"whatsapp"`, `"email"`).
    pub channel_type: String,
    /// Channel-native identifier for this contact (e.g. Telegram user ID).
    pub channel_id: String,
    /// Human-readable display name on this channel (e.g. username, email address).
    #[serde(default)]
    pub display_name: Option<String>,
    /// Whether this channel identity has been verified.
    #[serde(default)]
    pub verified: bool,
    /// Unix timestamp (milliseconds) when the record was created.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) when the record was last updated.
    pub updated_at: u64,
}

impl ContactChannel {
    /// Create a new channel identity for a contact.
    #[must_use]
    pub fn new(
        contact_id: impl Into<String>,
        channel_type: impl Into<String>,
        channel_id: impl Into<String>,
    ) -> Self {
        let now = now_ms();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            contact_id: contact_id.into(),
            channel_type: channel_type.into(),
            channel_id: channel_id.into(),
            display_name: None,
            verified: false,
            created_at: now,
            updated_at: now,
        }
    }
}

// ── Matter ────────────────────────────────────────────────────────────────────

/// A legal matter or case optionally linked to a contact.
///
/// `contact_id` is nullable: a matter may span multiple contacts or exist
/// independently. When a contact is deleted the DB sets this to NULL via
/// `ON DELETE SET NULL`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Matter {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Contact primarily associated with this matter, if any.
    #[serde(default)]
    pub contact_id: Option<String>,
    /// Short descriptive title.
    pub title: String,
    /// Optional longer description.
    #[serde(default)]
    pub description: Option<String>,
    /// Current lifecycle status.
    #[serde(default)]
    pub status: MatterStatus,
    /// Current phase within the matter lifecycle.
    #[serde(default)]
    pub phase: MatterPhase,
    /// Practice area or domain of this matter.
    pub practice_area: PracticeArea,
    /// Unix timestamp (milliseconds) when the matter was created.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) when the matter was last updated.
    pub updated_at: u64,
}

impl Matter {
    /// Create a new matter linked to the given contact.
    #[must_use]
    pub fn new(
        contact_id: impl Into<String>,
        title: impl Into<String>,
        practice_area: PracticeArea,
    ) -> Self {
        let now = now_ms();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            contact_id: Some(contact_id.into()),
            title: title.into(),
            description: None,
            status: MatterStatus::default(),
            phase: MatterPhase::default(),
            practice_area,
            created_at: now,
            updated_at: now,
        }
    }
}

// ── Interaction ───────────────────────────────────────────────────────────────

/// A recorded touchpoint between the system and a contact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Contact involved in this interaction.
    pub contact_id: String,
    /// Optional matter this interaction relates to.
    #[serde(default)]
    pub matter_id: Option<String>,
    /// Type of interaction.
    pub kind: InteractionKind,
    /// Summary of what occurred.
    pub summary: String,
    /// Optional channel where the interaction took place (e.g. `"telegram"`).
    #[serde(default)]
    pub channel: Option<String>,
    /// Unix timestamp (milliseconds) when the interaction was created.
    pub created_at: u64,
    /// Unix timestamp (milliseconds) when the interaction was last updated.
    pub updated_at: u64,
}

impl Interaction {
    /// Create a new interaction for the given contact.
    #[must_use]
    pub fn new(
        contact_id: impl Into<String>,
        kind: InteractionKind,
        summary: impl Into<String>,
    ) -> Self {
        let now = now_ms();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            contact_id: contact_id.into(),
            matter_id: None,
            kind,
            summary: summary.into(),
            channel: None,
            created_at: now,
            updated_at: now,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

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
    use super::*;

    // ── Contact tests ─────────────────────────────────────────────────────────

    #[test]
    fn new_contact_has_uuid() {
        let c = Contact::new("Alice");
        assert!(!c.id.is_empty());
        assert_eq!(c.name, "Alice");
        assert!(c.source.is_none());
        assert!(c.external_id.is_none());
        assert!(c.email.is_none());
        assert!(c.phone.is_none());
        assert_eq!(c.stage, ContactStage::Lead);
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
    fn contact_pii_fields_serde_roundtrip() {
        use secrecy::ExposeSecret;
        let mut c = Contact::new("Dave");
        c.email = Some(Secret::new("dave@example.com".to_owned()));
        c.phone = Some(Secret::new("+15555550100".to_owned()));
        let json = serde_json::to_string(&c).unwrap();
        let c2: Contact = serde_json::from_str(&json).unwrap();
        assert_eq!(
            c2.email.as_ref().unwrap().expose_secret(),
            "dave@example.com"
        );
        assert_eq!(c2.phone.as_ref().unwrap().expose_secret(), "+15555550100");
    }

    #[test]
    fn contact_debug_redacts_pii() {
        let mut c = Contact::new("Eve");
        c.email = Some(Secret::new("eve@example.com".to_owned()));
        c.phone = Some(Secret::new("+15555550200".to_owned()));
        let debug = format!("{c:?}");
        assert!(!debug.contains("eve@example.com"));
        assert!(!debug.contains("+15555550200"));
        assert!(debug.contains("[REDACTED]"));
    }

    // ── Enum round-trip tests ─────────────────────────────────────────────────

    #[test]
    fn contact_stage_roundtrip() {
        for v in [
            ContactStage::Lead,
            ContactStage::Prospect,
            ContactStage::Active,
            ContactStage::Inactive,
            ContactStage::Closed,
        ] {
            assert_eq!(v.as_str().parse::<ContactStage>().unwrap(), v);
        }
    }

    #[test]
    fn matter_status_roundtrip() {
        for v in [
            MatterStatus::Open,
            MatterStatus::OnHold,
            MatterStatus::Closed,
            MatterStatus::Archived,
        ] {
            assert_eq!(v.as_str().parse::<MatterStatus>().unwrap(), v);
        }
    }

    #[test]
    fn matter_phase_roundtrip() {
        for v in [
            MatterPhase::Intake,
            MatterPhase::Discovery,
            MatterPhase::Negotiation,
            MatterPhase::Resolution,
            MatterPhase::Review,
            MatterPhase::Closed,
        ] {
            assert_eq!(v.as_str().parse::<MatterPhase>().unwrap(), v);
        }
    }

    #[test]
    fn practice_area_roundtrip() {
        for v in [
            PracticeArea::Corporate,
            PracticeArea::Employment,
            PracticeArea::FamilyLaw,
            PracticeArea::Immigration,
            PracticeArea::IntellectualProperty,
            PracticeArea::Litigation,
            PracticeArea::RealEstate,
            PracticeArea::Tax,
            PracticeArea::Other,
        ] {
            assert_eq!(v.as_str().parse::<PracticeArea>().unwrap(), v);
        }
    }

    #[test]
    fn interaction_kind_roundtrip() {
        for v in [
            InteractionKind::Call,
            InteractionKind::Email,
            InteractionKind::Message,
            InteractionKind::Meeting,
            InteractionKind::Note,
            InteractionKind::Document,
        ] {
            assert_eq!(v.as_str().parse::<InteractionKind>().unwrap(), v);
        }
    }

    #[test]
    fn enum_unknown_value_errors() {
        assert!("bogus".parse::<ContactStage>().is_err());
        assert!("bogus".parse::<MatterStatus>().is_err());
        assert!("bogus".parse::<MatterPhase>().is_err());
        assert!("bogus".parse::<PracticeArea>().is_err());
        assert!("bogus".parse::<InteractionKind>().is_err());
    }

    // ── ContactChannel tests ──────────────────────────────────────────────────

    #[test]
    fn contact_channel_new() {
        let ch = ContactChannel::new("cid-1", "telegram", "tg-999");
        assert!(!ch.id.is_empty());
        assert_eq!(ch.contact_id, "cid-1");
        assert_eq!(ch.channel_type, "telegram");
        assert_eq!(ch.channel_id, "tg-999");
        assert!(ch.display_name.is_none());
        assert!(!ch.verified);
    }

    #[test]
    fn contact_channel_serde_roundtrip() {
        let ch = ContactChannel::new("cid-2", "slack", "U456");
        let json = serde_json::to_string(&ch).unwrap();
        let ch2: ContactChannel = serde_json::from_str(&json).unwrap();
        assert_eq!(ch, ch2);
    }

    // ── Matter tests ──────────────────────────────────────────────────────────

    #[test]
    fn matter_new_defaults() {
        let m = Matter::new("cid-1", "Contract Review", PracticeArea::Corporate);
        assert!(!m.id.is_empty());
        assert_eq!(m.contact_id.as_deref(), Some("cid-1"));
        assert_eq!(m.title, "Contract Review");
        assert_eq!(m.status, MatterStatus::Open);
        assert_eq!(m.phase, MatterPhase::Intake);
        assert_eq!(m.practice_area, PracticeArea::Corporate);
        assert!(m.description.is_none());
    }

    #[test]
    fn matter_serde_roundtrip() {
        let m = Matter::new("cid-3", "IP Dispute", PracticeArea::IntellectualProperty);
        let json = serde_json::to_string(&m).unwrap();
        let m2: Matter = serde_json::from_str(&json).unwrap();
        assert_eq!(m, m2);
    }

    // ── Interaction tests ─────────────────────────────────────────────────────

    #[test]
    fn interaction_new_defaults() {
        let i = Interaction::new("cid-1", InteractionKind::Call, "Intake call");
        assert!(!i.id.is_empty());
        assert_eq!(i.contact_id, "cid-1");
        assert_eq!(i.kind, InteractionKind::Call);
        assert_eq!(i.summary, "Intake call");
        assert!(i.matter_id.is_none());
        assert!(i.channel.is_none());
    }

    #[test]
    fn interaction_serde_roundtrip() {
        let i = Interaction::new("cid-4", InteractionKind::Email, "Sent retainer agreement");
        let json = serde_json::to_string(&i).unwrap();
        let i2: Interaction = serde_json::from_str(&json).unwrap();
        assert_eq!(i, i2);
    }
}
