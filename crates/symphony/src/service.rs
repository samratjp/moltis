use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::{
    config::ServiceConfig,
    error::Result,
    models::{Issue, WorkflowDefinition},
    tracker::LinearTrackerClient,
    watcher::{WorkflowWatchEvent, WorkflowWatcher},
    workflow::{load_workflow, resolve_workflow_path},
};

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub workflow_path: Option<PathBuf>,
    pub once: bool,
}

#[derive(Debug, Clone)]
pub struct SymphonyRuntime {
    workflow: WorkflowDefinition,
    config: ServiceConfig,
    tracker: LinearTrackerClient,
}

#[derive(Debug, Clone)]
pub struct PollSummary {
    pub candidate_count: usize,
    pub identifiers: Vec<String>,
}

impl SymphonyRuntime {
    pub fn load(workflow_path: Option<PathBuf>) -> Result<Self> {
        let path = resolve_workflow_path(workflow_path.as_deref());
        let workflow = load_workflow(&path)?;
        let config = ServiceConfig::from_workflow(&workflow)?;
        let tracker = LinearTrackerClient::new()?;
        Ok(Self {
            workflow,
            config,
            tracker,
        })
    }

    #[must_use]
    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }

    #[must_use]
    pub fn workflow(&self) -> &WorkflowDefinition {
        &self.workflow
    }

    pub fn reload(&mut self) -> Result<()> {
        let workflow = load_workflow(&self.workflow.path)?;
        let config = ServiceConfig::from_workflow(&workflow)?;
        self.workflow = workflow;
        self.config = config;
        Ok(())
    }

    pub async fn poll_once(&self) -> Result<PollSummary> {
        let issues = self
            .tracker
            .fetch_candidate_issues(&self.config.tracker)
            .await?;
        let sorted = sort_issues_for_dispatch(issues);

        Ok(PollSummary {
            candidate_count: sorted.len(),
            identifiers: sorted.into_iter().map(|issue| issue.identifier).collect(),
        })
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
pub async fn run_service(options: RunOptions) -> anyhow::Result<()> {
    let mut runtime = SymphonyRuntime::load(options.workflow_path)?;
    tracing::info!(
        workflow_path = %runtime.workflow().path.display(),
        poll_interval_ms = runtime.config().polling.interval_ms,
        workspace_root = %runtime.config().workspace.root.display(),
        "symphony runtime ready"
    );

    if options.once {
        let summary = runtime.poll_once().await?;
        tracing::info!(
            candidate_count = summary.candidate_count,
            identifiers = ?summary.identifiers,
            "symphony poll cycle completed"
        );
        return Ok(());
    }

    let (_watcher, mut rx) = WorkflowWatcher::start(runtime.workflow().path.clone())?;
    run_loop(&mut runtime, &mut rx).await?;
    Ok(())
}

async fn run_loop(
    runtime: &mut SymphonyRuntime,
    rx: &mut mpsc::UnboundedReceiver<WorkflowWatchEvent>,
) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(
        runtime.config().polling.interval_ms,
    ));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("symphony shutdown requested");
                return Ok(());
            }
            Some(WorkflowWatchEvent::Changed) = rx.recv() => {
                match runtime.reload() {
                    Ok(()) => {
                        tracing::info!(
                            workflow_path = %runtime.workflow().path.display(),
                            poll_interval_ms = runtime.config().polling.interval_ms,
                            "reloaded symphony workflow"
                        );
                        interval = tokio::time::interval(std::time::Duration::from_millis(
                            runtime.config().polling.interval_ms,
                        ));
                        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "invalid workflow reload ignored");
                    }
                }
            }
            _ = interval.tick() => {
                match runtime.poll_once().await {
                    Ok(summary) => {
                        tracing::info!(
                            workflow_path = %runtime.workflow().path.display(),
                            poll_interval_ms = runtime.config().polling.interval_ms,
                            candidate_count = summary.candidate_count,
                            identifiers = ?summary.identifiers,
                            "symphony poll tick completed"
                        );
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "symphony poll tick failed");
                    }
                }
            }
        }
    }
}

#[must_use]
pub fn sort_issues_for_dispatch(mut issues: Vec<Issue>) -> Vec<Issue> {
    issues.sort_by(|left, right| {
        let left_priority = left.priority.unwrap_or(i32::MAX);
        let right_priority = right.priority.unwrap_or(i32::MAX);

        left_priority
            .cmp(&right_priority)
            .then_with(|| compare_created_at(left.created_at, right.created_at))
            .then_with(|| left.identifier.cmp(&right.identifier))
    });
    issues
}

fn compare_created_at(
    left: Option<time::OffsetDateTime>,
    right: Option<time::OffsetDateTime>,
) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use {super::*, time::OffsetDateTime};

    #[test]
    fn invalid_reload_keeps_last_good_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("WORKFLOW.md");
        std::fs::write(
            &path,
            "---\ntracker:\n  kind: linear\n  api_key: secret\n  project_slug: moltis\n---\nHello {{ issue.title }}\n",
        )
        .unwrap();

        let mut runtime = SymphonyRuntime::load(Some(path.clone())).unwrap();
        std::fs::write(&path, "---\n- invalid\n---\nBroken\n").unwrap();

        assert!(runtime.reload().is_err());
        assert_eq!(runtime.config().tracker.kind, "linear");
        assert_eq!(
            runtime.workflow().prompt_template,
            "Hello {{ issue.title }}"
        );
    }

    #[test]
    fn sorts_issues_by_priority_then_created_at_then_identifier() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let later = now + time::Duration::minutes(5);
        let issues = vec![
            Issue {
                id: "2".to_string(),
                identifier: "MT-20".to_string(),
                title: "B".to_string(),
                priority: Some(2),
                created_at: Some(later),
                ..Issue::default()
            },
            Issue {
                id: "1".to_string(),
                identifier: "MT-10".to_string(),
                title: "A".to_string(),
                priority: Some(1),
                created_at: Some(later),
                ..Issue::default()
            },
            Issue {
                id: "3".to_string(),
                identifier: "MT-11".to_string(),
                title: "C".to_string(),
                priority: Some(1),
                created_at: Some(now),
                ..Issue::default()
            },
        ];

        let sorted = sort_issues_for_dispatch(issues);
        assert_eq!(
            sorted
                .into_iter()
                .map(|issue| issue.identifier)
                .collect::<Vec<_>>(),
            vec![
                "MT-11".to_string(),
                "MT-10".to_string(),
                "MT-20".to_string()
            ]
        );
    }
}
