use thiserror::Error;

#[derive(Error, Debug)]
pub enum RssyError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Feed parsing error: {0}")]
    FeedParsing(#[from] feed_rs::parser::ParseFeedError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RssyError>;
