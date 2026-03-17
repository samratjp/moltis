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
    matters: RwLock<HashMap<String, Matter>>,
    interactions: RwLock<Vec<Interaction>>,
    channels: RwLock<HashMap<String, ContactChannel>>,
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
            .iter()
            .filter(|i| i.contact_id == contact_id)
            .cloned()
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse(i.created_at));
        Ok(out)
    }

    async fn list_interactions_by_matter(&self, matter_id: &str) -> Result<Vec<Interaction>> {
        let interactions = self.interactions.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<Interaction> = interactions
            .iter()
            .filter(|i| i.matter_id.as_deref() == Some(matter_id))
            .cloned()
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse(i.created_at));
        Ok(out)
    }

    async fn create_interaction(&self, interaction: Interaction) -> Result<()> {
        let mut interactions = self.interactions.write().unwrap_or_else(|e| e.into_inner());
        interactions.push(interaction);
        Ok(())
    }

    async fn delete_interaction(&self, id: &str) -> Result<()> {
        let mut interactions = self.interactions.write().unwrap_or_else(|e| e.into_inner());
        interactions.retain(|i| i.id != id);
        Ok(())
    }

    // ── Contact channels ──────────────────────────────────────────────────────

    async fn list_channels_by_contact(&self, contact_id: &str) -> Result<Vec<ContactChannel>> {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        let mut out: Vec<ContactChannel> = channels
            .values()
            .filter(|ch| ch.contact_id == contact_id)
            .cloned()
            .collect();
        out.sort_by_key(|ch| ch.created_at);
        Ok(out)
    }

    async fn get_channel_by_type_and_id(
        &self,
        channel_type: &str,
        channel_identifier: &str,
    ) -> Result<Option<ContactChannel>> {
        let channels = self.channels.read().unwrap_or_else(|e| e.into_inner());
        Ok(channels
            .values()
            .find(|ch| {
                ch.channel_type == channel_type && ch.channel_identifier == channel_identifier
            })
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {
        super::*,
        crate::types::{Contact, ContactChannel, Interaction, InteractionKind, Matter},
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

    // ── Matter tests ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_get_matter() {
        let store = MemoryCrmStore::new();
        let m = Matter::new("Deal");
        let id = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        let found = store.get_matter(&id).await.unwrap().unwrap();
        assert_eq!(found.title, "Deal");
    }

    #[tokio::test]
    async fn list_matters_by_contact_filters_correctly() {
        let store = MemoryCrmStore::new();
        let cid = "contact-1";
        let m1 = Matter::for_contact("Deal A", cid);
        let m2 = Matter::new("Unrelated");
        store.upsert_matter(m1.clone()).await.unwrap();
        store.upsert_matter(m2).await.unwrap();
        let by_contact = store.list_matters_by_contact(cid).await.unwrap();
        assert_eq!(by_contact.len(), 1);
        assert_eq!(by_contact[0].id, m1.id);
    }

    #[tokio::test]
    async fn delete_matter_is_idempotent() {
        let store = MemoryCrmStore::new();
        let m = Matter::new("Gone");
        let id = m.id.clone();
        store.upsert_matter(m).await.unwrap();
        store.delete_matter(&id).await.unwrap();
        store.delete_matter(&id).await.unwrap();
        assert!(store.get_matter(&id).await.unwrap().is_none());
    }

    // ── Interaction tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_list_interaction_by_contact() {
        let store = MemoryCrmStore::new();
        let cid = "c-1";
        let i = Interaction::new(cid, InteractionKind::Note, Some("hi".into()));
        store.create_interaction(i.clone()).await.unwrap();
        let found = store.list_interactions_by_contact(cid).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, i.id);
    }

    #[tokio::test]
    async fn list_interactions_by_matter_filters_correctly() {
        let store = MemoryCrmStore::new();
        let cid = "c-2";
        let mid = "m-2";
        let mut i = Interaction::new(cid, InteractionKind::Call, None);
        i.matter_id = Some(mid.into());
        store.create_interaction(i.clone()).await.unwrap();
        store
            .create_interaction(Interaction::new(cid, InteractionKind::Message, None))
            .await
            .unwrap();
        let by_matter = store.list_interactions_by_matter(mid).await.unwrap();
        assert_eq!(by_matter.len(), 1);
        assert_eq!(by_matter[0].id, i.id);
    }

    #[tokio::test]
    async fn delete_interaction_is_idempotent() {
        let store = MemoryCrmStore::new();
        let cid = "c-3";
        let i = Interaction::new(cid, InteractionKind::Meeting, None);
        let iid = i.id.clone();
        store.create_interaction(i).await.unwrap();
        store.delete_interaction(&iid).await.unwrap();
        store.delete_interaction(&iid).await.unwrap();
        assert!(
            store
                .list_interactions_by_contact(cid)
                .await
                .unwrap()
                .is_empty()
        );
    }

    // ── ContactChannel tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn upsert_and_list_channels() {
        let store = MemoryCrmStore::new();
        let cid = "c-4";
        let ch = ContactChannel::new(cid, "telegram", "111");
        store.upsert_channel(ch.clone()).await.unwrap();
        let channels = store.list_channels_by_contact(cid).await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, ch.id);
    }

    #[tokio::test]
    async fn get_channel_by_type_and_id_finds_channel() {
        let store = MemoryCrmStore::new();
        let cid = "c-5";
        let ch = ContactChannel::new(cid, "whatsapp", "+1234");
        store.upsert_channel(ch.clone()).await.unwrap();
        let found = store
            .get_channel_by_type_and_id("whatsapp", "+1234")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, ch.id);
    }

    #[tokio::test]
    async fn delete_channel_is_idempotent() {
        let store = MemoryCrmStore::new();
        let cid = "c-6";
        let ch = ContactChannel::new(cid, "slack", "U001");
        let chid = ch.id.clone();
        store.upsert_channel(ch).await.unwrap();
        store.delete_channel(&chid).await.unwrap();
        store.delete_channel(&chid).await.unwrap();
        assert!(
            store
                .list_channels_by_contact(cid)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
