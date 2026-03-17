use std::error::Error as StdError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("record not found: {id}")]
    NotFound { id: String },
    #[error("invalid value for {field}: {value:?}")]
    Parse { field: &'static str, value: String },
    #[error("{context}: {source}")]
    External {
        context: String,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },
}

impl Error {
    #[must_use]
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound { id: id.into() }
    }

    #[must_use]
    pub fn parse(field: &'static str, value: impl Into<String>) -> Self {
        Self::Parse {
            field,
            value: value.into(),
        }
    }

    #[must_use]
    pub fn external<E>(context: impl Into<String>, source: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::External {
            context: context.into(),
            source: Box::new(source),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
