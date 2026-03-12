use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum SymphonyError {
    #[error("missing workflow file: {path}")]
    MissingWorkflowFile { path: PathBuf },

    #[error("workflow parse error: {message}")]
    WorkflowParseError { message: String },

    #[error("workflow front matter must decode to a map")]
    WorkflowFrontMatterNotAMap,

    #[error("template parse error: {message}")]
    TemplateParseError { message: String },

    #[error("template render error: {message}")]
    TemplateRenderError { message: String },

    #[error("unsupported tracker kind: {kind}")]
    UnsupportedTrackerKind { kind: String },

    #[error("missing tracker api key")]
    MissingTrackerApiKey,

    #[error("missing tracker project slug")]
    MissingTrackerProjectSlug,

    #[error("missing codex command")]
    MissingCodexCommand,

    #[error("tracker request failed: {message}")]
    TrackerRequest { message: String },

    #[error("tracker API returned non-success status: {status}")]
    TrackerStatus { status: u16 },

    #[error("tracker GraphQL errors: {message}")]
    TrackerGraphql { message: String },

    #[error("tracker response payload was malformed")]
    TrackerPayload,

    #[error("invalid config for `{field}`: {message}")]
    InvalidConfig {
        field: &'static str,
        message: String,
    },

    #[error("workspace path escapes root: root={root} path={path}")]
    WorkspacePathEscape { root: PathBuf, path: PathBuf },

    #[error("workspace path exists but is not a directory: {path}")]
    WorkspacePathNotDirectory { path: PathBuf },

    #[error("hook `{name}` failed: {message}")]
    HookFailed { name: &'static str, message: String },

    #[error("hook `{name}` timed out after {timeout_ms} ms")]
    HookTimedOut { name: &'static str, timeout_ms: u64 },

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),

    #[error(transparent)]
    Notify(#[from] notify_debouncer_full::notify::Error),
}

pub type Result<T, E = SymphonyError> = std::result::Result<T, E>;
