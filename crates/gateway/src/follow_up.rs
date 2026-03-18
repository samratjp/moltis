//! Follow-up engine — prompt enrichment for the `__follow_up__` cron job.
//!
//! Responsible for converting [`StaleContactInfo`] records from the CRM store
//! into a PII-safe prompt that an agent can use to summarise who needs follow-up.
//! Email and phone are never included; only name, stage, last-interaction age,
//! and matter title are exposed to the LLM.

use moltis_crm::store::StaleContactInfo;

/// Session name used for the follow-up cron job.
///
/// Shared between job registration and the `on_agent_turn` callback to ensure
/// the session is detected correctly without risk of typos.
pub const FOLLOW_UP_SESSION_NAME: &str = "follow_up";

/// Fixed ID for the system follow-up cron job.
pub const FOLLOW_UP_JOB_ID: &str = "__follow_up__";

/// Maximum number of stale contacts included in a single follow-up prompt.
pub const MAX_CONTACTS_IN_PROMPT: usize = 50;

/// Default prompt used when no custom prompt is configured.
pub const DEFAULT_PROMPT: &str = "Review the stale contacts listed below and compose a concise \
summary of who needs follow-up. For each contact include their name, how long since the last \
interaction, their current stage, and the most recent matter if available. Keep the summary \
brief and actionable.";

/// PII-safe summary of a stale contact for the follow-up prompt.
///
/// Constructed from [`StaleContactInfo`] — intentionally excludes email and
/// phone so no PII is passed to the LLM.
#[derive(Debug, Clone)]
pub struct StaleContactSummary {
    /// Contact's display name.
    pub name: String,
    /// Contact's current lifecycle stage (e.g. "lead", "active").
    pub stage: String,
    /// Number of days since the last interaction, or `None` if there has never
    /// been an interaction with this contact.
    pub days_since_interaction: Option<u64>,
    /// Title of the most recently updated matter for this contact, if any.
    pub matter_title: Option<String>,
}

impl From<StaleContactInfo> for StaleContactSummary {
    fn from(info: StaleContactInfo) -> Self {
        let days_since_interaction = info.last_interaction_at.map(|ts_ms| {
            let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1_000;
            now_ms.saturating_sub(ts_ms) / (24 * 3_600 * 1_000)
        });
        Self {
            name: info.contact_name,
            stage: info.stage.to_string(),
            days_since_interaction,
            matter_title: info.matter_title,
        }
    }
}

/// Build a follow-up prompt with stale contacts injected.
///
/// Never includes PII (no email or phone). Caps the contact list at
/// [`MAX_CONTACTS_IN_PROMPT`] entries and appends an overflow notice when
/// the full list is truncated.
#[must_use]
pub fn build_followup_prompt(base_prompt: &str, contacts: &[StaleContactSummary]) -> String {
    if contacts.is_empty() {
        return format!("{base_prompt}\n\nNo contacts require follow-up at this time.");
    }

    let mut prompt = format!("{base_prompt}\n\n## Contacts Needing Follow-up\n\n");

    let shown = contacts.len().min(MAX_CONTACTS_IN_PROMPT);
    for contact in contacts.iter().take(shown) {
        prompt.push_str(&format!("- **{}**", contact.name));
        prompt.push_str(&format!(" (stage: {})", contact.stage));
        match contact.days_since_interaction {
            Some(days) => prompt.push_str(&format!(", last contact: {days} days ago")),
            None => prompt.push_str(", no prior contact recorded"),
        }
        if let Some(ref title) = contact.matter_title {
            prompt.push_str(&format!(", matter: {title}"));
        }
        prompt.push('\n');
    }

    if contacts.len() > shown {
        prompt.push_str(&format!(
            "\n…and {} more contacts not shown (listing most stale first).\n",
            contacts.len() - shown
        ));
    }

    prompt
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_contact(
        name: &str,
        stage: &str,
        days: Option<u64>,
        matter: Option<&str>,
    ) -> StaleContactSummary {
        StaleContactSummary {
            name: name.to_owned(),
            stage: stage.to_owned(),
            days_since_interaction: days,
            matter_title: matter.map(String::from),
        }
    }

    #[test]
    fn prompt_formats_correctly() {
        let contacts = vec![
            make_contact("Alice Smith", "lead", Some(20), Some("Contract Review")),
            make_contact("Bob Jones", "prospect", Some(15), None),
        ];
        let prompt = build_followup_prompt("Review stale contacts.", &contacts);
        assert!(prompt.contains("Alice Smith"));
        assert!(prompt.contains("lead"));
        assert!(prompt.contains("20 days ago"));
        assert!(prompt.contains("Contract Review"));
        assert!(prompt.contains("Bob Jones"));
        assert!(prompt.contains("15 days ago"));
    }

    #[test]
    fn prompt_excludes_pii() {
        // Confirm email- and phone-shaped content never appears in the prompt.
        let contacts = vec![make_contact("Alice", "active", Some(30), None)];
        let prompt = build_followup_prompt("Base prompt.", &contacts);
        assert!(!prompt.contains('@'));
        // '+' could appear in phone numbers
        assert!(!prompt.contains('+'));
    }

    #[test]
    fn prompt_handles_empty_list() {
        let prompt = build_followup_prompt("Base.", &[]);
        assert!(prompt.contains("No contacts require follow-up"));
    }

    #[test]
    fn prompt_caps_large_lists() {
        let contacts: Vec<StaleContactSummary> = (0..60_usize)
            .map(|i| make_contact(&format!("Contact{i}"), "lead", Some(i as u64), None))
            .collect();
        let prompt = build_followup_prompt("Base.", &contacts);
        assert!(prompt.contains("10 more contacts not shown"));
    }

    #[test]
    fn prompt_handles_no_prior_contact() {
        let contacts = vec![make_contact("New Person", "lead", None, None)];
        let prompt = build_followup_prompt("Base.", &contacts);
        assert!(prompt.contains("no prior contact recorded"));
    }
}
