//! The registry of deck formats, and what reading and writing one yields.
//!
//! A format is a module under `formats/` implementing [`Format`], and one
//! entry in [`FORMATS`]. The rest of the crate — detection, import, export —
//! only ever goes through this registry, so a new format is those two things
//! and nothing else.

use crate::document::{CardError, Document};
use crate::formats;

/// The formats this build reads and writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FormatId {
    /// Baylee's own text: `docs/deck-format.md` §"Text format", the
    /// `deckrow` grammar with `SB:`/`CMD:`/`MB:` prefixes.
    Baylee,
    /// The versioned JSON document.
    Json,
    /// The same document as YAML.
    Yaml,
    /// Moxfield's text export ("Copy for Moxfield"), and the plain deck
    /// lists other sites write in the same shape.
    Moxfield,
}

impl FormatId {
    /// Every format, in the order a chooser offers them.
    pub const ALL: [Self; 4] = [Self::Baylee, Self::Json, Self::Yaml, Self::Moxfield];

    /// A stable lower-case key, for settings and file names.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Baylee => "baylee",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Moxfield => "moxfield",
        }
    }

    /// The format a key names.
    #[must_use]
    pub fn of_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.key().eq_ignore_ascii_case(key))
    }

    /// The file extension a saved export takes.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::Baylee | Self::Moxfield => "txt",
            Self::Json => "json",
            Self::Yaml => "yaml",
        }
    }

    /// The media type of a saved export.
    #[must_use]
    pub fn media_type(self) -> &'static str {
        match self {
            Self::Baylee | Self::Moxfield => "text/plain",
            Self::Json => "application/json",
            Self::Yaml => "application/yaml",
        }
    }

    /// Whether this format can say every field of a [`Document`].
    ///
    /// The structured formats are the document through serde; the text
    /// formats say what a row can say, and [`Written::losses`] names what
    /// they could not.
    #[must_use]
    pub fn lossless(self) -> bool {
        matches!(self, Self::Json | Self::Yaml)
    }
}

/// How sure a format is that a text is its own.
///
/// Ordered: detection takes the highest, and ties go to the format listed
/// first in [`FORMATS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sniff {
    /// Not this format.
    No,
    /// Could be: the fallback reading of a plain deck list.
    Plain,
    /// Something in the text points here.
    Likely,
    /// The text says so itself: a header, or a structure only this format
    /// has.
    Certain,
}

/// One deck format.
pub trait Format: Sync {
    /// Which one it is.
    fn id(&self) -> FormatId;
    /// How sure it is that `text` is its own. Cheap: it is asked of every
    /// format for every paste.
    fn sniff(&self, text: &str) -> Sniff;
    /// Reads a document.
    ///
    /// # Errors
    /// [`ReadError`] when the text is not this format, or says something no
    /// deck may say.
    fn read(&self, text: &str) -> Result<Read, ReadError>;
    /// Writes a document, saying what it could not write.
    fn write(&self, document: &Document) -> Written;
}

/// Every format, in the order detection breaks ties in.
pub static FORMATS: [&dyn Format; 4] = [
    &formats::json::Json,
    &formats::yaml::Yaml,
    &formats::baylee::Baylee,
    &formats::moxfield::Moxfield,
];

/// The format behind an id.
#[must_use]
pub fn get(id: FormatId) -> &'static dyn Format {
    FORMATS
        .iter()
        .copied()
        .find(|format| format.id() == id)
        .unwrap_or(&formats::baylee::Baylee)
}

/// The format surest that `text` is its own, or `None` for a text that holds
/// nothing at all.
#[must_use]
pub fn detect(text: &str) -> Option<FormatId> {
    if text.trim().is_empty() {
        return None;
    }
    let mut best: Option<(Sniff, FormatId)> = None;
    for format in FORMATS {
        let sniff = format.sniff(text);
        if sniff > Sniff::No && best.is_none_or(|(held, _)| sniff > held) {
            best = Some((sniff, format.id()));
        }
    }
    best.map(|(_, id)| id)
}

/// Reads `text` as one format, bounded and checked.
///
/// # Errors
/// [`ReadError`] as [`Format::read`], and when the text or the document it
/// held is over a limit.
pub fn read(id: FormatId, text: &str) -> Result<Read, ReadError> {
    if text.len() > crate::MAX_DOCUMENT_BYTES {
        return Err(ReadError::TooLarge { bytes: text.len() });
    }
    let read = get(id).read(text)?;
    read.document.validate()?;
    if read.document.cards.is_empty() {
        return Err(match read.skipped.first() {
            Some(first) => ReadError::Unreadable(first.clone()),
            None => ReadError::Empty,
        });
    }
    Ok(read)
}

/// A document read, and what of the text was not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// The deck.
    pub document: Document,
    /// Lines of a text format that held no row it could read. Reported,
    /// never dropped unseen: a line the reader skipped is a card the player
    /// may think they imported.
    pub skipped: Vec<Skipped>,
}

/// A line a text reader could not take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    /// 1-based line number.
    pub line: usize,
    /// The line as written, cut to a length worth showing.
    pub text: String,
}

impl Skipped {
    /// The most of a skipped line kept for the report, in characters.
    pub const SHOWN: usize = 80;

    pub(crate) fn new(line: usize, text: &str) -> Self {
        Self {
            line,
            text: text.trim().chars().take(Self::SHOWN).collect(),
        }
    }
}

/// A document written, and what the format could not say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    /// The text.
    pub text: String,
    /// What was left out or said differently, one entry per kind. Empty
    /// means the text says everything the document did.
    pub losses: Vec<Loss>,
}

/// One kind of thing a format could not write, and how many rows it hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loss {
    /// What.
    pub kind: LossKind,
    /// On how many rows (1 for the deck-wide kinds).
    pub rows: usize,
}

/// What a format could not write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LossKind {
    /// The deck's name.
    Name,
    /// A language.
    Lang,
    /// An exact printing id.
    ScryfallId,
    /// A note.
    Note,
    /// A finish the format has no marker for (Moxfield writes only foil and
    /// etched). The row is written with no marker, which the format reads as
    /// non-foil.
    Finish,
    /// A collector number with no set in front of it, which a text row
    /// cannot tell apart from the end of the card's name.
    CollectorNumber,
    /// The maybeboard, which the format has no section for.
    Maybeboard,
}

/// Tallies losses by kind, in [`LossKind`] order.
#[derive(Default)]
pub(crate) struct Tally(std::collections::BTreeMap<LossKind, usize>);

impl Tally {
    pub(crate) fn add(&mut self, kind: LossKind) {
        *self.0.entry(kind).or_default() += 1;
    }

    pub(crate) fn losses(self) -> Vec<Loss> {
        self.0
            .into_iter()
            .map(|(kind, rows)| Loss { kind, rows })
            .collect()
    }
}

/// Why a text could not be read as a deck.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// Over [`crate::MAX_DOCUMENT_BYTES`].
    #[error("a deck file is at most 256 KiB; this is {bytes} bytes")]
    TooLarge {
        /// How large it was.
        bytes: usize,
    },
    /// Over [`crate::MAX_ROWS`].
    #[error("a deck file holds at most 1000 rows; this holds {0}")]
    TooManyRows(usize),
    /// Nothing but blank lines and comments.
    #[error("there is no deck in this text")]
    Empty,
    /// Text, but not one line of it a deck row.
    #[error("line {}: not a deck row: {}", .0.line, .0.text)]
    Unreadable(Skipped),
    /// A JSON or YAML document that did not parse, in its parser's words.
    #[error("{0}")]
    Syntax(String),
    /// A document of a version this build does not read.
    #[error("deck document version {0} is not one this build reads (it reads version 1)")]
    Version(u32),
    /// A deck name that is too long or holds a control character.
    #[error("the deck's name is too long or holds a line break")]
    DeckName,
    /// One card that cannot be taken.
    #[error("card {}: {why}", .at + 1)]
    Card {
        /// 0-based position in the document.
        at: usize,
        /// Why.
        why: CardError,
    },
}

impl ReadError {
    /// A parser's error, cut to its first line: YAML's carries a drawn
    /// excerpt of the text under it, which is for a terminal, not a dialog.
    pub(crate) fn syntax(error: &dyn std::fmt::Display) -> Self {
        let said = error.to_string();
        Self::Syntax(said.lines().next().unwrap_or_default().trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_is_in_the_registry_once() {
        for id in FormatId::ALL {
            assert_eq!(FORMATS.iter().filter(|f| f.id() == id).count(), 1, "{id:?}");
            assert_eq!(get(id).id(), id);
            assert_eq!(FormatId::of_key(id.key()), Some(id));
        }
    }

    #[test]
    fn an_empty_text_has_no_format() {
        assert_eq!(detect(""), None);
        assert_eq!(detect("  \n\n "), None);
    }

    #[test]
    fn a_key_is_found_in_any_case_and_a_stranger_is_none() {
        assert_eq!(FormatId::of_key("JSON"), Some(FormatId::Json));
        assert_eq!(FormatId::of_key("Moxfield"), Some(FormatId::Moxfield));
        assert_eq!(FormatId::of_key("csv"), None);
        assert_eq!(FormatId::of_key(""), None);
    }

    #[test]
    fn only_json_and_yaml_say_everything_and_each_names_a_file_type() {
        let lossless: Vec<FormatId> = FormatId::ALL.into_iter().filter(|f| f.lossless()).collect();
        assert_eq!(lossless, vec![FormatId::Json, FormatId::Yaml]);
        assert_eq!(FormatId::Json.extension(), "json");
        assert_eq!(FormatId::Yaml.extension(), "yaml");
        assert_eq!(FormatId::Baylee.extension(), "txt");
        assert_eq!(FormatId::Moxfield.media_type(), "text/plain");
        assert_eq!(FormatId::Json.media_type(), "application/json");
        assert_eq!(FormatId::Yaml.media_type(), "application/yaml");
    }

    #[test]
    fn a_parsers_error_is_cut_to_its_first_line() {
        let said = ReadError::syntax(&"expected a value\n  1 | {\n    ^ here");
        assert_eq!(said, ReadError::Syntax("expected a value".into()));
        assert_eq!(ReadError::syntax(&""), ReadError::Syntax(String::new()));
    }

    #[test]
    fn a_skipped_line_is_trimmed_and_cut_to_a_length_worth_showing() {
        let skipped = Skipped::new(4, &format!("  {}  ", "x".repeat(200)));
        assert_eq!(skipped.line, 4);
        assert_eq!(skipped.text.chars().count(), Skipped::SHOWN);
        assert!(!skipped.text.starts_with(' '));
    }

    #[test]
    fn losses_are_counted_by_kind_in_kind_order() {
        let mut tally = Tally::default();
        tally.add(LossKind::Note);
        tally.add(LossKind::Name);
        tally.add(LossKind::Note);
        assert_eq!(
            tally.losses(),
            vec![
                Loss {
                    kind: LossKind::Name,
                    rows: 1
                },
                Loss {
                    kind: LossKind::Note,
                    rows: 2
                },
            ]
        );
        assert!(Tally::default().losses().is_empty());
    }

    #[test]
    fn a_read_error_says_where_and_why() {
        assert_eq!(
            ReadError::Card {
                at: 2,
                why: CardError::Count
            }
            .to_string(),
            "card 3: a card needs a copy count from 1 to 1000000"
        );
        assert_eq!(
            ReadError::Unreadable(Skipped::new(7, "hello")).to_string(),
            "line 7: not a deck row: hello"
        );
        assert_eq!(
            ReadError::TooLarge { bytes: 9 }.to_string(),
            "a deck file is at most 256 KiB; this is 9 bytes"
        );
    }
}
