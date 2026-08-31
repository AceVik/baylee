//! The `CardIndex` ledger: which index a card owns, and keeps.
//!
//! An index used to be a card's position in the alphabetically sorted pool.
//! That made it a *position*, and adding one card renumbered every card after
//! it — while a `CardIndex` is an *identity*: `DeckEntry` stores one, the
//! gateway persists decks made of them, and a replay is a list of actions
//! naming them. Renumbering pointed every saved deck at a different card, and
//! nothing would have said so.
//!
//! Assignments therefore live in `data/card-index.tsv` and are append-only. A
//! card new to the pool takes the next free index. A card that leaves the pool
//! keeps its own: the slot is retired rather than handed on, because a deck
//! saved last year may still name it.
//!
//! The file is the source of truth, not a cache — `cargo xtask codegen
//! --check` fails if a run would change it, so an index can never be assigned
//! without landing in a commit.

use crate::error::CodegenError;
use std::fmt::Write as _;

/// One assignment: an index, the card that owns it, and the name it had when
/// it was assigned (a comment column — the oracle id is the key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// The permanent `CardIndex`.
    pub index: u32,
    /// Scryfall oracle id — the card's rules identity.
    pub oracle_id: String,
    /// The card's name, for readability in diffs.
    pub name: String,
}

/// Every assignment ever made, ordered by index.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IndexLedger {
    entries: Vec<LedgerEntry>,
}

const HEADER: &str = "\
# CardIndex ledger — assigned once, never reused, never reordered.
#
# A card's index is its permanent rules identity: DeckEntry stores one, the
# gateway persists decks made of them, and a replay names them. So this file
# is append-only. A new card takes the next free index; a card that leaves the
# pool retires its own and the slot stays empty.
#
# Written by `cargo xtask codegen`. `codegen --check` fails if a run would
# change it, so no index is ever assigned outside a commit.
#
# index\toracle_id\tname
";

impl IndexLedger {
    /// Reads a ledger file. An empty or missing file is an empty ledger.
    ///
    /// # Errors
    /// If a line is neither a comment nor `index<TAB>oracle_id<TAB>name`, or
    /// if two entries claim the same index or the same card.
    pub fn parse(text: &str) -> Result<Self, CodegenError> {
        let mut entries: Vec<LedgerEntry> = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut cols = line.splitn(3, '\t');
            let bad = || CodegenError::LedgerLine {
                line: n + 1,
                text: line.to_string(),
            };
            let index: u32 = cols.next().ok_or_else(bad)?.parse().map_err(|_| bad())?;
            let oracle_id = cols.next().ok_or_else(bad)?.to_string();
            let name = cols.next().unwrap_or("").to_string();
            if oracle_id.is_empty() {
                return Err(bad());
            }
            if entries
                .iter()
                .any(|e| e.index == index || e.oracle_id == oracle_id)
            {
                return Err(bad());
            }
            entries.push(LedgerEntry {
                index,
                oracle_id,
                name,
            });
        }
        entries.sort_by_key(|e| e.index);
        Ok(Self { entries })
    }

    /// Renders the file this ledger came from (byte-stable for a given state).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::from(HEADER);
        for e in &self.entries {
            let _ = writeln!(out, "{}\t{}\t{}", e.index, e.oracle_id, e.name);
        }
        out
    }

    /// The index this card owns, if it has one.
    #[must_use]
    pub fn index_of(&self, oracle_id: &str) -> Option<u32> {
        self.entries
            .iter()
            .find(|e| e.oracle_id == oracle_id)
            .map(|e| e.index)
    }

    /// The card's index, assigning the next free one if it has none.
    ///
    /// A card that is already in the ledger keeps its index even if its name
    /// has since changed on Scryfall; the name column is updated so the change
    /// shows up in the diff.
    pub fn assign(&mut self, oracle_id: &str, name: &str) -> u32 {
        if let Some(e) = self.entries.iter_mut().find(|e| e.oracle_id == oracle_id) {
            if e.name != name {
                e.name = name.to_string();
            }
            return e.index;
        }
        let index = self.entries.last().map_or(0, |e| e.index + 1);
        self.entries.push(LedgerEntry {
            index,
            oracle_id: oracle_id.to_string(),
            name: name.to_string(),
        });
        index
    }

    /// One past the highest index ever assigned — the length the registry's
    /// index table needs, holes included.
    #[must_use]
    pub fn slots(&self) -> usize {
        self.entries.last().map_or(0, |e| e.index as usize + 1)
    }

    /// Every assignment, ordered by index.
    #[must_use]
    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole file exists for: a card added later must not
    /// move a card that was already there, whatever its name.
    #[test]
    fn a_new_card_never_moves_an_existing_one() {
        let mut l = IndexLedger::default();
        assert_eq!(l.assign("oracle-m", "Middle"), 0);
        assert_eq!(l.assign("oracle-z", "Zebra"), 1);
        // "Aardvark" sorts first and would have taken index 0 under the old
        // positional scheme.
        assert_eq!(l.assign("oracle-a", "Aardvark"), 2);
        assert_eq!(l.index_of("oracle-m"), Some(0));
        assert_eq!(l.index_of("oracle-z"), Some(1));
    }

    /// Asking twice is asking once: codegen runs on every build.
    #[test]
    fn assigning_the_same_card_twice_returns_the_same_index() {
        let mut l = IndexLedger::default();
        let first = l.assign("oracle-a", "Ancestral Recall");
        assert_eq!(l.assign("oracle-a", "Ancestral Recall"), first);
        assert_eq!(l.entries().len(), 1);
    }

    /// A Scryfall rename keeps the index and updates the readable column, so
    /// the rename shows up in the diff instead of silently drifting.
    #[test]
    fn a_renamed_card_keeps_its_index() {
        let mut l = IndexLedger::default();
        let i = l.assign("oracle-a", "Old Name");
        assert_eq!(l.assign("oracle-a", "New Name"), i);
        assert_eq!(l.entries()[0].name, "New Name");
    }

    /// A retired index is not handed on: the next card takes the next number,
    /// not the hole, because a deck saved last year may still name it.
    #[test]
    fn a_hole_is_never_refilled() {
        let text = "0\toracle-a\tA\n2\toracle-c\tC\n";
        let mut l = IndexLedger::parse(text).expect("parses");
        assert_eq!(l.slots(), 3, "index 1 is retired but still occupies a slot");
        assert_eq!(l.assign("oracle-d", "D"), 3);
    }

    #[test]
    fn round_trips_through_the_file() {
        let mut l = IndexLedger::default();
        l.assign("oracle-a", "Ancestral Recall");
        l.assign("oracle-b", "Black Lotus");
        let reparsed = IndexLedger::parse(&l.render()).expect("parses");
        assert_eq!(reparsed, l);
    }

    #[test]
    fn rejects_a_duplicate_index_or_card() {
        assert!(IndexLedger::parse("0\toracle-a\tA\n0\toracle-b\tB\n").is_err());
        assert!(IndexLedger::parse("0\toracle-a\tA\n1\toracle-a\tA\n").is_err());
        assert!(IndexLedger::parse("nope\toracle-a\tA\n").is_err());
        assert!(IndexLedger::parse("0\t\tA\n").is_err());
    }

    #[test]
    fn comments_and_blank_lines_are_not_entries() {
        let l = IndexLedger::parse("# a comment\n\n0\toracle-a\tA\n").expect("parses");
        assert_eq!(l.entries().len(), 1);
    }
}
