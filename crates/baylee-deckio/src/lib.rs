//! Decks in and out of Baylee: one document model, a registry of formats, a
//! registry of URL sources.
//!
//! `docs/deck-format.md` is the specification; this crate is it.
//!
//! # The shape
//!
//! - [`Document`] is the one model every format reads into and writes from:
//!   a versioned list of [`Card`]s, each with a [`Zone`], a count, a name and
//!   every printing field `baylee_core::deckrow` knows. [`Document::stored`]
//!   turns it into what a saved deck *is* — `deckrow::Row`s for the main deck
//!   and the sideboard plus the commanders by name — and
//!   [`Document::from_stored`] goes back. Nothing here resolves a name
//!   against the card pool or a printing against a catalog: that happens
//!   where saved decks are resolved today, so an imported deck and a stored
//!   one cannot resolve differently.
//! - [`format`] is the registry of formats. Each lives in its own module
//!   under `formats/`, implements [`Format`], and says how sure it is that a
//!   text is its own ([`Format::sniff`]); [`import`] asks all of them and
//!   reads with the surest. Adding a format is one module and one line in
//!   [`format::FORMATS`].
//! - [`source`] is the registry of places a deck can be *linked* from. Given
//!   a URL a [`source::Source`] answers either with a plan to fetch it (a GET
//!   and the format that reads the body) or with an instruction for the
//!   player. Moxfield has no public API and sits behind bot protection, so
//!   its answer is the instruction; nothing in this crate touches a network.
//!
//! # What a stranger can send
//!
//! A pasted deck is untrusted text, and a deck is a thing a stranger can
//! `POST`. So every reader is bounded before it starts ([`MAX_DOCUMENT_BYTES`])
//! and after it finishes ([`MAX_ROWS`], and each row through the same
//! `deckrow` limits a stored row is held to).
//!
//! No renderer, no async, no network, no Bevy: this builds for the browser.

pub mod document;
pub mod format;
mod formats;
pub mod source;

pub use document::{Card, Document, Stored, VERSION, Zone};
pub use format::{Format, FormatId, Loss, LossKind, Read, ReadError, Skipped, Written};

/// The largest text an import reads, in bytes.
///
/// A 250-row deck with a full note on every row is about 140 KB as text and
/// less as JSON; this is that with room, and far short of a size that would
/// cost a frame to scan.
pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;

/// The most rows one document may carry, across every zone.
///
/// The gateway stores at most 250 lines per list; this is four lists' worth,
/// so a real deck with a maybeboard always fits, and a document claiming a
/// million rows is refused before anything is built from it.
pub const MAX_ROWS: usize = 1_000;

/// The longest deck name a document may carry, in characters.
///
/// Longer than the gateway's 64 bytes on purpose: a name that is too long for
/// the gateway is the builder's to say, in the words it already has; a name
/// that is a paragraph is refused here.
pub const MAX_NAME: usize = 200;

/// What a paste turned out to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Import {
    /// A link to a deck somewhere, and what to do about it.
    Source(source::Recognised),
    /// A link to a site no source knows, by its host.
    UnknownLink(String),
    /// A deck, read by the format that recognised it.
    Read {
        /// The format that read it.
        format: FormatId,
        /// What it read.
        read: Read,
    },
}

/// Reads whatever was pasted: a link a [`source`] recognises, or a deck in
/// whichever [`format`] is surest it is its own.
///
/// # Errors
/// [`ReadError`] when the text is too large, empty, or no format can read it.
pub fn import(text: &str) -> Result<Import, ReadError> {
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err(ReadError::TooLarge { bytes: text.len() });
    }
    if let Some(found) = source::recognise(text) {
        return Ok(Import::Source(found));
    }
    if let Some(host) = source::unknown_link(text) {
        return Ok(Import::UnknownLink(host));
    }
    let format = format::detect(text).ok_or(ReadError::Empty)?;
    let read = format::read(format, text)?;
    Ok(Import::Read { format, read })
}

/// Writes a document in one format.
#[must_use]
pub fn export(format: FormatId, document: &Document) -> Written {
    format::get(format).write(document)
}
