//! Data retention engine.
//!
//! Runs a periodic sweep that purges aged records from CRM interactions,
//! message log, and session JSONL files according to the `[data_retention]`
//! config section. When `enabled` is `false` (the default), no data is ever
//! deleted.

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use {
    moltis_channels::message_log::MessageLog,
    moltis_config::schema::DataRetentionConfig,
    moltis_sessions::store::SessionStore,
    tracing::{debug, info, instrument, warn},
};

#[cfg(feature = "crm")]
use moltis_crm::store::CrmStore;

/// Per-store counts from a single retention sweep.
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct RetentionReport {
    /// Number of CRM interaction records deleted (or that would be deleted in dry-run).
    pub crm_interactions_deleted: u64,
    /// Number of message log entries deleted (or that would be deleted in dry-run).
    pub message_log_deleted: u64,
    /// Number of session files deleted (or that would be deleted in dry-run).
    pub sessions_deleted: u64,
    /// Whether this was a dry-run (nothing was actually deleted).
    pub dry_run: bool,
}

/// Engine that deletes aged records according to the configured retention policy.
pub struct RetentionEngine {
    config: DataRetentionConfig,
    /// Effective CRM interaction retention days (config field or `crm.retention_days` fallback).
    crm_interactions_days: Option<u64>,
    #[cfg(feature = "crm")]
    crm: Arc<dyn CrmStore>,
    message_log: Arc<dyn MessageLog>,
    session_store: Arc<SessionStore>,
}

impl RetentionEngine {
    /// Create a new engine.
    ///
    /// `crm_interactions_days_fallback` is used when
    /// `config.crm_interactions_days` is `None` — typically `crm.retention_days`.
    #[cfg(feature = "crm")]
    pub fn new(
        config: DataRetentionConfig,
        crm_interactions_days_fallback: Option<u64>,
        crm: Arc<dyn CrmStore>,
        message_log: Arc<dyn MessageLog>,
        session_store: Arc<SessionStore>,
    ) -> Self {
        let crm_interactions_days = config
            .crm_interactions_days
            .or(crm_interactions_days_fallback);
        Self {
            config,
            crm_interactions_days,
            crm,
            message_log,
            session_store,
        }
    }

    #[cfg(not(feature = "crm"))]
    pub fn new(
        config: DataRetentionConfig,
        crm_interactions_days_fallback: Option<u64>,
        message_log: Arc<dyn MessageLog>,
        session_store: Arc<SessionStore>,
    ) -> Self {
        let crm_interactions_days = config
            .crm_interactions_days
            .or(crm_interactions_days_fallback);
        Self {
            config,
            crm_interactions_days,
            message_log,
            session_store,
        }
    }

    /// Run one retention sweep and return a report.
    ///
    /// In dry-run mode the report contains what *would* have been deleted but no
    /// rows are actually removed.
    #[instrument(skip(self), fields(dry_run = self.config.dry_run))]
    pub async fn run_once(&self) -> RetentionReport {
        let mut report = RetentionReport {
            dry_run: self.config.dry_run,
            ..Default::default()
        };

        #[cfg(feature = "crm")]
        if let Some(days) = self.crm_interactions_days {
            report.crm_interactions_deleted = self.purge_crm_interactions(days).await;
        }

        if let Some(days) = self.config.message_log_days {
            report.message_log_deleted = self.purge_message_log(days).await;
        }

        if let Some(days) = self.config.sessions_days {
            report.sessions_deleted = self.purge_sessions(days).await;
        }

        info!(
            crm_interactions = report.crm_interactions_deleted,
            message_log = report.message_log_deleted,
            sessions = report.sessions_deleted,
            dry_run = report.dry_run,
            "retention sweep complete",
        );

        report
    }

    /// Spawn the background retention loop as a detached tokio task.
    ///
    /// Waits 30 seconds after startup before the first sweep, then runs once
    /// per day.
    pub fn spawn(engine: Arc<Self>) {
        tokio::spawn(async move {
            engine.run_loop().await;
        });
    }

    async fn run_loop(&self) {
        // Short startup delay so retention doesn't compete with server init.
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            let _ = self.run_once().await;
            let interval = Duration::try_from(time::Duration::days(1))
                .unwrap_or(Duration::from_secs(24 * 3600));
            tokio::time::sleep(interval).await;
        }
    }

    // ── store-specific helpers ─────────────────────────────────────────────────

    #[cfg(feature = "crm")]
    async fn purge_crm_interactions(&self, days: u64) -> u64 {
        let cutoff_ms = cutoff_epoch_ms(days);
        if self.config.dry_run {
            debug!(
                days,
                cutoff_ms, "dry-run: would purge CRM interactions older than cutoff"
            );
            return 0;
        }
        match self.crm.delete_interactions_before(cutoff_ms).await {
            Ok(n) => {
                debug!(n, days, "purged CRM interactions");
                n
            },
            Err(e) => {
                warn!("retention: CRM interaction purge failed: {e}");
                0
            },
        }
    }

    async fn purge_message_log(&self, days: u64) -> u64 {
        let cutoff_secs = cutoff_epoch_secs(days);
        if self.config.dry_run {
            debug!(
                days,
                cutoff_secs, "dry-run: would purge message log entries older than cutoff"
            );
            return 0;
        }
        match self.message_log.delete_before(cutoff_secs).await {
            Ok(n) => {
                debug!(n, days, "purged message log entries");
                n
            },
            Err(e) => {
                warn!("retention: message log purge failed: {e}");
                0
            },
        }
    }

    async fn purge_sessions(&self, days: u64) -> u64 {
        let cutoff = cutoff_system_time(days);
        if self.config.dry_run {
            debug!(days, "dry-run: would purge session files older than cutoff");
            return 0;
        }
        match self.session_store.cleanup_before(cutoff).await {
            Ok(n) => {
                debug!(n, days, "purged session files");
                n
            },
            Err(e) => {
                warn!("retention: session purge failed: {e}");
                0
            },
        }
    }
}

// ── cutoff helpers ─────────────────────────────────────────────────────────────

/// Cutoff as epoch milliseconds — for CRM `created_at` (stored as epoch ms).
fn cutoff_epoch_ms(days: u64) -> i64 {
    let std_duration =
        Duration::try_from(time::Duration::days(days as i64)).unwrap_or(Duration::ZERO);
    SystemTime::now()
        .checked_sub(std_duration)
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Cutoff as epoch seconds — for message log `created_at` (stored as epoch secs).
fn cutoff_epoch_secs(days: u64) -> i64 {
    let std_duration =
        Duration::try_from(time::Duration::days(days as i64)).unwrap_or(Duration::ZERO);
    SystemTime::now()
        .checked_sub(std_duration)
        .unwrap_or(UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Cutoff as `SystemTime` — for session file mtime comparison.
fn cutoff_system_time(days: u64) -> SystemTime {
    let std_duration =
        Duration::try_from(time::Duration::days(days as i64)).unwrap_or(Duration::ZERO);
    SystemTime::now()
        .checked_sub(std_duration)
        .unwrap_or(UNIX_EPOCH)
}

// ── tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;

    #[test]
    fn cutoff_epoch_ms_zero_days_is_now() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let cutoff = cutoff_epoch_ms(0);
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        assert!(cutoff >= before && cutoff <= after);
    }

    #[test]
    fn cutoff_epoch_secs_30_days_is_in_the_past() {
        let cutoff = cutoff_epoch_secs(30);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        // Cutoff should be roughly 30 days before now.
        let expected = now - 30 * 24 * 3600;
        assert!(
            (cutoff - expected).abs() < 5,
            "cutoff={cutoff}, expected≈{expected}"
        );
    }

    #[test]
    fn cutoff_system_time_7_days_is_before_now() {
        let cutoff = cutoff_system_time(7);
        assert!(cutoff < SystemTime::now());
    }

    #[tokio::test]
    async fn dry_run_deletes_nothing() {
        use {
            moltis_crm::{
                store::MemoryCrmStore,
                types::{Contact, Interaction, InteractionKind},
            },
            moltis_sessions::store::SessionStore,
        };

        // Build an in-memory CRM store with one ancient interaction.
        let crm = Arc::new(MemoryCrmStore::new());
        let contact = Contact::new("Test");
        let contact_id = contact.id.clone();
        crm.upsert(contact).await.unwrap();
        // Create interaction with created_at = 0 (epoch start — ancient).
        let mut interaction = Interaction::new(&contact_id, InteractionKind::Note, "old note");
        interaction.created_at = 0;
        crm.upsert_interaction(interaction.clone()).await.unwrap();

        // SessionStore in a temp dir.
        let tmp = tempfile::tempdir().unwrap();
        let session_store = Arc::new(SessionStore::new(tmp.path().to_path_buf()));

        // DummyMessageLog that tracks delete calls.
        struct DummyLog;
        #[async_trait::async_trait]
        impl MessageLog for DummyLog {
            async fn log(
                &self,
                _: moltis_channels::message_log::MessageLogEntry,
            ) -> moltis_channels::Result<()> {
                Ok(())
            }

            async fn list_by_account(
                &self,
                _: &str,
                _: &str,
                _: u32,
            ) -> moltis_channels::Result<Vec<moltis_channels::message_log::MessageLogEntry>>
            {
                Ok(vec![])
            }

            async fn unique_senders(
                &self,
                _: &str,
                _: &str,
            ) -> moltis_channels::Result<Vec<moltis_channels::message_log::SenderSummary>>
            {
                Ok(vec![])
            }

            async fn delete_before(&self, _: i64) -> moltis_channels::Result<u64> {
                Ok(0)
            }
        }

        let config = DataRetentionConfig {
            enabled: true,
            dry_run: true,
            crm_interactions_days: Some(1),
            message_log_days: Some(1),
            sessions_days: Some(1),
            ..Default::default()
        };

        let engine = RetentionEngine::new(
            config,
            None,
            crm.clone() as Arc<dyn CrmStore>,
            Arc::new(DummyLog) as Arc<dyn MessageLog>,
            session_store,
        );

        let report = engine.run_once().await;
        assert_eq!(
            report.crm_interactions_deleted, 0,
            "dry-run must not delete"
        );
        assert!(report.dry_run);

        // The interaction must still be there.
        let still_there = crm.get_interaction(&interaction.id).await.unwrap();
        assert!(still_there.is_some(), "dry-run must not remove CRM records");
    }

    #[tokio::test]
    async fn live_run_deletes_old_crm_interactions() {
        use {
            moltis_crm::{
                store::MemoryCrmStore,
                types::{Contact, Interaction, InteractionKind},
            },
            moltis_sessions::store::SessionStore,
        };

        let crm = Arc::new(MemoryCrmStore::new());
        let contact = Contact::new("Test2");
        let contact_id = contact.id.clone();
        crm.upsert(contact).await.unwrap();

        // Ancient interaction (epoch 0).
        let mut old = Interaction::new(&contact_id, InteractionKind::Call, "old call");
        old.created_at = 0;
        crm.upsert_interaction(old.clone()).await.unwrap();

        // Recent interaction (now).
        let recent = Interaction::new(&contact_id, InteractionKind::Note, "recent note");
        crm.upsert_interaction(recent.clone()).await.unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let session_store = Arc::new(SessionStore::new(tmp.path().to_path_buf()));

        struct NoopLog;
        #[async_trait::async_trait]
        impl MessageLog for NoopLog {
            async fn log(
                &self,
                _: moltis_channels::message_log::MessageLogEntry,
            ) -> moltis_channels::Result<()> {
                Ok(())
            }

            async fn list_by_account(
                &self,
                _: &str,
                _: &str,
                _: u32,
            ) -> moltis_channels::Result<Vec<moltis_channels::message_log::MessageLogEntry>>
            {
                Ok(vec![])
            }

            async fn unique_senders(
                &self,
                _: &str,
                _: &str,
            ) -> moltis_channels::Result<Vec<moltis_channels::message_log::SenderSummary>>
            {
                Ok(vec![])
            }

            async fn delete_before(&self, _: i64) -> moltis_channels::Result<u64> {
                Ok(0)
            }
        }

        let config = DataRetentionConfig {
            enabled: true,
            dry_run: false,
            crm_interactions_days: Some(1),
            ..Default::default()
        };

        let engine = RetentionEngine::new(
            config,
            None,
            crm.clone() as Arc<dyn CrmStore>,
            Arc::new(NoopLog) as Arc<dyn MessageLog>,
            session_store,
        );

        let report = engine.run_once().await;
        assert_eq!(
            report.crm_interactions_deleted, 1,
            "ancient record should be deleted"
        );

        assert!(
            crm.get_interaction(&old.id).await.unwrap().is_none(),
            "old gone"
        );
        assert!(
            crm.get_interaction(&recent.id).await.unwrap().is_some(),
            "recent kept"
        );
    }
}
