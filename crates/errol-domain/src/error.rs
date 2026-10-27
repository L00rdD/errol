use thiserror::Error;

/// Every failure the domain knows of. Adapters convert their own errors into these.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// The feed could not be read (service down, bad JSON).
    #[error("feed unavailable: {0}")]
    FeedUnavailable(String),
    /// A picture could not be fetched or decoded.
    #[error("picture unavailable: {0}")]
    PictureUnavailable(String),
}
