use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;

use crate::{Result, store::CrmStore, types::Contact};

/// In-memory implementation of [`CrmStore`] for testing.
#[derive(Debug, Default)]
pub struct MemoryCrmStore {
    contacts: RwLock<HashMap<String, Contact>>,
}

impl MemoryCrmStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CrmStore for MemoryCrmStore {
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use {super::*, crate::types::Contact};

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
}
