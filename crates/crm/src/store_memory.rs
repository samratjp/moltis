use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;

use crate::{
    Result,
    store::CrmStore,
    types::{Contact, ContactChannel, Interaction, Matter},
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

    async fn list_matters(&self) -> Result<Vec<Matter>> {
        let matters = self.matters.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<Matter> = matters.values().cloned().collect();
        out.sort_by_key(|m| std::cmp::Reverse(m.updated_at));
        Ok(out)
    }

    async fn list_matters_by_contact(&self, contact_id: &str) -> Result<Vec<Matter>> {
        let matters = self.matters.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<Matter> = matters
            .values()
            .filter(|m| m.contact_id.as_deref() == Some(contact_id))
            .cloned()
            .collect();
        out.sort_by_key(|m| std::cmp::Reverse(m.updated_at));
        Ok(out)
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
            Contact, ContactChannel, Interaction, InteractionKind, Matter, PracticeArea,
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
}
