//! The one model every format reads into and writes from.
//!
//! A [`Document`] is what `docs/deck-format.md` §"JSON format" specifies,
//! field for field; JSON and YAML are this struct through serde, and the
//! text formats build and read it by hand. Every field of a printing is
//! here, so JSON and YAML lose nothing: a finish, a set, a collector number,
//! a language and a Scryfall id survive the round trip exactly.

use crate::{MAX_NAME, MAX_ROWS};
use baylee_core::deckrow::{self, MAX_COUNT, MAX_NOTE, PrintChoice, Row};
use baylee_core::preset::Finish;
use serde::{Deserialize, Serialize};

/// The document version this build writes and the only one it reads.
///
/// A document that says anything else is refused by name
/// ([`crate::ReadError::Version`]) rather than read as far as it happens to
/// fit: a later version may have changed what a field means.
pub const VERSION: u32 = 1;

/// Which list of a deck a card is in.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Zone {
    /// The deck itself.
    #[default]
    Main,
    /// The sideboard.
    Side,
    /// The commander, or both of a pair.
    Commander,
    /// Cards being considered. Baylee stores no maybeboard, so an import
    /// reports these as not kept rather than dropping them unseen.
    Maybe,
}

/// A whole deck, as a file carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Always [`VERSION`] when written.
    pub version: u32,
    /// The deck's name, when the file names it.
    #[serde(default)]
    pub name: Option<String>,
    /// The game format the deck is built for (`commander`, `constructed`),
    /// when the file says. Informational: nothing reads rules from it.
    #[serde(default)]
    pub format: Option<String>,
    /// Every row, in every zone.
    pub cards: Vec<Card>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            version: VERSION,
            name: None,
            format: None,
            cards: Vec::new(),
        }
    }
}

/// One row of a deck: this many of this card, in this zone, in this
/// printing as far as the owner chose one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    /// Which list it is in; absent means the deck itself.
    #[serde(default)]
    pub zone: Zone,
    /// Copies, 1 to `deckrow::MAX_COUNT`.
    pub count: u32,
    /// The card's name, as Baylee spells it: a double-faced card as
    /// `Front // Back` or as its front face.
    pub name: String,
    /// Set code, upper-case.
    #[serde(default)]
    pub set: Option<String>,
    /// Collector number within the set, spelled as the printing spells it.
    #[serde(default)]
    pub collector_number: Option<String>,
    /// Language code, lower-case.
    #[serde(default)]
    pub lang: Option<String>,
    /// The finish, in the wire vocabulary of [`finish_name`].
    #[serde(default, with = "finish_wire")]
    pub finish: Option<Finish>,
    /// The exact printing.
    #[serde(default)]
    pub scryfall_id: Option<String>,
    /// What the owner wrote about this row.
    #[serde(default)]
    pub note: Option<String>,
}

impl Card {
    /// A card in a zone, from a deck row.
    #[must_use]
    pub fn from_row(zone: Zone, row: Row) -> Self {
        let Row {
            count,
            name,
            print,
            note,
        } = row;
        Self {
            zone,
            count,
            name,
            set: print.set,
            collector_number: print.collector_number,
            lang: print.lang,
            finish: print.finish,
            scryfall_id: print.scryfall_id,
            note,
        }
    }

    /// The deck row this card is, zone aside.
    #[must_use]
    pub fn row(&self) -> Row {
        Row {
            count: self.count,
            name: self.name.clone(),
            print: PrintChoice {
                set: self.set.clone(),
                collector_number: self.collector_number.clone(),
                lang: self.lang.clone(),
                finish: self.finish,
                scryfall_id: self.scryfall_id.clone(),
            },
            note: self.note.clone(),
        }
    }
}

/// Why one card of a document cannot be taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CardError {
    /// No name, or only spaces.
    #[error("a card needs a name")]
    Name,
    /// Zero copies, or more than `deckrow::MAX_COUNT`.
    #[error("a card needs a copy count from 1 to 1000000")]
    Count,
    /// A note longer than `deckrow::MAX_NOTE`.
    #[error("a card's note is at most 500 characters")]
    Note,
    /// A field the stored row cannot say back exactly: a set code the row
    /// grammar would read as part of the name, a language that is not a
    /// code, a name holding the note fence or a line break.
    #[error("a card's name or printing cannot be stored as written")]
    Unstorable,
}

impl Document {
    /// Checks every bound a stored deck is held to, so that nothing a file
    /// can say reaches the builder in a shape the gateway would refuse or,
    /// worse, store as something else.
    ///
    /// The last check is the exact one: every card must survive being written
    /// as a stored row and read back. That is what a save does to it, so a
    /// card that does not survive it would be saved as a different card.
    ///
    /// # Errors
    /// [`crate::ReadError`] naming the first card that fails, by position.
    pub fn validate(&self) -> Result<(), crate::ReadError> {
        if self.version != VERSION {
            return Err(crate::ReadError::Version(self.version));
        }
        if self.cards.len() > MAX_ROWS {
            return Err(crate::ReadError::TooManyRows(self.cards.len()));
        }
        if let Some(name) = &self.name
            && (name.chars().count() > MAX_NAME || name.chars().any(char::is_control))
        {
            return Err(crate::ReadError::DeckName);
        }
        for (at, card) in self.cards.iter().enumerate() {
            card_check(card).map_err(|why| crate::ReadError::Card { at, why })?;
        }
        Ok(())
    }

    /// The deck as a saved deck is kept: main rows, sideboard rows, the
    /// commanders by name, and the maybeboard the store has no place for.
    ///
    /// A commander is *also* a main row, because that is how a stored deck
    /// seats one (`baylee_cards::decks::from_lines` moves it out of the main
    /// list when the game starts); naming it only in `commanders` would
    /// build a deck one card short.
    #[must_use]
    pub fn stored(&self) -> Stored {
        let mut stored = Stored {
            name: self.name.clone(),
            ..Stored::default()
        };
        for card in &self.cards {
            match card.zone {
                Zone::Main => stored.cards.push(card.row()),
                Zone::Side => stored.sideboard.push(card.row()),
                Zone::Maybe => stored.maybe.push(card.row()),
                Zone::Commander => {
                    stored.cards.push(card.row());
                    if !stored.commanders.contains(&card.name) {
                        stored.commanders.push(card.name.clone());
                    }
                }
            }
        }
        stored
    }

    /// A document from a saved deck's parts, the inverse of
    /// [`Document::stored`].
    ///
    /// Each commander takes one copy out of the first main row naming it, so
    /// the leader is written once, in its own zone, with the printing its
    /// row chose. A commander with no main row is still written: the deck
    /// names it, and an export that dropped it would be a different deck.
    #[must_use]
    pub fn from_stored(
        name: Option<&str>,
        format: Option<&str>,
        cards: &[Row],
        sideboard: &[Row],
        commanders: &[String],
    ) -> Self {
        let mut main: Vec<Row> = cards.to_vec();
        let mut leaders = Vec::new();
        for commander in commanders {
            match main.iter().position(|row| &row.name == commander) {
                Some(at) => {
                    let mut leader = main[at].clone();
                    leader.count = 1;
                    if main[at].count > 1 {
                        main[at].count -= 1;
                    } else {
                        main.remove(at);
                    }
                    leaders.push(leader);
                }
                None => leaders.push(Row::plain(1, commander.clone())),
            }
        }
        let cards = leaders
            .into_iter()
            .map(|row| Card::from_row(Zone::Commander, row))
            .chain(main.into_iter().map(|row| Card::from_row(Zone::Main, row)))
            .chain(
                sideboard
                    .iter()
                    .cloned()
                    .map(|row| Card::from_row(Zone::Side, row)),
            )
            .collect();
        Self {
            version: VERSION,
            name: name.map(str::to_string),
            format: format.map(str::to_string),
            cards,
        }
    }

    /// Rows in one zone, in document order.
    pub fn zone(&self, zone: Zone) -> impl Iterator<Item = &Card> {
        self.cards.iter().filter(move |card| card.zone == zone)
    }
}

/// A deck in the parts a saved deck is kept as.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stored {
    /// The deck's name, when the document named it.
    pub name: Option<String>,
    /// The main deck, commanders included.
    pub cards: Vec<Row>,
    /// The sideboard.
    pub sideboard: Vec<Row>,
    /// The commanders, by name, each also among `cards`.
    pub commanders: Vec<String>,
    /// The maybeboard, which nothing stores.
    pub maybe: Vec<Row>,
}

/// The checks one card is held to.
fn card_check(card: &Card) -> Result<(), CardError> {
    if card.name.trim().is_empty() {
        return Err(CardError::Name);
    }
    if card.count == 0 || card.count > MAX_COUNT {
        return Err(CardError::Count);
    }
    if card
        .note
        .as_ref()
        .is_some_and(|note| note.chars().count() > MAX_NOTE)
    {
        return Err(CardError::Note);
    }
    if card.name.chars().any(char::is_control)
        || card
            .note
            .as_ref()
            .is_some_and(|note| note.chars().any(char::is_control))
    {
        return Err(CardError::Unstorable);
    }
    // The exact test: what a save writes, read back, is this card. A
    // collector number with no set is the one field the row drops by design
    // (it could not be read back apart from the name), so it is compared on
    // what the row *can* hold; so is a finish that names the default, which
    // the row writes as nothing because it means nothing more.
    let mut row = card.row();
    if row.print.set.is_none() {
        row.print.collector_number = None;
    }
    if row.print.finish == Some(Finish::Normal) {
        row.print.finish = None;
    }
    let written = row.to_string();
    match deckrow::parse(&written) {
        Ok(back) if back == row => Ok(()),
        _ => Err(CardError::Unstorable),
    }
}

/// The word a finish is written as in JSON and YAML.
///
/// Scryfall's own vocabulary for the three a printing is sold in
/// (`nonfoil`, `foil`, `etched`), and the cosmetic treatments Baylee adds by
/// their own names. A table in one place, matched exhaustively, so a new
/// finish is a compile error here rather than a finish the file cannot say.
#[must_use]
pub fn finish_name(finish: Finish) -> &'static str {
    match finish {
        Finish::Normal => "nonfoil",
        Finish::Foil => "foil",
        Finish::Etched => "etched",
        Finish::Holographic => "holographic",
        Finish::Glitter => "glitter",
        Finish::Galaxy => "galaxy",
    }
}

/// Every finish, for the reader and for tests that must cover them all.
pub const FINISHES: [Finish; 6] = [
    Finish::Normal,
    Finish::Foil,
    Finish::Etched,
    Finish::Holographic,
    Finish::Glitter,
    Finish::Galaxy,
];

/// The finish a wire word names, in any case.
#[must_use]
pub fn finish_of(word: &str) -> Option<Finish> {
    FINISHES
        .into_iter()
        .find(|finish| finish_name(*finish).eq_ignore_ascii_case(word))
}

/// `Option<Finish>` as its wire word, or `null`.
mod finish_wire {
    use super::{Finish, finish_name, finish_of};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    // Serde's `with` hands the field by reference.
    #[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S: Serializer>(
        finish: &Option<Finish>,
        out: S,
    ) -> Result<S::Ok, S::Error> {
        match finish {
            Some(finish) => out.serialize_some(finish_name(*finish)),
            None => out.serialize_none(),
        }
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        input: D,
    ) -> Result<Option<Finish>, D::Error> {
        let word: Option<String> = Option::deserialize(input)?;
        match word {
            None => Ok(None),
            Some(word) => finish_of(&word)
                .map(Some)
                .ok_or_else(|| D::Error::custom(format!("unknown finish `{word}`"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> Row {
        deckrow::parse(text).expect(text)
    }

    #[test]
    fn a_commander_is_stored_among_the_main_rows_and_named_once() {
        let doc = Document {
            cards: vec![
                Card::from_row(Zone::Commander, row("1 Atraxa, Grand Unifier (ONE) 196")),
                Card::from_row(Zone::Main, row("1 Sol Ring")),
            ],
            ..Document::default()
        };
        let stored = doc.stored();
        assert_eq!(stored.commanders, ["Atraxa, Grand Unifier"]);
        assert_eq!(stored.cards.len(), 2, "the leader is a main row too");
        let back = Document::from_stored(None, None, &stored.cards, &[], &stored.commanders);
        assert_eq!(back.cards, doc.cards, "and comes back in its own zone");
    }

    #[test]
    fn a_commander_without_a_row_is_still_written() {
        let doc = Document::from_stored(None, None, &[], &[], &["Atraxa".to_string()]);
        assert_eq!(doc.cards.len(), 1);
        assert_eq!(doc.cards[0].zone, Zone::Commander);
    }

    #[test]
    fn a_commander_takes_one_copy_out_of_a_row_of_several() {
        let doc = Document::from_stored(None, None, &[row("2 Forest")], &[], &["Forest".into()]);
        let counts: Vec<(Zone, u32)> = doc.cards.iter().map(|c| (c.zone, c.count)).collect();
        assert_eq!(counts, [(Zone::Commander, 1), (Zone::Main, 1)]);
    }

    #[test]
    fn every_finish_has_a_word_and_the_word_reads_back() {
        for finish in FINISHES {
            assert_eq!(finish_of(finish_name(finish)), Some(finish));
        }
        assert_eq!(finish_of("FOIL"), Some(Finish::Foil));
        assert_eq!(finish_of("shiny"), None);
    }

    #[test]
    fn a_card_the_row_would_misread_is_refused() {
        // A six-letter set code is a real Scryfall code (`PMPS06`) the row
        // grammar reads as part of the name: stored, it would become a card
        // called `Forest (PMPS06)`.
        let mut doc = Document {
            cards: vec![Card::from_row(Zone::Main, row("1 Forest"))],
            ..Document::default()
        };
        doc.cards[0].set = Some("PMPS06".into());
        assert!(matches!(
            doc.validate(),
            Err(crate::ReadError::Card {
                at: 0,
                why: CardError::Unstorable
            })
        ));
        doc.cards[0].set = None;
        doc.cards[0].name = "Sol Ring # ramp".into();
        assert!(doc.validate().is_err(), "a name holding the note fence");
        doc.cards[0].name = "Sol\nRing".into();
        assert!(doc.validate().is_err(), "a name holding a line break");
    }

    #[test]
    fn the_bounds_of_a_row_hold_for_a_document() {
        let mut doc = Document {
            cards: vec![Card::from_row(Zone::Main, row("1 Forest"))],
            ..Document::default()
        };
        assert_eq!(doc.validate(), Ok(()));
        doc.cards[0].count = 0;
        assert!(doc.validate().is_err());
        doc.cards[0].count = MAX_COUNT + 1;
        assert!(doc.validate().is_err());
        doc.cards[0].count = 1;
        doc.cards[0].note = Some("x".repeat(MAX_NOTE + 1));
        assert!(doc.validate().is_err());
        doc.cards[0].note = None;
        doc.version = 2;
        assert_eq!(doc.validate(), Err(crate::ReadError::Version(2)));
        doc.version = VERSION;
        doc.cards = vec![doc.cards[0].clone(); MAX_ROWS + 1];
        assert_eq!(
            doc.validate(),
            Err(crate::ReadError::TooManyRows(MAX_ROWS + 1))
        );
        doc.cards.truncate(1);
        doc.name = Some("x".repeat(MAX_NAME + 1));
        assert_eq!(doc.validate(), Err(crate::ReadError::DeckName));
    }
}
