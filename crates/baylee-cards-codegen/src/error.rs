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
    /// A `CardIndex` ledger row claiming an index, a card or a constant that
    /// an earlier row already holds.
    #[error("card-index ledger row {line}: {text}")]
    LedgerLine {
        /// 1-based position in the table.
        line: usize,
        /// What the row claimed.
        text: String,
    },
    /// A printed `Partner with <name>` naming no card, or naming several.
    ///
    /// The one place a card names another card, so the one place a printed
    /// name has to be resolved against the ledger rather than merely copied.
    #[error("`Partner with {name}`: {reason}")]
    PartnerName {
        /// The name as the printed line gave it.
        name: String,
        /// Why no single row answers for it.
        reason: String,
    },
    /// A card name carries a letter the slug table has no ASCII answer for.
    ///
    /// Refused rather than dropped, because the slug becomes a constant's
    /// name in the ledger and is frozen there: `Barad-dûr` shipped as
    /// `barad_dr` for as long as the table said nothing about `û`.
    #[error("no ASCII spelling for '{letter}' in '{name}' — add it to stubgen::TRANSLITERATE")]
    Untransliterable {
        /// The card.
        name: String,
        /// The letter with no answer.
        letter: char,
    },
    /// Two cards want one constant and no tie-break freed it.
    #[error("constant {constant} is claimed by both '{held}' and '{wanted}'")]
    ConstantCollision {
        /// The contested name.
        constant: String,
        /// The card that holds it.
        held: String,
        /// The card that wanted it.
        wanted: String,
    },
    /// Two cards in the pool print the same English name.
    ///
    /// There is no answer `by_name` could give for such a name, and the
    /// perfect hash cannot be built over the pair at all — identical keys
    /// hash identically and no displacement separates them. Named here so
    /// the failure says which card rather than "no seed worked".
    #[error("two cards are named '{name}' (entries {first} and {again})")]
    DuplicateCardName {
        /// The contested name.
        name: String,
        /// Position of the first card with it.
        first: usize,
        /// Position of the second.
        again: usize,
    },
    /// No seed placed the name set in a perfect hash.
    ///
    /// Not expected to happen: the table is two thirds full and a bucket may
    /// be displaced a million times. It is a bail rather than an `expect`
    /// because the alternative is a table that silently lost a card.
    #[error("no perfect hash for {keys} card names in {seeds} seeds")]
    NoPerfectHash {
        /// How many names were being placed.
        keys: usize,
        /// How many seeds were tried.
        seeds: u64,
    },
    /// The pool outgrew the slot table's `u16`.
    ///
    /// Unreachable at 1365 cards and checked anyway, because the failure it
    /// prevents is a position that wraps into the empty marker — a card that
    /// silently stops resolving rather than a build that stops.
    #[error("the pool has {cards} names; the slot table holds at most {limit}")]
    PoolTooLarge {
        /// How many names were being placed.
        cards: usize,
        /// What fits.
        limit: usize,
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
