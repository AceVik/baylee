//! One line of a deck list, with the printing the owner chose.
//!
//! `docs/deck-format.md` specifies the text form; this is it:
//!
//! ```text
//! 4 Lightning Bolt
//! 4 Lightning Bolt (M11) 149 [de] *F* scryfall=e3285e6a-…
//! ```
//!
//! Everything after the name is optional and independent. A row that names
//! nothing is a row that says "any printing, English, not foiled" — which is
//! exactly what every deck saved before this existed says, so the old form
//! keeps parsing and keeps meaning what it meant.
//!
//! # Why a string and not a struct
//!
//! A deck is stored as lines, exported as lines and imported as lines. Making
//! the stored form the exported form means there is one parser to be wrong in,
//! and a deck that round-trips through a text file is bit-identical to the one
//! that never left. The gateway keeps `Vec<String>`; this module is what both
//! ends read it with.
//!
//! # What the printing does *not* do
//!
//! Nothing here reaches the rules. The engine sees a [`crate::ids::CardIndex`]
//! and a [`crate::ids::PrintRef`], and never looks inside the second one.
//! Two copies of a card with different finishes are the same card.

use crate::preset::Finish;
use std::fmt;

/// The most copies a row may name.
///
/// Not a rules limit — the gateway enforces those — but a parse limit: a row
/// claiming four billion copies is a denial of service, not a deck.
pub const MAX_COUNT: u32 = 1_000_000;

/// How much note one row may carry, in characters.
///
/// The same argument as [`MAX_COUNT`], for the same reason: a note is free
/// text and a deck is a thing a stranger can `POST`. Long enough for the
/// sentence somebody writes about why a card is in the deck; far short of a
/// document.
pub const MAX_NOTE: usize = 500;

/// What fences a note off from the card it is about.
///
/// A note cannot be one more trailing group, because the groups are
/// recognised by *shape* and an unrecognised trailing word deliberately stays
/// part of the name — which is what keeps `Erase (Not the Urza's Legacy One)`
/// readable, and what would eat every note that did not happen to look like a
/// set code. So it is fenced, with the marker every other deck format on the
/// internet already uses for the same job.
///
/// The fence is safe to spend, and that is measured rather than assumed: of
/// the 33 694 names in the `CardIndex` ledger — every card there is, not this
/// pool — **none contains a `#` at all**, let alone one with a space in front
/// of it. `no_card_name_could_be_mistaken_for_a_note` is that measurement as
/// a test, so a future card called `Card #1` fails the build here instead of
/// losing half its name in somebody's deck.
///
/// It is the *marker* and not the whole separator: a row is written with a
/// space after it as well (`1 Sol Ring # ramp`), and read as everything past
/// the marker, trimmed. Reading the trailing space too would make
/// `1 Sol Ring #` — a row somebody started a note on and did not — a card
/// called `Sol Ring #`.
pub const NOTE_FENCE: &str = " #";

/// A printing choice, as a deck row names it.
///
/// Every field is optional and they narrow independently: a row may say only
/// "the German one", only "foil", or name the exact printing by id. What is
/// left unsaid is resolved when the deck is loaded, and a deck that says
/// nothing at all still plays.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrintChoice {
    /// Set code, upper-case as printed (`M11`, `LEA`).
    pub set: Option<String>,
    /// Collector number within the set. A string: it may be `123a` or `★12`.
    pub collector_number: Option<String>,
    /// Two-letter language code, lower-case (`en`, `de`, `ja`).
    pub lang: Option<String>,
    /// Finish. `None` means the row did not say, which reads as non-foil.
    pub finish: Option<Finish>,
    /// The exact printing, when the row names one. This wins over everything
    /// else: it is already the answer the other fields are narrowing towards.
    pub scryfall_id: Option<String>,
}

impl PrintChoice {
    /// Whether this choice says anything at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The finish, defaulting to non-foil.
    #[must_use]
    pub fn finish_or_default(&self) -> Finish {
        self.finish.unwrap_or_default()
    }

    /// The language, defaulting to English.
    #[must_use]
    pub fn lang_or_default(&self) -> &str {
        self.lang.as_deref().unwrap_or("en")
    }
}

/// One row of a deck list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Copies.
    pub count: u32,
    /// Card name, as written. English or localized — resolving it is the
    /// caller's business, and the caller has the catalog.
    pub name: String,
    /// The printing the owner picked, as far as they picked one.
    pub print: PrintChoice,
    /// What the owner wrote about this card, after the [`NOTE_FENCE`].
    ///
    /// The deck's own business and nothing else's: no rule, no route and no
    /// renderer reads it, and it travels with the row so that a deck exported
    /// to a text file and imported again still says why the card is in it.
    pub note: Option<String>,
}

impl Row {
    /// A plain row: this many of this card, any printing.
    #[must_use]
    pub fn plain(count: u32, name: impl Into<String>) -> Self {
        Self {
            count,
            name: name.into(),
            print: PrintChoice::default(),
            note: None,
        }
    }
}

/// Why a row could not be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum RowError {
    /// No leading count, or nothing after it.
    #[error("a deck row is `N Card Name`")]
    Shape,
    /// The count is not a number, is zero, or is absurd.
    #[error("a deck row needs a copy count of 1 or more")]
    Count,
    /// A `*…*` group that is not a finish this build knows.
    #[error("unknown finish marker")]
    Finish,
    /// A `[xx]` group that is not a language code.
    #[error("a language is two or three letters")]
    Lang,
    /// A note longer than [`MAX_NOTE`].
    #[error("a card's note is at most 500 characters")]
    Note,
}

/// The marker a finish is written with, and back again.
///
/// Scryfall's own vocabulary for a printing's finishes is
/// `nonfoil` / `foil` / `etched`; the deck format's is a one-letter marker, as
/// every other deck format on the internet writes it.
fn finish_marker(finish: Finish) -> Option<&'static str> {
    match finish {
        Finish::Normal => None,
        Finish::Foil => Some("F"),
        Finish::Etched => Some("E"),
    }
}

/// Reads one finish marker.
fn parse_finish(marker: &str) -> Result<Finish, RowError> {
    match marker.to_ascii_uppercase().as_str() {
        "F" | "FOIL" => Ok(Finish::Foil),
        "E" | "ETCHED" => Ok(Finish::Etched),
        "N" | "NONFOIL" => Ok(Finish::Normal),
        _ => Err(RowError::Finish),
    }
}

/// Reads one deck row.
///
/// The name is whatever sits between the count and the first *recognised*
/// trailing group, and trailing groups are recognised by shape rather than by
/// position: `(SET) 123`, `[de]`, `*F*`, `scryfall=…`. Order between them does
/// not matter, because no two of them can be confused for each other and
/// insisting on an order would only make hand-written lists fail.
///
/// The note is cut off **first**, at the [`NOTE_FENCE`], because it is free
/// text and everything below this line reads by shape. A note saying
/// `(M11) foil` would otherwise be read as a printing the owner never chose.
///
/// # Errors
/// [`RowError`] when the row has no count, or when a group it does recognise
/// is malformed. An unrecognised trailing word is *not* an error: it stays
/// part of the name, which is what keeps a card called `Erase (Not the Urza's
/// Legacy One)` readable.
pub fn parse(row: &str) -> Result<Row, RowError> {
    let row = row.trim();
    // The first fence wins, so a note may contain the marker itself.
    let (row, note) = match row.split_once(NOTE_FENCE) {
        Some((head, tail)) => {
            let tail = tail.trim();
            if tail.chars().count() > MAX_NOTE {
                return Err(RowError::Note);
            }
            // `1 Sol Ring # ` is a row with an empty note, which is a row
            // with no note: writing it back would produce a bare fence, and
            // the round-trip has to hold.
            (
                head.trim_end(),
                (!tail.is_empty()).then(|| tail.to_string()),
            )
        }
        None => (row, None),
    };
    let (count, rest) = row.split_once(' ').ok_or(RowError::Shape)?;
    let count: u32 = count.trim().parse().map_err(|_| RowError::Count)?;
    if count == 0 || count > MAX_COUNT {
        return Err(RowError::Count);
    }

    let mut print = PrintChoice::default();
    let mut name_end = rest.len();
    // Right to left: every group is a suffix, and the first token that is not
    // one ends the name. Going the other way would have to guess where a name
    // containing a bracket stops.
    let mut cursor = rest.trim_end();
    loop {
        let trimmed = cursor.trim_end();
        if trimmed.is_empty() {
            break;
        }
        // The last token is a candidate even when it is the only one, which
        // is how `1 [de]` is caught as a row with no card in it rather than
        // as a card called `[de]`.
        let (head, last) = trimmed.rsplit_once(' ').unwrap_or(("", trimmed));
        let taken = match classify(last)? {
            Group::Set(set) => {
                print.set = Some(set);
                true
            }
            Group::Number(number) => {
                // A bare number is only a collector number when a set code
                // stands in front of it; otherwise it is part of the name
                // (`Borrowing 100,000 Arrows`).
                if head.trim_end().ends_with(')') {
                    print.collector_number = Some(number);
                    true
                } else {
                    false
                }
            }
            Group::Lang(lang) => {
                print.lang = Some(lang);
                true
            }
            Group::Finish(finish) => {
                print.finish = Some(finish);
                true
            }
            Group::Id(id) => {
                print.scryfall_id = Some(id);
                true
            }
            Group::None => false,
        };
        if !taken {
            break;
        }
        cursor = head;
        name_end = head.trim_end().len();
    }

    let name = rest[..name_end.min(rest.len())].trim().to_string();
    if name.is_empty() {
        return Err(RowError::Shape);
    }
    Ok(Row {
        count,
        name,
        print,
        note,
    })
}

/// What one trailing token is.
enum Group {
    /// `(M11)`.
    Set(String),
    /// A bare integer, which is a collector number only after a set.
    Number(String),
    /// `[de]`.
    Lang(String),
    /// `*F*`.
    Finish(Finish),
    /// `scryfall=…`.
    Id(String),
    /// Part of the name.
    None,
}

/// Classifies one trailing token.
fn classify(token: &str) -> Result<Group, RowError> {
    if let Some(rest) = token.strip_prefix("scryfall=") {
        // Not validated as a UUID here: a deck naming a printing this build
        // has never heard of should fail where printings are resolved, with a
        // message about that printing, rather than as "malformed row".
        return Ok(if rest.is_empty() {
            Group::None
        } else {
            Group::Id(rest.to_string())
        });
    }
    if let Some(inner) = token.strip_prefix('(').and_then(|t| t.strip_suffix(')'))
        && is_set_code(inner)
    {
        return Ok(Group::Set(inner.to_ascii_uppercase()));
    }
    if let Some(inner) = token.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        if !(2..=3).contains(&inner.chars().count())
            || !inner.chars().all(|c| c.is_ascii_alphabetic())
        {
            return Err(RowError::Lang);
        }
        return Ok(Group::Lang(inner.to_ascii_lowercase()));
    }
    if let Some(inner) = token.strip_prefix('*').and_then(|t| t.strip_suffix('*'))
        && !inner.is_empty()
    {
        return Ok(Group::Finish(parse_finish(inner)?));
    }
    if !token.is_empty() && token.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Ok(Group::Number(token.to_string()));
    }
    Ok(Group::None)
}

/// Whether a parenthesised token looks like a set code rather than part of a
/// card name.
///
/// Set codes are three to five alphanumerics. `(Not the Urza's Legacy One)`
/// never reaches here as one token, but a one-word parenthetical could, and
/// `(A)` or `(seriously)` must stay part of the name.
fn is_set_code(inner: &str) -> bool {
    (3..=5).contains(&inner.chars().count()) && inner.chars().all(|c| c.is_ascii_alphanumeric())
}

impl fmt::Display for Row {
    /// Writes the row back. Round-trips: [`parse`] of this string is this row.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.count, self.name)?;
        if let Some(set) = &self.print.set {
            write!(f, " ({set})")?;
            // A collector number without a set would parse back as part of
            // the name, so it is only written when the set is there to
            // anchor it.
            if let Some(number) = &self.print.collector_number {
                write!(f, " {number}")?;
            }
        }
        if let Some(lang) = &self.print.lang {
            write!(f, " [{lang}]")?;
        }
        if let Some(marker) = self.print.finish.and_then(finish_marker) {
            write!(f, " *{marker}*")?;
        }
        if let Some(id) = &self.print.scryfall_id {
            write!(f, " scryfall={id}")?;
        }
        // Last, because it is the one thing that may hold anything at all.
        if let Some(note) = &self.note {
            write!(f, "{NOTE_FENCE} {note}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_form_still_means_what_it_meant() {
        let row = parse("4 Lightning Bolt").expect("a plain row");
        assert_eq!(row.count, 4);
        assert_eq!(row.name, "Lightning Bolt");
        assert!(row.print.is_empty(), "no printing was named");
        assert_eq!(row.print.finish_or_default(), Finish::Normal);
        assert_eq!(row.print.lang_or_default(), "en");
    }

    #[test]
    fn a_row_carries_what_its_owner_wrote_about_the_card() {
        let row = parse("1 Sol Ring # der beste Stein im Deck").expect("a noted row");
        assert_eq!(row.name, "Sol Ring", "the fence ends the name");
        assert_eq!(row.note.as_deref(), Some("der beste Stein im Deck"));
        assert_eq!(row.to_string(), "1 Sol Ring # der beste Stein im Deck");
    }

    /// The note is cut before anything is read by shape, so what a note says
    /// is never mistaken for what the owner chose.
    #[test]
    fn a_note_that_looks_like_a_printing_is_still_a_note() {
        let row = parse("1 Sol Ring # lieber die (M11) in *F*").expect("a noted row");
        assert_eq!(row.name, "Sol Ring");
        assert!(row.print.is_empty(), "nothing in the note was chosen");
        assert_eq!(row.note.as_deref(), Some("lieber die (M11) in *F*"));
    }

    /// A note sits behind the printing, and both survive the round trip.
    #[test]
    fn a_printing_and_a_note_do_not_disturb_each_other() {
        let text = "2 Lightning Bolt (M11) 149 [de] *F* # die deutsche";
        let row = parse(text).expect("a full row");
        assert_eq!(row.name, "Lightning Bolt");
        assert_eq!(row.print.set.as_deref(), Some("M11"));
        assert_eq!(row.print.finish, Some(Finish::Foil));
        assert_eq!(row.note.as_deref(), Some("die deutsche"));
        assert_eq!(row.to_string(), text, "the row writes back as it was read");
    }

    /// The counter-half: a row that says nothing extra says nothing extra,
    /// so every deck saved before notes existed is byte-identical after one.
    #[test]
    fn a_row_with_no_note_writes_no_fence() {
        let row = parse("4 Lightning Bolt").expect("a plain row");
        assert_eq!(row.note, None);
        assert_eq!(row.to_string(), "4 Lightning Bolt");
        // An empty note is no note, or the round trip would grow a bare fence.
        let empty = parse("4 Lightning Bolt # ").expect("a row with nothing after the fence");
        assert_eq!(empty.note, None);
        assert_eq!(empty.to_string(), "4 Lightning Bolt");
    }

    #[test]
    fn a_note_has_a_ceiling_like_every_other_field() {
        let long = format!("1 Sol Ring # {}", "x".repeat(MAX_NOTE + 1));
        assert_eq!(parse(&long), Err(RowError::Note));
        let fits = format!("1 Sol Ring # {}", "x".repeat(MAX_NOTE));
        assert!(parse(&fits).is_ok(), "the ceiling itself is allowed");
    }

    /// A card whose *name* holds the marker would be cut in half. No card
    /// does — `no_card_name_could_be_mistaken_for_a_note` in
    /// `baylee-cards-index` is that claim over all 33 694 of them, because
    /// this crate does not link the ledger. What is checked here is that the
    /// two halves agree about the spelling.
    #[test]
    fn the_fence_is_what_the_ledger_is_held_against() {
        assert_eq!(NOTE_FENCE, " #");
    }

    #[test]
    fn every_group_is_read() {
        let row = parse("1 Lightning Bolt (M11) 149 [de] *F* scryfall=e3285e6a-8c1d-4c9f")
            .expect("a full row");
        assert_eq!(row.name, "Lightning Bolt");
        assert_eq!(row.print.set.as_deref(), Some("M11"));
        assert_eq!(row.print.collector_number.as_deref(), Some("149"));
        assert_eq!(row.print.lang.as_deref(), Some("de"));
        assert_eq!(row.print.finish, Some(Finish::Foil));
        assert_eq!(row.print.scryfall_id.as_deref(), Some("e3285e6a-8c1d-4c9f"));
    }

    /// The groups narrow independently, so any subset must read.
    #[test]
    fn a_row_may_name_one_thing_and_stay_silent_about_the_rest() {
        assert_eq!(
            parse("1 Forest [ja]").expect("row").print.lang.as_deref(),
            Some("ja")
        );
        assert_eq!(
            parse("1 Forest *E*").expect("row").print.finish,
            Some(Finish::Etched)
        );
        assert_eq!(
            parse("1 Forest (LEA)").expect("row").print.set.as_deref(),
            Some("LEA")
        );
        for row in ["1 Forest [ja]", "1 Forest *E*", "1 Forest (LEA)"] {
            assert_eq!(parse(row).expect("row").name, "Forest", "{row}");
        }
    }

    /// The order of the trailing groups is not load-bearing: a hand-written
    /// list must not fail because somebody put the finish first.
    #[test]
    fn the_groups_may_come_in_any_order() {
        let a = parse("2 Forest (LEA) 1 [de] *F*").expect("row");
        let b = parse("2 Forest *F* [de] (LEA) 1").expect("row");
        assert_eq!(a.print.set, b.print.set);
        assert_eq!(a.print.lang, b.print.lang);
        assert_eq!(a.print.finish, b.print.finish);
        assert_eq!(a.name, b.name);
        // Only the collector number differs: it is a suffix of the set, and
        // in the second row the set does not stand in front of it.
        assert_eq!(a.print.collector_number.as_deref(), Some("1"));
    }

    /// A name that ends in a number, or in a parenthetical, must survive.
    #[test]
    fn a_name_that_looks_like_a_group_is_still_a_name() {
        assert_eq!(
            parse("1 Borrowing 100,000 Arrows").expect("row").name,
            "Borrowing 100,000 Arrows"
        );
        // A bare trailing number with no set in front of it is name, not
        // collector number.
        let row = parse("1 Kher Keep 4").expect("row");
        assert_eq!(row.name, "Kher Keep 4");
        assert_eq!(row.print.collector_number, None);
    }

    #[test]
    fn every_row_survives_a_round_trip() {
        let rows = [
            "4 Lightning Bolt",
            "1 Lightning Bolt (M11) 149",
            "1 Lightning Bolt [de]",
            "1 Lightning Bolt *F*",
            "1 Lightning Bolt *E*",
            "1 Lightning Bolt (M11) 149 [de] *F*",
            "1 Lightning Bolt scryfall=e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d",
            "1 Lightning Bolt (M11) 149 [de] *F* scryfall=e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d",
            "20 Forest",
        ];
        for src in rows {
            let row = parse(src).expect(src);
            let written = row.to_string();
            assert_eq!(written, src, "{src} wrote as {written}");
            assert_eq!(parse(&written).expect(&written), row);
        }
    }

    /// Non-foil is the default, so it is not written — a row that said `*N*`
    /// on the way in comes back plain, and means the same thing.
    #[test]
    fn the_default_finish_is_not_written_out() {
        let row = parse("1 Forest *N*").expect("row");
        assert_eq!(row.print.finish, Some(Finish::Normal));
        assert_eq!(row.to_string(), "1 Forest");
    }

    #[test]
    fn a_row_that_makes_no_sense_says_which_part() {
        // Two words with no leading number: the count is what is wrong, and
        // saying so is more use than "malformed row".
        assert_eq!(parse("Lightning Bolt"), Err(RowError::Count));
        assert_eq!(parse("Bolt"), Err(RowError::Shape));
        assert_eq!(parse("x Lightning Bolt"), Err(RowError::Count));
        assert_eq!(parse("0 Lightning Bolt"), Err(RowError::Count));
        assert_eq!(parse("99999999 Lightning Bolt"), Err(RowError::Count));
        assert_eq!(parse("1 Forest *Q*"), Err(RowError::Finish));
        assert_eq!(parse("1 Forest [deutsch]"), Err(RowError::Lang));
        assert_eq!(parse("1 "), Err(RowError::Shape));
    }

    /// A row of nothing but groups has no card in it.
    #[test]
    fn a_row_with_no_name_is_not_a_row() {
        assert_eq!(parse("1 [de] *F*"), Err(RowError::Shape));
    }

    #[test]
    fn a_plain_row_writes_as_the_old_form() {
        assert_eq!(Row::plain(4, "Forest").to_string(), "4 Forest");
    }
}
