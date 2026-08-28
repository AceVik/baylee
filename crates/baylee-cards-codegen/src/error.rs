//! Error type for code generation.

use std::path::PathBuf;

/// Everything that can go wrong during codegen.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// Filesystem failure.
    #[error("io error at {path}: {source}")]
    Io {
        /// The offending path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// HTTP/transport failure.
    #[error("http error for {url}: {message}")]
    Http {
        /// The requested URL.
        url: String,
        /// What went wrong.
        message: String,
    },
    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Scryfall lookup failed for a card name.
    #[error("scryfall card not found: {0}")]
    CardNotFound(String),
    /// A mana cost failed validation.
    #[error("invalid mana cost '{cost}' on {card}: {reason}")]
    Mana {
        /// Card name.
        card: String,
        /// The offending cost string.
        cost: String,
        /// Parser message.
        reason: &'static str,
    },
    /// Invalid line in the acceptance deck file.
    #[error("acceptance deck file line {line}: {text}")]
    DeckLine {
        /// 1-based line number.
        line: usize,
        /// Line content.
        text: String,
    },
}

impl CodegenError {
    /// Wraps an [`std::io::Error`] with its path.
    pub fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Self {
        move |source| CodegenError::Io {
            path: path.into(),
            source,
        }
    }
}
