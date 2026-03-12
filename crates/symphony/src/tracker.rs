use {
    reqwest::header::{AUTHORIZATION, CONTENT_TYPE},
    secrecy::ExposeSecret,
    serde::Deserialize,
    time::{OffsetDateTime, format_description::well_known::Rfc3339},
};

use crate::{
    config::TrackerConfig,
    error::{Result, SymphonyError},
    models::Issue,
};

const CANDIDATE_QUERY: &str = r#"
query SymphonyCandidateIssues($projectSlug: String!, $states: [String!], $after: String) {
  issues(
    first: 50
    after: $after
    filter: {
      project: { slugId: { eq: $projectSlug } }
      state: { name: { in: $states } }
    }
  ) {
    nodes {
      id
      identifier
      title
      description
      priority
      url
      branchName
      createdAt
      updatedAt
      state { name }
      labels { nodes { name } }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
"#;

const STATES_BY_IDS_QUERY: &str = r#"
query SymphonyIssueStatesByIds($ids: [ID!]) {
  issues(filter: { id: { in: $ids } }) {
    nodes {
      id
      identifier
      title
      description
      priority
      url
      branchName
      createdAt
      updatedAt
      state { name }
      labels { nodes { name } }
    }
  }
}
"#;

#[derive(Debug, Clone)]
pub struct LinearTrackerClient {
    http: reqwest::Client,
}

impl LinearTrackerClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| SymphonyError::TrackerRequest {
                message: error.to_string(),
            })?;
        Ok(Self { http })
    }

    pub async fn fetch_candidate_issues(&self, config: &TrackerConfig) -> Result<Vec<Issue>> {
        let mut issues = Vec::new();
        let mut after: Option<String> = None;

        loop {
            let payload = serde_json::json!({
                "query": CANDIDATE_QUERY,
                "variables": {
                    "projectSlug": config.project_slug.clone().unwrap_or_default(),
                    "states": &config.active_states,
                    "after": after,
                }
            });
            let response: GraphQlEnvelope<CandidateData> =
                self.post_graphql(config, payload).await?;
            let response = response.into_result()?;
            let data = response.data.ok_or(SymphonyError::TrackerPayload)?;
            issues.extend(data.issues.nodes.into_iter().map(Issue::from));

            if !data.issues.page_info.has_next_page {
                break;
            }

            after = data.issues.page_info.end_cursor;
            if after.is_none() {
                return Err(SymphonyError::TrackerPayload);
            }
        }

        Ok(issues)
    }

    pub async fn fetch_issues_by_states(
        &self,
        config: &TrackerConfig,
        states: &[String],
    ) -> Result<Vec<Issue>> {
        let mut cloned = config.clone();
        cloned.active_states = states.to_vec();
        self.fetch_candidate_issues(&cloned).await
    }

    pub async fn fetch_issue_states_by_ids(
        &self,
        config: &TrackerConfig,
        ids: &[String],
    ) -> Result<Vec<Issue>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let payload = serde_json::json!({
            "query": STATES_BY_IDS_QUERY,
            "variables": {
                "ids": ids,
            }
        });
        let response: GraphQlEnvelope<StatesByIdData> = self.post_graphql(config, payload).await?;
        let response = response.into_result()?;
        let data = response.data.ok_or(SymphonyError::TrackerPayload)?;
        Ok(data.issues.nodes.into_iter().map(Issue::from).collect())
    }

    async fn post_graphql<T>(&self, config: &TrackerConfig, payload: serde_json::Value) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or(SymphonyError::MissingTrackerApiKey)?;
        let response = self
            .http
            .post(&config.endpoint)
            .header(AUTHORIZATION, api_key.expose_secret())
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|error| SymphonyError::TrackerRequest {
                message: error.to_string(),
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(SymphonyError::TrackerStatus {
                status: status.as_u16(),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|error| SymphonyError::TrackerRequest {
                message: error.to_string(),
            })
    }
}

#[derive(Debug, Deserialize)]
struct GraphQlEnvelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphQlError>,
}

impl<T> GraphQlEnvelope<T> {
    fn into_result(self) -> Result<Self> {
        if self.errors.is_empty() {
            Ok(self)
        } else {
            Err(SymphonyError::TrackerGraphql {
                message: self
                    .errors
                    .into_iter()
                    .map(|error| error.message)
                    .collect::<Vec<_>>()
                    .join("; "),
            })
        }
    }
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct CandidateData {
    issues: LinearIssueConnection,
}

#[derive(Debug, Deserialize)]
struct StatesByIdData {
    issues: LinearIssueNodesOnly,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearIssueConnection {
    nodes: Vec<LinearIssueNode>,
    page_info: LinearPageInfo,
}

#[derive(Debug, Deserialize)]
struct LinearIssueNodesOnly {
    nodes: Vec<LinearIssueNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearPageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearIssueNode {
    id: String,
    identifier: String,
    title: String,
    description: Option<String>,
    priority: Option<i32>,
    url: Option<String>,
    branch_name: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    state: Option<LinearStateNode>,
    labels: Option<LinearLabelsConnection>,
}

#[derive(Debug, Deserialize)]
struct LinearStateNode {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LinearLabelsConnection {
    nodes: Vec<LinearLabelNode>,
}

#[derive(Debug, Deserialize)]
struct LinearLabelNode {
    name: String,
}

impl From<LinearIssueNode> for Issue {
    fn from(value: LinearIssueNode) -> Self {
        Self {
            id: value.id,
            identifier: value.identifier,
            title: value.title,
            description: value.description,
            priority: value
                .priority
                .and_then(|priority| (priority != 0).then_some(priority)),
            state: value.state.map(|state| state.name).unwrap_or_default(),
            branch_name: value.branch_name,
            url: value.url,
            labels: value
                .labels
                .map(|labels| {
                    labels
                        .nodes
                        .into_iter()
                        .map(|label| label.name.to_ascii_lowercase())
                        .collect()
                })
                .unwrap_or_default(),
            blocked_by: Vec::new(),
            created_at: parse_timestamp(value.created_at.as_deref()),
            updated_at: parse_timestamp(value.updated_at.as_deref()),
        }
    }
}

fn parse_timestamp(value: Option<&str>) -> Option<OffsetDateTime> {
    value.and_then(|inner| OffsetDateTime::parse(inner, &Rfc3339).ok())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use {super::*, secrecy::Secret};

    fn tracker_config(endpoint: String) -> TrackerConfig {
        TrackerConfig {
            kind: "linear".to_string(),
            endpoint,
            api_key: Some(Secret::new("secret".to_string())),
            project_slug: Some("moltis".to_string()),
            active_states: vec!["Todo".to_string(), "In Progress".to_string()],
            terminal_states: vec!["Done".to_string()],
        }
    }

    #[tokio::test]
    async fn fetches_candidate_issues_across_pages() {
        let mut server = mockito::Server::new_async().await;
        let page_one = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "data": {
                        "issues": {
                            "nodes": [{
                                "id": "1",
                                "identifier": "MT-1",
                                "title": "First",
                                "description": null,
                                "priority": 1,
                                "url": null,
                                "branchName": null,
                                "createdAt": "2026-03-12T10:00:00Z",
                                "updatedAt": "2026-03-12T10:05:00Z",
                                "state": { "name": "Todo" },
                                "labels": { "nodes": [{ "name": "Backend" }] }
                            }],
                            "pageInfo": {
                                "hasNextPage": true,
                                "endCursor": "cursor-1"
                            }
                        }
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;
        let page_two = server
            .mock("POST", "/")
            .with_status(200)
            .with_body(
                serde_json::json!({
                    "data": {
                        "issues": {
                            "nodes": [{
                                "id": "2",
                                "identifier": "MT-2",
                                "title": "Second",
                                "description": null,
                                "priority": 2,
                                "url": null,
                                "branchName": null,
                                "createdAt": "2026-03-12T11:00:00Z",
                                "updatedAt": "2026-03-12T11:05:00Z",
                                "state": { "name": "In Progress" },
                                "labels": { "nodes": [{ "name": "UI" }] }
                            }],
                            "pageInfo": {
                                "hasNextPage": false,
                                "endCursor": null
                            }
                        }
                    }
                })
                .to_string(),
            )
            .create_async()
            .await;

        let client = LinearTrackerClient::new().unwrap();
        let issues = client
            .fetch_candidate_issues(&tracker_config(server.url()))
            .await
            .unwrap();

        page_one.assert_async().await;
        page_two.assert_async().await;
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].labels, vec!["backend".to_string()]);
        assert_eq!(issues[1].labels, vec!["ui".to_string()]);
    }
}
