use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;

use crate::{
    Result,
    store::CrmStore,
    types::{
        Contact, ContactChannel, ContactStage, Interaction, Matter, MatterPhase, MatterStatus,
        PracticeArea,
    },
};

/// In-memory implementation of [`CrmStore`] for testing.
#[derive(Debug, Default)]
pub struct MemoryCrmStore {
    contacts: RwLock<HashMap<String, Contact>>,
    channels: RwLock<HashMap<String, ContactChannel>>,
    matters: RwLock<HashMap<String, Matter>>,
    interactions: RwLock<HashMap<String, Interaction>>,
}

impl MemoryCrmStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CrmStore for MemoryCrmStore {
    // ── Contacts ──────────────────────────────────────────────────────────────

    async fn list(&self) -> Result<Vec<Contact>> {
        let contacts = self.contacts.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<Contact> = contacts.values().cloned().collect();
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        Ok(out)
    }

    async fn list_filtered(
        &self,
        stage: Option<ContactStage>,
        search: Option<&str>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Contact>> {
        use secrecy::ExposeSecret;

        let contacts = self.contacts.read().unwrap_or_else(|e| e.into_inner());
        let search_lower = search.map(str::to_lowercase);
        let mut out: Vec<Contact> = contacts
            .values()
            .filter(|c| stage.is_none_or(|s| c.stage == s))
            .filter(|c| {
                search_lower.as_deref().is_none_or(|term| {
                    c.name.to_lowercase().contains(term)
                        || c.email
                            .as_ref()
                            .is_some_and(|e| e.expose_secret().to_lowercase().contains(term))
                        || c.phone
                            .as_ref()
                            .is_some_and(|p| p.expose_secret().to_lowercase().contains(term))
                })
            })
            .cloned()
            .collect();
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        let offset = offset as usize;
        let limit = limit as usize;
        Ok(out.into_iter().skip(offset).take(limit).collect())
    }

    async fn get(&self, id: &str) -> Result<Option<Contact>> {
        let contacts = self.contacts.read().unwrap_or_else(|e| e.into_inner());
        Ok(contacts.get(id).cloned())
    }

    async fn get_by_external(&self, source: &str, external_id: &str) -> Result<Option<Contact>> {
        let contacts = self.contacts.read().unwrap_or_else(|e| e.into_inner());
        Ok(contacts
            .values()
            .find(|c| {
                c.source.as_deref() == Some(source) && c.external_id.as_deref() == Some(external_id)
            })
            .cloned())
    }

    async fn upsert(&self, contact: Contact) -> Result<()> {
        let mut contacts = self.contacts.write().unwrap_or_else(|e| e.into_inner());
        contacts.insert(contact.id.clone(), contact);
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut contacts = self.contacts.write().unwrap_or_else(|e| e.into_inner());
        contacts.remove(id);
        Ok(())
    }

    // ── Contact channels ──────────────────────────────────────────────────────

    async fn list_channels_for_contact(&self, contact_id: &str) -> Result<Vec<ContactChannel>> {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<ContactChannel> = channels
            .values()
            .filter(|ch| ch.contact_id == contact_id)
            .cloned()
            .collect();
        out.sort_by_key(|ch| std::cmp::Reverse(ch.updated_at));
        Ok(out)
    }

    async fn get_channel_by_external(
        &self,
        channel_type: &str,
        channel_id: &str,
    ) -> Result<Option<ContactChannel>> {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        Ok(channels
            .values()
            .find(|ch| ch.channel_type == channel_type && ch.channel_id == channel_id)
            .cloned())
    }

    async fn upsert_channel(&self, channel: ContactChannel) -> Result<()> {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        channels.insert(channel.id.clone(), channel);
        Ok(())
    }

    async fn delete_channel(&self, id: &str) -> Result<()> {
        let mut channels = self.channels.write().unwrap_or_else(|e| e.into_inner());
        channels.remove(id);
        Ok(())
    }

    // ── Matters ───────────────────────────────────────────────────────────────

    async fn list_matters_filtered(
        &self,
        contact_id: Option<&str>,
        status: Option<MatterStatus>,
        phase: Option<MatterPhase>,
        practice_area: Option<PracticeArea>,
        search: Option<&str>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Matter>> {
        let matters = self.matters.read().unwrap_or_else(|e| e.into_inner());
        let search_lower = search.map(str::to_lowercase);
        let mut out: Vec<Matter> = matters
            .values()
            .filter(|m| contact_id.is_none_or(|cid| m.contact_id.as_deref() == Some(cid)))
            .filter(|m| status.is_none_or(|s| m.status == s))
            .filter(|m| phase.is_none_or(|p| m.phase == p))
            .filter(|m| practice_area.is_none_or(|pa| m.practice_area == pa))
            .filter(|m| {
                search_lower.as_deref().is_none_or(|term| {
                    m.title.to_lowercase().contains(term)
                        || m.description
                            .as_deref()
                            .is_some_and(|d| d.to_lowercase().contains(term))
                })
            })
            .cloned()
            .collect();
        out.sort_by_key(|m| std::cmp::Reverse(m.updated_at));
        let offset = offset as usize;
        let limit = limit as usize;
        Ok(out.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_matter(&self, id: &str) -> Result<Option<Matter>> {
        let matters = self.matters.read().unwrap_or_else(|e| e.into_inner());
        Ok(matters.get(id).cloned())
    }

    async fn upsert_matter(&self, matter: Matter) -> Result<()> {
        let mut matters = self.matters.write().unwrap_or_else(|e| e.into_inner());
        matters.insert(matter.id.clone(), matter);
        Ok(())
    }

    async fn delete_matter(&self, id: &str) -> Result<()> {
        let mut matters = self.matters.write().unwrap_or_else(|e| e.into_inner());
        matters.remove(id);
        Ok(())
    }

    // ── Interactions ──────────────────────────────────────────────────────────

    async fn list_interactions_by_contact(&self, contact_id: &str) -> Result<Vec<Interaction>> {
        let interactions = self.interactions.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<Interaction> = interactions
            .values()
            .filter(|i| i.contact_id == contact_id)
            .cloned()
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse(i.updated_at));
        Ok(out)
    }

    async fn list_interactions_by_matter(&self, matter_id: &str) -> Result<Vec<Interaction>> {
        let interactions = self.interactions.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<Interaction> = interactions
            .values()
            .filter(|i| i.matter_id.as_deref() == Some(matter_id))
            .cloned()
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse(i.updated_at));
        Ok(out)
    }

    async fn get_interaction(&self, id: &str) -> Result<Option<Interaction>> {
        let interactions = self.interactions.read().unwrap_or_else(|e| e.into_inner());
        Ok(interactions.get(id).cloned())
    }

    async fn upsert_interaction(&self, interaction: Interaction) -> Result<()> {
        let mut interactions = self.interactions.write().unwrap_or_else(|e| e.into_inner());
        interactions.insert(interaction.id.clone(), interaction);
        Ok(())
    }

    async fn delete_interaction(&self, id: &str) -> Result<()> {
        let mut interactions = self.interactions.write().unwrap_or_else(|e| e.into_inner());
        interactions.remove(id);
        Ok(())
    }

    async fn delete_interactions_before(&self, cutoff_epoch_ms: i64) -> Result<u64> {
        let mut interactions = self.interactions.write().unwrap_or_else(|e| e.into_inner());
        let before = interactions.len() as u64;
        interactions.retain(|_, i| i.created_at as i64 >= cutoff_epoch_ms);
        Ok(before - interactions.len() as u64)
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
            MatterPhase, MatterStatus, PracticeArea,
        },
    };

    // ── Contact tests ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("Alice");
        let id = c.id.clone();
        store.upsert(c).await.unwrap();
        let found = store.get(&id).await.unwrap().unwrap();
        assert_eq!(found.name, "Alice");
    }

    #[tokio::test]
    async fn upsert_updates_existing() {
        let store = MemoryCrmStore::new();
        let mut c = Contact::new("Alice");
        store.upsert(c.clone()).await.unwrap();
        c.name = "Alice Updated".into();
        store.upsert(c.clone()).await.unwrap();
        let found = store.get(&c.id).await.unwrap().unwrap();
        assert_eq!(found.name, "Alice Updated");
    }

    #[tokio::test]
    async fn list_returns_all() {
        let store = MemoryCrmStore::new();
        store.upsert(Contact::new("A")).await.unwrap();
        store.upsert(Contact::new("B")).await.unwrap();
        let all = store.list().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let store = MemoryCrmStore::new();
        assert!(store.get("missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("X");
        let id = c.id.clone();
        store.upsert(c).await.unwrap();
        store.delete(&id).await.unwrap();
        store.delete(&id).await.unwrap();
        assert!(store.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_by_external_finds_contact() {
        let store = MemoryCrmStore::new();
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
        let store = MemoryCrmStore::new();
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

    // ── ContactChannel tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_list_channels() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("Chan");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let ch = ContactChannel::new(&contact_id, "telegram", "tg-1");
        store.upsert_channel(ch).await.unwrap();
        let channels = store.list_channels_for_contact(&contact_id).await.unwrap();
        assert_eq!(channels.len(), 1);
    }

    #[tokio::test]
    async fn get_channel_by_external() {
        let store = MemoryCrmStore::new();
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
        let store = MemoryCrmStore::new();
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
        let store = MemoryCrmStore::new();
        let c = Contact::new("MatterC");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let m = Matter::new(&contact_id, "IP Filing", PracticeArea::IntellectualProperty);
        let mid = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        let found = store.get_matter(&mid).await.unwrap().unwrap();
        assert_eq!(found.title, "IP Filing");
    }

    #[tokio::test]
    async fn list_matters_by_contact() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("MatterMulti");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        store
            .upsert_matter(Matter::new(&contact_id, "M1", PracticeArea::Tax))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&contact_id, "M2", PracticeArea::RealEstate))
            .await
            .unwrap();
        let matters = store.list_matters_by_contact(&contact_id).await.unwrap();
        assert_eq!(matters.len(), 2);
    }

    #[tokio::test]
    async fn delete_matter_is_idempotent() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("MatterDel");
        store.upsert(c.clone()).await.unwrap();
        let m = Matter::new(&c.id, "To Delete", PracticeArea::Other);
        let mid = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        store.delete_matter(&mid).await.unwrap();
        store.delete_matter(&mid).await.unwrap();
        assert!(store.get_matter(&mid).await.unwrap().is_none());
    }

    // ── Interaction tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_interaction() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("IntC");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let i = Interaction::new(&contact_id, InteractionKind::Note, "Spoke briefly");
        let iid = i.id.clone();
        store.upsert_interaction(i).await.unwrap();
        let found = store.get_interaction(&iid).await.unwrap().unwrap();
        assert_eq!(found.summary, "Spoke briefly");
    }

    #[tokio::test]
    async fn list_interactions_by_contact() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("IntMulti");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        store
            .upsert_interaction(Interaction::new(
                &contact_id,
                InteractionKind::Call,
                "Call 1",
            ))
            .await
            .unwrap();
        store
            .upsert_interaction(Interaction::new(
                &contact_id,
                InteractionKind::Meeting,
                "Meeting 1",
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
    async fn list_interactions_by_matter() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("IntMatter");
        let contact_id = c.id.clone();
        store.upsert(c).await.unwrap();
        let m = Matter::new(&contact_id, "The Matter", PracticeArea::Litigation);
        let matter_id = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        let mut i = Interaction::new(&contact_id, InteractionKind::Email, "Settlement email");
        i.matter_id = Some(matter_id.clone());
        store.upsert_interaction(i).await.unwrap();
        let by_matter = store.list_interactions_by_matter(&matter_id).await.unwrap();
        assert_eq!(by_matter.len(), 1);
    }

    #[tokio::test]
    async fn delete_interaction_is_idempotent() {
        let store = MemoryCrmStore::new();
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
    async fn list_filtered_by_stage() {
        let store = MemoryCrmStore::new();
        let mut active = Contact::new("Active");
        active.stage = ContactStage::Active;
        store.upsert(active).await.unwrap();
        store.upsert(Contact::new("Lead")).await.unwrap();
        let results = store
            .list_filtered(Some(ContactStage::Active), None, 0, 50)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Active");
    }

    #[tokio::test]
    async fn list_filtered_by_search() {
        let store = MemoryCrmStore::new();
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
    async fn list_filtered_pagination() {
        let store = MemoryCrmStore::new();
        for i in 0..5 {
            store.upsert(Contact::new(format!("C{i}"))).await.unwrap();
        }
        let page = store.list_filtered(None, None, 0, 3).await.unwrap();
        assert_eq!(page.len(), 3);
        let rest = store.list_filtered(None, None, 3, 3).await.unwrap();
        assert_eq!(rest.len(), 2);
    }

    // ── get_with_channels tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn get_with_channels_returns_contact_and_channels() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("ChanUser");
        let cid = c.id.clone();
        store.upsert(c).await.unwrap();
        store
            .upsert_channel(ContactChannel::new(&cid, "slack", "U-1"))
            .await
            .unwrap();
        let result = store.get_with_channels(&cid).await.unwrap().unwrap();
        assert_eq!(result.channels.len(), 1);
    }

    #[tokio::test]
    async fn get_with_channels_missing_returns_none() {
        let store = MemoryCrmStore::new();
        assert!(store.get_with_channels("nope").await.unwrap().is_none());
    }

    // ── list_matters_filtered tests ───────────────────────────────────────────

    #[tokio::test]
    async fn list_matters_filtered_by_contact_and_practice_area() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("FC");
        store.upsert(c.clone()).await.unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "Corp", PracticeArea::Corporate))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "Tax", PracticeArea::Tax))
            .await
            .unwrap();
        let by_pa = store
            .list_matters_filtered(
                None,
                None,
                None,
                Some(PracticeArea::Corporate),
                None,
                0,
                u64::MAX,
            )
            .await
            .unwrap();
        assert_eq!(by_pa.len(), 1);
        assert_eq!(by_pa[0].title, "Corp");
    }

    #[tokio::test]
    async fn list_matters_filtered_by_status() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("StatusC");
        store.upsert(c.clone()).await.unwrap();
        let mut open = Matter::new(&c.id, "Open Matter", PracticeArea::Corporate);
        open.status = MatterStatus::Open;
        store.upsert_matter(open).await.unwrap();
        let mut closed = Matter::new(&c.id, "Closed Matter", PracticeArea::Corporate);
        closed.status = MatterStatus::Closed;
        store.upsert_matter(closed).await.unwrap();
        let results = store
            .list_matters_filtered(
                None,
                Some(MatterStatus::Open),
                None,
                None,
                None,
                0,
                u64::MAX,
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Open Matter");
    }

    #[tokio::test]
    async fn list_matters_filtered_by_phase() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("PhaseC");
        store.upsert(c.clone()).await.unwrap();
        let mut intake = Matter::new(&c.id, "Intake Matter", PracticeArea::Litigation);
        intake.phase = MatterPhase::Intake;
        store.upsert_matter(intake).await.unwrap();
        let mut discovery = Matter::new(&c.id, "Discovery Matter", PracticeArea::Litigation);
        discovery.phase = MatterPhase::Discovery;
        store.upsert_matter(discovery).await.unwrap();
        let results = store
            .list_matters_filtered(
                None,
                None,
                Some(MatterPhase::Discovery),
                None,
                None,
                0,
                u64::MAX,
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Discovery Matter");
    }

    #[tokio::test]
    async fn list_matters_filtered_search() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("SearchC");
        store.upsert(c.clone()).await.unwrap();
        store
            .upsert_matter(Matter::new(
                &c.id,
                "Contract Review",
                PracticeArea::Corporate,
            ))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "Tax Filing", PracticeArea::Tax))
            .await
            .unwrap();
        let results = store
            .list_matters_filtered(None, None, None, None, Some("contract"), 0, u64::MAX)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Contract Review");
    }

    #[tokio::test]
    async fn list_matters_default_returns_all() {
        let store = MemoryCrmStore::new();
        let c = Contact::new("AllC");
        store.upsert(c.clone()).await.unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "M1", PracticeArea::Tax))
            .await
            .unwrap();
        store
            .upsert_matter(Matter::new(&c.id, "M2", PracticeArea::Other))
            .await
            .unwrap();
        let all = store.list_matters().await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
