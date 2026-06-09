use thiserror::Error;

#[derive(Debug, Error)]
pub enum BookError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    #[error("a book with id {0} already exists")]
    DuplicateId(String),

    #[error("could not locate a config directory")]
    NoConfigDir,
}

pub type Result<T> = std::result::Result<T, BookError>;
