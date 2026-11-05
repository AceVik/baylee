//! What a client may know about a pool card beyond its text: what each format
//! says about it, and the sets and rarities it was printed in.
//!
//! `GET /pool/facts` (`docs/protocol.md` §"Pool facts") sends this for the whole
//! pool, and a client filters and later validates against it locally. It is
//! here for the reason [`crate::CardTextEntry`] is: both ends read one shape,
//! and this crate depends on serde and nothing else, so the gateway, a wasm
//! client and `baylee_cards::formats` can all link it. The engine does not:
//! legality is the gateway's question and never a rule of the game
//! (`docs/banlists-and-formats.md`).
//!
//! # What it holds, and what it leaves out
//!
//! Status per card and format, and which `(set, rarity)` pairs a card has
//! printings in: what the client's filters read, keyed on the registry index
//! of this game's own pool. No printing ids, collector numbers, artists,
//! languages or dates, because no feature reads them here and Scryfall's terms
//! say "You may not simply repackage, republish, or proxy Scryfall data"
//! (`docs/legal.md` §3). [`PoolFactsBody::sets`] names a set because the
//! dialog offers sets by name.
//!
//! # Why the wire is shaped this way
//!
//! Ten thousand rows, so nothing is spelled twice: a set is a place in
//! [`PoolFactsBody::sets`], a card's verdicts are a place in
//! [`PoolFactsBody::verdicts`] (a few hundred distinct strings stand for every
//! card), and a printing is one number holding both its set and its rarity.
//! Decoded, the rows lie in one flat list and are found by binary search.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a format says about a card, in Scryfall's four words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Legality {
    /// It may be played.
    Legal,
    /// It may not.
    Banned,
    /// One copy of it may be played, as in Vintage.
    Restricted,
    /// The format does not have it in its pool at all.
    NotLegal,
}

impl Legality {
    /// The word Scryfall writes.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Legal => "legal",
            Self::Banned => "banned",
            Self::Restricted => "restricted",
            Self::NotLegal => "not_legal",
        }
    }

    /// The status a word from Scryfall names.
    #[must_use]
    pub fn of(word: &str) -> Option<Self> {
        [Self::Legal, Self::Banned, Self::Restricted, Self::NotLegal]
            .into_iter()
            .find(|status| status.word() == word)
    }

    /// Whether a deck of the format may hold the card: a restricted card is
    /// played, once, which is what Scryfall's `f:` means too.
    #[must_use]
    pub const fn playable(self) -> bool {
        matches!(self, Self::Legal | Self::Restricted)
    }

    /// The letter it stands as in a [`PoolFactsBody::verdicts`] string.
    const fn letter(self) -> u8 {
        match self {
            Self::Legal => b'l',
            Self::Banned => b'b',
            Self::Restricted => b'r',
            Self::NotLegal => b'n',
        }
    }

    /// The status a letter stands for; `None` for [`NO_VERDICT`] and anything
    /// else.
    const fn of_letter(letter: u8) -> Option<Self> {
        match letter {
            b'l' => Some(Self::Legal),
            b'b' => Some(Self::Banned),
            b'r' => Some(Self::Restricted),
            b'n' => Some(Self::NotLegal),
            _ => None,
        }
    }
}

/// The letter of a card no verdict was stored for in that format.
const NO_VERDICT: u8 = b'-';

/// A printing's rarity, in the order Scryfall compares them (`r>=rare`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    /// Common.
    Common,
    /// Uncommon.
    Uncommon,
    /// Rare.
    Rare,
    /// Special: the timeshifted and box-topper kind.
    Special,
    /// Mythic rare.
    Mythic,
    /// Bonus: a printing outside a set's own rarities.
    Bonus,
}

impl Rarity {
    /// Every rarity, in comparison order.
    pub const ALL: [Self; 6] = [
        Self::Common,
        Self::Uncommon,
        Self::Rare,
        Self::Special,
        Self::Mythic,
        Self::Bonus,
    ];

    /// The word Scryfall writes.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Special => "special",
            Self::Mythic => "mythic",
            Self::Bonus => "bonus",
        }
    }

    /// The rarity a word names: the whole word, or its first letter, which is
    /// how `r:m` is written. Case does not matter.
    #[must_use]
    pub fn of(word: &str) -> Option<Self> {
        let folded = word.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|rarity| rarity.word() == folded || rarity.word()[..1] == folded)
    }

    /// Its place in [`Self::ALL`], which is what a packed printing holds.
    const fn code(self) -> u32 {
        self as u32
    }
}

/// How many low bits of a packed printing hold the rarity.
const RARITY_BITS: u32 = 3;
/// The rarity of a printing the catalog stored none for.
const NO_RARITY: u32 = (1 << RARITY_BITS) - 1;

/// One set a pool card was printed in.
///
/// @description One set a pool card was printed in.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PoolSet {
    /// @description Scryfall's set code, lower case.
    /// @internal
    /// Its code, as `s:` is written and a deck line prints it.
    pub code: String,
    /// @description The set's name, as Scryfall gives it (never translated).
    /// @internal
    /// Its name, which the filter dialog offers it by.
    pub name: String,
}

/// `GET /pool/facts`, as it travels.
///
/// @description What a client filters a pool card by: every format's verdict
/// and the sets and rarities it was printed in, for the whole pool. Empty
/// (`version` too) where the gateway has no filled catalog.
/// @openapi
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PoolFactsBody {
    /// @description A hash of the rest of this answer, 16 hex digits: what
    /// `have` names to be told nothing changed. Empty on an empty answer.
    /// @internal
    /// A hash of the rest, so a copy can be kept by it. Content and not the
    /// catalog's stamp: a restored catalog goes back to an older stamp, and
    /// two gateways on two catalogs may hold the same one.
    pub version: String,
    /// @description Scryfall's format keys, sorted; a `verdicts` string has a
    /// letter for each, in this order.
    /// @internal
    /// The formats the catalog stores, sorted, as they arrived: the list
    /// follows the feed, so a new format needs no release.
    pub formats: Vec<String>,
    /// @description The sets the pool was printed in, newest first.
    /// @internal
    /// The sets the pool was printed in, newest first.
    pub sets: Vec<PoolSet>,
    /// @description The distinct verdict strings, one letter per format: `l`
    /// legal, `b` banned, `r` restricted, `n` not legal, `-` nothing stored.
    /// @internal
    /// Every distinct verdict string, sorted.
    pub verdicts: Vec<String>,
    /// @description One row per pool card the catalog knows, by ascending
    /// registry index: `[index, place in verdicts, printing…]`, where a
    /// printing is a place in `sets` times 8 plus a rarity, 0 to 5 for
    /// `common`, `uncommon`, `rare`, `special`, `mythic`, `bonus`, 7 for none
    /// stored.
    /// @internal
    /// `[index, verdict, printing…]`, ascending by index, a printing being
    /// the `(set, rarity)` pair in one number, each pair once.
    pub cards: Vec<Vec<u32>>,
}

/// One card, as [`PoolFactsBody::assemble`] takes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CardLine {
    /// Its registry index.
    pub index: u32,
    /// What each format says, in Scryfall's words. Empty when nothing is
    /// stored for the card.
    pub legalities: BTreeMap<String, String>,
    /// Its `(set code, rarity word)` pairs. A set [`PoolFactsBody::assemble`]
    /// was not given is left out.
    pub printed: Vec<(String, String)>,
}

impl PoolFactsBody {
    /// The table for these cards, every list in one order whatever order they
    /// came in, so a hash of it says whether anything changed. `sets` is in
    /// the order the answer lists them. [`Self::version`] is left empty for
    /// the gateway, which hashes what it is about to send.
    #[must_use]
    pub fn assemble(sets: Vec<PoolSet>, mut lines: Vec<CardLine>) -> Self {
        lines.sort_by_key(|line| line.index);
        let mut formats: Vec<String> = lines
            .iter()
            .flat_map(|line| line.legalities.keys().cloned())
            .collect();
        formats.sort_unstable();
        formats.dedup();
        let verdict_of = |line: &CardLine| -> String {
            formats
                .iter()
                .map(|format| {
                    line.legalities
                        .get(format)
                        .and_then(|word| Legality::of(word))
                        .map_or(NO_VERDICT, Legality::letter) as char
                })
                .collect()
        };
        let mut verdicts: Vec<String> = lines.iter().map(verdict_of).collect();
        verdicts.sort_unstable();
        verdicts.dedup();
        let place_of: BTreeMap<&str, u32> = sets
            .iter()
            .zip(0..)
            .map(|(set, at)| (set.code.as_str(), at))
            .collect();
        let cards = lines
            .iter()
            .map(|line| {
                let verdict = verdicts
                    .binary_search(&verdict_of(line))
                    .ok()
                    .and_then(|at| u32::try_from(at).ok())
                    .unwrap_or(0);
                let mut printed: Vec<u32> = line
                    .printed
                    .iter()
                    .filter_map(|(set, rarity)| {
                        let at = *place_of.get(set.as_str())?;
                        let rarity = Rarity::of(rarity).map_or(NO_RARITY, Rarity::code);
                        Some(at << RARITY_BITS | rarity)
                    })
                    .collect();
                printed.sort_unstable();
                printed.dedup();
                let mut row = vec![line.index, verdict];
                row.append(&mut printed);
                row
            })
            .collect();
        Self {
            version: String::new(),
            formats,
            sets,
            verdicts,
            cards,
        }
    }
}

/// Where one card's facts lie in a [`PoolFacts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    index: u32,
    verdict: u32,
    from: u32,
    to: u32,
}

/// A [`PoolFactsBody`] as a client holds it: one flat list of printings, and
/// rows found by index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoolFacts {
    version: String,
    formats: Vec<String>,
    sets: Vec<PoolSet>,
    verdicts: Vec<String>,
    rows: Vec<Row>,
    printed: Vec<u32>,
}

impl From<PoolFactsBody> for PoolFacts {
    /// A row too short to name a card is left out rather than refused: what
    /// is left is still true of the cards it names.
    fn from(body: PoolFactsBody) -> Self {
        let mut rows = Vec::with_capacity(body.cards.len());
        let mut printed = Vec::with_capacity(body.cards.iter().map(Vec::len).sum());
        for row in &body.cards {
            let [index, verdict, printings @ ..] = row.as_slice() else {
                continue;
            };
            let from = u32::try_from(printed.len()).unwrap_or(u32::MAX);
            printed.extend_from_slice(printings);
            rows.push(Row {
                index: *index,
                verdict: *verdict,
                from,
                to: u32::try_from(printed.len()).unwrap_or(u32::MAX),
            });
        }
        rows.sort_unstable_by_key(|row| row.index);
        Self {
            version: body.version,
            formats: body.formats,
            sets: body.sets,
            verdicts: body.verdicts,
            rows,
            printed,
        }
    }
}

impl PoolFacts {
    /// The version this table was sent under.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Whether it says nothing about any card, which is what a gateway with
    /// no catalog sends.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// How many cards it knows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// The formats it can answer, sorted.
    #[must_use]
    pub fn formats(&self) -> &[String] {
        &self.formats
    }

    /// Whether it can answer this format.
    #[must_use]
    pub fn knows_format(&self, format: &str) -> bool {
        self.format_at(format).is_some()
    }

    /// The sets the pool was printed in, newest first.
    #[must_use]
    pub fn sets(&self) -> &[PoolSet] {
        &self.sets
    }

    /// What it says about the card with this registry index; `None` for a
    /// card it does not know, which is not a card with no verdicts.
    #[must_use]
    pub fn of(&self, index: u32) -> Option<CardFacts<'_>> {
        let at = self
            .rows
            .binary_search_by_key(&index, |row| row.index)
            .ok()?;
        let row = self.rows[at];
        Some(CardFacts {
            table: self,
            verdict: self
                .verdicts
                .get(row.verdict as usize)
                .map_or("", String::as_str),
            printed: self
                .printed
                .get(row.from as usize..row.to as usize)
                .unwrap_or_default(),
        })
    }

    fn format_at(&self, format: &str) -> Option<usize> {
        self.formats
            .iter()
            .position(|known| known.eq_ignore_ascii_case(format))
    }
}

/// What the table says about one card.
#[derive(Clone, Copy, Debug)]
pub struct CardFacts<'a> {
    table: &'a PoolFacts,
    verdict: &'a str,
    printed: &'a [u32],
}

/// One `(set, rarity)` pair a card has printings in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Printed<'a> {
    /// The set's code.
    pub set: &'a str,
    /// Its rarity there; `None` where the catalog stored none.
    pub rarity: Option<Rarity>,
}

impl<'a> CardFacts<'a> {
    /// What this format says about the card; `None` for a format the table
    /// does not know and for a card nothing is stored about in it.
    #[must_use]
    pub fn legality(self, format: &str) -> Option<Legality> {
        let at = self.table.format_at(format)?;
        Legality::of_letter(*self.verdict.as_bytes().get(at)?)
    }

    /// Every `(set, rarity)` pair the card has printings in.
    pub fn printings(self) -> impl Iterator<Item = Printed<'a>> {
        let table = self.table;
        self.printed.iter().filter_map(move |packed| {
            let set = table.sets.get((packed >> RARITY_BITS) as usize)?;
            let rarity = Rarity::ALL.get((packed & NO_RARITY) as usize).copied();
            Some(Printed {
                set: &set.code,
                rarity,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(code: &str) -> PoolSet {
        PoolSet {
            code: code.to_string(),
            name: code.to_uppercase(),
        }
    }

    fn line(index: u32, legalities: &[(&str, &str)], printed: &[(&str, &str)]) -> CardLine {
        CardLine {
            index,
            legalities: legalities
                .iter()
                .map(|(format, word)| ((*format).to_string(), (*word).to_string()))
                .collect(),
            printed: printed
                .iter()
                .map(|(set, rarity)| ((*set).to_string(), (*rarity).to_string()))
                .collect(),
        }
    }

    fn table() -> PoolFacts {
        PoolFactsBody::assemble(
            vec![set("m21"), set("lea")],
            vec![
                line(
                    9,
                    &[("vintage", "restricted"), ("modern", "not_legal")],
                    &[("lea", "rare")],
                ),
                line(
                    4,
                    &[("vintage", "legal"), ("modern", "legal")],
                    &[("lea", "common"), ("m21", "uncommon"), ("m21", "uncommon")],
                ),
            ],
        )
        .into()
    }

    #[test]
    fn a_card_is_found_by_index_whatever_order_it_came_in() {
        let table = table();
        assert_eq!(table.len(), 2);
        let card = table.of(9).expect("the card is there");
        assert_eq!(card.legality("vintage"), Some(Legality::Restricted));
        assert_eq!(card.legality("Modern"), Some(Legality::NotLegal));
        assert_eq!(
            card.printings().collect::<Vec<_>>(),
            [Printed {
                set: "lea",
                rarity: Some(Rarity::Rare)
            }]
        );
        assert!(table.of(5).is_none(), "a card it was never told about");
    }

    #[test]
    fn a_pair_is_kept_once_and_a_format_nobody_stored_says_nothing() {
        let table = table();
        let card = table.of(4).expect("the card is there");
        assert_eq!(card.printings().count(), 2);
        assert_eq!(card.legality("frobnicate"), None);
        assert!(!table.knows_format("frobnicate"));
        assert!(table.knows_format("VINTAGE"));
    }

    /// The hash the gateway sends is of this order, so the same cards must
    /// make the same table whatever order a query handed them over in.
    #[test]
    fn the_same_cards_make_the_same_table() {
        let a = line(
            1,
            &[("modern", "legal")],
            &[("m21", "rare"), ("lea", "rare")],
        );
        let b = line(2, &[("pauper", "banned")], &[("lea", "common")]);
        let sets = || vec![set("m21"), set("lea")];
        let one = PoolFactsBody::assemble(sets(), vec![a.clone(), b.clone()]);
        let two = PoolFactsBody::assemble(sets(), vec![b, a]);
        assert_eq!(one, two);
    }

    #[test]
    fn a_card_with_nothing_stored_has_no_verdict_and_a_rarity_may_be_missing() {
        let body = PoolFactsBody::assemble(
            vec![set("m21")],
            vec![
                line(1, &[], &[("m21", "")]),
                line(2, &[("modern", "legal")], &[]),
            ],
        );
        let table = PoolFacts::from(body);
        let bare = table.of(1).expect("the card is there");
        assert_eq!(bare.legality("modern"), None);
        assert_eq!(
            bare.printings().collect::<Vec<_>>(),
            [Printed {
                set: "m21",
                rarity: None
            }]
        );
    }

    /// A row from a gateway that answers something odd is not worth a panic:
    /// what the row cannot say is simply not said.
    #[test]
    fn a_mangled_row_is_read_for_what_it_says() {
        let table = PoolFacts::from(PoolFactsBody {
            version: "v".to_string(),
            formats: vec!["modern".to_string()],
            sets: vec![set("m21")],
            verdicts: vec!["l".to_string()],
            cards: vec![vec![1], vec![2, 9, 800, 0], vec![3, 0]],
        });
        assert!(table.of(1).is_none(), "too short to name a card");
        let far = table.of(2).expect("the card is there");
        assert_eq!(far.legality("modern"), None, "no such verdict");
        assert_eq!(far.printings().count(), 1, "no such set");
        assert_eq!(
            table.of(3).expect("the card is there").legality("modern"),
            Some(Legality::Legal)
        );
    }

    /// The row a client reads is the row the route describes: pinned, as
    /// `CardTextEntry` is, so the two ends cannot drift.
    #[test]
    fn the_wire_is_pinned() {
        let mut body = PoolFactsBody::assemble(
            vec![set("m21")],
            vec![line(7, &[("modern", "banned")], &[("m21", "mythic")])],
        );
        body.version = "abc".to_string();
        let written = serde_json::to_string(&body).expect("writes");
        assert_eq!(
            written,
            r#"{"version":"abc","formats":["modern"],"sets":[{"code":"m21","name":"M21"}],"verdicts":["b"],"cards":[[7,0,4]]}"#
        );
        let read: PoolFactsBody = serde_json::from_str(&written).expect("reads");
        assert_eq!(read, body);
    }

    #[test]
    fn rarities_are_compared_in_scryfalls_order_and_read_by_word_or_letter() {
        assert!(Rarity::Common < Rarity::Uncommon && Rarity::Rare < Rarity::Mythic);
        assert_eq!(Rarity::of("m"), Some(Rarity::Mythic));
        assert_eq!(Rarity::of("Rare"), Some(Rarity::Rare));
        assert_eq!(Rarity::of("frob"), None);
        assert_eq!(Rarity::of(""), None);
    }

    #[test]
    fn a_restricted_card_is_played_and_a_banned_one_is_not() {
        assert!(Legality::Restricted.playable() && Legality::Legal.playable());
        assert!(!Legality::Banned.playable() && !Legality::NotLegal.playable());
        assert_eq!(Legality::of("not_legal"), Some(Legality::NotLegal));
        assert_eq!(Legality::of("kept"), None);
    }
}
