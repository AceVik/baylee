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
//! card new to the corpus takes the next free index. A card that leaves keeps
//! its own: the slot is retired rather than handed on, because a deck saved
//! last year may still name it.
//!
//! The file is the source of truth, not a cache, and `cargo xtask ledger` is
//! the only thing that writes it — codegen reads it and refuses a card with
//! no row, so an index can never be assigned as a side effect of a build.
//! What codegen *does* write from it is `crates/baylee-core/src/generated/
//! index/`, the same assignment as Rust constants (see [`crate::cardindex`]),
//! and `codegen --check` is what holds the two in step.
//!
//! # Why the order is written down and not recomputed
//!
//! An index is assigned by **first appearance**, over the whole card corpus
//! rather than over the cards this repo has implemented. Both halves matter.
//! Over implemented cards only, adopting a 1993 card would have to insert it
//! among 1993's neighbours or admit the order is a lie; over the corpus every
//! card already has its place, so adopting one moves nothing.
//!
//! And the rule is *recorded* rather than re-derived, because its input is
//! not a constant. "First appearance" is a fact about Scryfall's data, not
//! about Magic: `Dog` dates to 1995 although the creature type was called
//! `Hound` until 2020, and a corpus filter or a backdated promo can reorder
//! cards nobody touched. A ledger row is the rule's answer, frozen on the day
//! it was first asked.
//!
//! # What each column is for
//!
//! `index`, `oracle_id`, `const` and `set` are **frozen**: they are what the
//! rest of the workspace names, and changing one is a breaking change to
//! stored data or to card files. `name` **follows Scryfall**, so a rename
//! shows up in the diff instead of drifting silently.
//!
//! `const` is stored rather than derived from `name` for exactly that reason.
//! Derive it, and a Scryfall rename — or a change to the corpus filter —
//! renames a constant and breaks every card file that used it.

use crate::error::CodegenError;
use crate::stubgen::{slug, untransliterable};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// One assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry {
    /// The permanent `CardIndex`.
    pub index: u32,
    /// Scryfall oracle id — the card's rules identity, and the key.
    pub oracle_id: String,
    /// The generated constant's name (`LIGHTNING_BOLT`), frozen on
    /// assignment.
    pub constant: String,
    /// Set code of the printing this card first appeared in, frozen on
    /// assignment. It decides which generated file the constant lives in.
    pub set: String,
    /// The card's whole name (`Fire // Ice`), following Scryfall.
    pub name: String,
}

/// Every assignment ever made, ordered by index.
#[derive(Debug, Default, Clone)]
pub struct IndexLedger {
    entries: Vec<LedgerEntry>,
    /// `oracle_id` to position in `entries`, so a corpus of 33 586 cards is
    /// not a quadratic scan. The linear `find` this replaced would cost about
    /// 560 million string comparisons on one pass.
    by_card: HashMap<String, usize>,
    /// Constants already claimed, for the same reason.
    taken: HashSet<String>,
}

impl PartialEq for IndexLedger {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for IndexLedger {}

const HEADER: &str = "\
# CardIndex ledger — assigned once, never reused, never reordered.
#
# A card's index is its permanent rules identity: DeckEntry stores one, the
# gateway persists decks made of them, and a replay names them. So this file
# is append-only. A new card takes the next free index; a card that leaves the
# corpus retires its own and the slot stays empty.
#
# Indices are assigned by first appearance over the whole card corpus, not
# over the cards this repo implements — so adopting an old card inserts
# nothing. The order is written down rather than recomputed, because what it
# derives from (release dates, the corpus filter, Scryfall's own errata) is
# not a constant.
#
# index, oracle_id, const and set are FROZEN: the workspace names them.
# name follows Scryfall, so a rename shows in the diff.
#
# Seeded by `baylee-catalog corpus` + `cargo xtask ledger`; read by
# `cargo xtask codegen`, which never assigns. `codegen --check` fails if a run
# would change this file, so no index is assigned outside a commit.
#
# index\toracle_id\tconst\tset\tname
";

/// What `baylee-cards-index`'s `generated.rs` opens with.
///
/// Short on purpose: the crate's own `lib.rs` carries the reasoning, and a
/// generated file that restates it would be one more thing to keep in step.
const ROWS_HEADER: &str = "\
// GENERATED by `cargo xtask ledger` — do not edit by hand.
//
// The CardIndex ledger: every card there is, and the index it owns for good.
// Assigned by first appearance over the whole card corpus, append-only, and
// never reordered — a card's index is what saved decks and replays name.
//
// `cargo xtask ledger` is the only thing that writes this. Codegen reads it
// and refuses a card with no row; `crates/baylee-core/src/generated/index/`
// is the same assignment rendered as constants, held in step by
// `cargo xtask codegen --check`.
//
// See this crate's lib.rs for why the ledger is Rust and not a data file.

#![allow(missing_docs, clippy::all, clippy::pedantic)]

use crate::Row;
use baylee_core::ids::CardIndex;

#[rustfmt::skip]
";

impl IndexLedger {
    /// Reads a ledger file. An empty or missing file is an empty ledger.
    ///
    /// # Errors
    /// If a line is not `index<TAB>oracle_id<TAB>const<TAB>set<TAB>name`, or
    /// if two entries claim the same index, card, or constant.
    pub fn parse(text: &str) -> Result<Self, CodegenError> {
        let mut this = Self::default();
        let mut seen_index: HashSet<u32> = HashSet::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bad = || CodegenError::LedgerLine {
                line: n + 1,
                text: line.to_string(),
            };
            let mut cols = line.splitn(5, '\t');
            let index: u32 = cols.next().ok_or_else(bad)?.parse().map_err(|_| bad())?;
            let oracle_id = cols.next().ok_or_else(bad)?.to_string();
            let constant = cols.next().ok_or_else(bad)?.to_string();
            let set = cols.next().ok_or_else(bad)?.to_string();
            let name = cols.next().unwrap_or("").to_string();
            if oracle_id.is_empty() || constant.is_empty() {
                return Err(bad());
            }
            if !seen_index.insert(index)
                || this.by_card.contains_key(&oracle_id)
                || !this.taken.insert(constant.clone())
            {
                return Err(bad());
            }
            this.by_card.insert(oracle_id.clone(), this.entries.len());
            this.entries.push(LedgerEntry {
                index,
                oracle_id,
                constant,
                set,
                name,
            });
        }
        this.sort_by_index();
        Ok(this)
    }

    /// Reads the ledger out of the compiled table.
    ///
    /// The same three locks `parse` applies — no index, no card and no
    /// constant twice — because what the compiler checks about the table is
    /// its *shape*, and none of those three is a shape: two rows naming one
    /// `oracle_id` are as well-typed as any other two.
    ///
    /// # Errors
    /// If two rows claim the same index, card, or constant.
    pub fn from_rows(rows: &[baylee_cards_index::Row]) -> Result<Self, CodegenError> {
        let mut this = Self {
            entries: Vec::with_capacity(rows.len()),
            by_card: HashMap::with_capacity(rows.len()),
            taken: HashSet::with_capacity(rows.len()),
        };
        let mut seen_index: HashSet<u32> = HashSet::with_capacity(rows.len());
        for (n, row) in rows.iter().enumerate() {
            let index = row.index.get();
            if !seen_index.insert(index)
                || this.by_card.contains_key(row.oracle_id)
                || !this.taken.insert(row.constant.to_string())
            {
                return Err(CodegenError::LedgerLine {
                    line: n + 1,
                    text: format!("{index}\t{}\t{}", row.oracle_id, row.constant),
                });
            }
            this.by_card
                .insert(row.oracle_id.to_string(), this.entries.len());
            this.entries.push(LedgerEntry {
                index,
                oracle_id: row.oracle_id.to_string(),
                constant: row.constant.to_string(),
                set: row.set.to_string(),
                name: row.name.to_string(),
            });
        }
        this.sort_by_index();
        Ok(this)
    }

    /// Restores the by-index order, and the position map with it.
    fn sort_by_index(&mut self) {
        self.entries.sort_by_key(|e| e.index);
        self.by_card = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.oracle_id.clone(), i))
            .collect();
    }

    /// Renders the file this ledger came from (byte-stable for a given state).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::from(HEADER);
        for e in &self.entries {
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                e.index, e.oracle_id, e.constant, e.set, e.name
            );
        }
        out
    }

    /// Renders `baylee-cards-index`'s `generated.rs` — the ledger itself.
    ///
    /// Every string goes out through `{:?}`, which is `str`'s own `Debug` and
    /// escapes exactly what Rust needs escaped while leaving printable
    /// Unicode alone. Seven card names in the corpus carry a `"` and 91 carry
    /// a letter outside ASCII, so hand-rolled quoting would be a bug rather
    /// than a shortcut.
    ///
    /// `#[rustfmt::skip]` is on the table for the same reason the constants
    /// tree is written verbatim: this is written without running rustfmt over
    /// it, so `cargo fmt --all` must be made to leave it alone or the two
    /// would undo each other forever.
    #[must_use]
    pub fn render_rows(&self) -> String {
        let mut out = String::with_capacity(self.entries.len() * 150);
        out.push_str(ROWS_HEADER);
        let _ = writeln!(out, "pub static ROWS: [Row; {}] = [", self.entries.len());
        for e in &self.entries {
            let _ = writeln!(
                out,
                "    Row {{ index: CardIndex::new({}), oracle_id: {:?}, constant: {:?}, set: {:?}, name: {:?} }},",
                e.index, e.oracle_id, e.constant, e.set, e.name
            );
        }
        out.push_str("];\n");
        out
    }

    /// The index this card owns, if it has one.
    #[must_use]
    pub fn index_of(&self, oracle_id: &str) -> Option<u32> {
        self.by_card.get(oracle_id).map(|&i| self.entries[i].index)
    }

    /// The row this card owns, if it has one.
    #[must_use]
    pub fn entry_of(&self, oracle_id: &str) -> Option<&LedgerEntry> {
        self.by_card.get(oracle_id).map(|&i| &self.entries[i])
    }

    /// The card's index, assigning the next free one if it has none.
    ///
    /// A card already in the ledger keeps its index, its constant and its set
    /// even if Scryfall has since renamed it or backdated an earlier
    /// printing; only the name column follows, so the change shows up in the
    /// diff rather than moving anything.
    ///
    /// # Errors
    /// If the name carries a letter with no ASCII spelling, or if its
    /// constant is claimed and no tie-break frees one.
    pub fn assign(&mut self, oracle_id: &str, name: &str, set: &str) -> Result<u32, CodegenError> {
        if let Some(&i) = self.by_card.get(oracle_id) {
            let e = &mut self.entries[i];
            if e.name != name {
                e.name = name.to_string();
            }
            return Ok(e.index);
        }
        let index = self.entries.last().map_or(0, |e| e.index + 1);
        let constant = self.free_constant(name, set, index)?;
        self.taken.insert(constant.clone());
        self.by_card
            .insert(oracle_id.to_string(), self.entries.len());
        self.entries.push(LedgerEntry {
            index,
            oracle_id: oracle_id.to_string(),
            constant,
            set: set.to_string(),
            name: name.to_string(),
        });
        Ok(index)
    }

    /// The constant a card gets, in three rungs.
    ///
    /// The bare name first; then the name with its set; then with its index,
    /// which cannot collide because no two cards share one. The *earlier*
    /// card keeps the bare form, and that is what makes the rule stable under
    /// appending: a card assigned today can never take a name already given
    /// out, so no existing constant ever moves.
    ///
    /// Measured over the corpus: the first rung is enough for all 33 586
    /// cards, and it stays that way only because the joke sets and the
    /// playtest cards are not in it. The other two rungs are what happens if
    /// that changes — instead of a rename nobody notices.
    fn free_constant(&self, name: &str, set: &str, index: u32) -> Result<String, CodegenError> {
        let front = name.split(" // ").next().unwrap_or(name);
        if let Some(letter) = untransliterable(front) {
            return Err(CodegenError::Untransliterable {
                name: name.to_string(),
                letter,
            });
        }
        let base = slug(front).to_uppercase();
        if base.is_empty() {
            return Err(CodegenError::ConstantCollision {
                constant: String::new(),
                held: String::new(),
                wanted: name.to_string(),
            });
        }
        let with_set = format!("{base}_{}", slug(set).to_uppercase());
        for candidate in [
            base.clone(),
            with_set.clone(),
            format!("{with_set}_{index}"),
        ] {
            if !self.taken.contains(&candidate) {
                return Ok(candidate);
            }
        }
        Err(CodegenError::ConstantCollision {
            constant: base.clone(),
            held: self
                .entries
                .iter()
                .find(|e| e.constant == base)
                .map_or_else(String::new, |e| e.name.clone()),
            wanted: name.to_string(),
        })
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

    /// Builds a ledger from rows without any of the guards `parse` applies.
    ///
    /// Test-only, and it exists for one test: `cardindex`'s refusal to render
    /// two cards claiming one constant cannot be reached through `parse` or
    /// `assign`, because both refuse a duplicate first. Proving the third
    /// lock works needs a door past the first two.
    #[cfg(test)]
    pub(crate) fn from_entries_unchecked(entries: Vec<LedgerEntry>) -> Self {
        let mut this = Self {
            entries,
            by_card: HashMap::new(),
            taken: HashSet::new(),
        };
        this.sort_by_index();
        this
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assign(l: &mut IndexLedger, oracle: &str, name: &str) -> u32 {
        l.assign(oracle, name, "tst").expect("assigns")
    }

    /// The property the whole file exists for: a card added later must not
    /// move a card that was already there, whatever its name.
    #[test]
    fn a_new_card_never_moves_an_existing_one() {
        let mut l = IndexLedger::default();
        assert_eq!(assign(&mut l, "oracle-m", "Middle"), 0);
        assert_eq!(assign(&mut l, "oracle-z", "Zebra"), 1);
        // "Aardvark" sorts first and would have taken index 0 under the old
        // positional scheme.
        assert_eq!(assign(&mut l, "oracle-a", "Aardvark"), 2);
        assert_eq!(l.index_of("oracle-m"), Some(0));
        assert_eq!(l.index_of("oracle-z"), Some(1));
    }

    /// Asking twice is asking once: codegen runs on every build.
    #[test]
    fn assigning_the_same_card_twice_returns_the_same_index() {
        let mut l = IndexLedger::default();
        let first = assign(&mut l, "oracle-a", "Ancestral Recall");
        assert_eq!(assign(&mut l, "oracle-a", "Ancestral Recall"), first);
        assert_eq!(l.entries().len(), 1);
    }

    /// A Scryfall rename keeps the index *and the constant*. The constant is
    /// what card files name, so a rename that moved it would break them.
    #[test]
    fn a_renamed_card_keeps_its_index_and_its_constant() {
        let mut l = IndexLedger::default();
        let i = assign(&mut l, "oracle-a", "Old Name");
        assert_eq!(assign(&mut l, "oracle-a", "New Name"), i);
        assert_eq!(l.entries()[0].name, "New Name");
        assert_eq!(l.entries()[0].constant, "OLD_NAME");
    }

    /// A retired index is not handed on: the next card takes the next number,
    /// not the hole, because a deck saved last year may still name it.
    #[test]
    fn a_hole_is_never_refilled() {
        let text = "0\toracle-a\tA\tlea\tA\n2\toracle-c\tC\tlea\tC\n";
        let mut l = IndexLedger::parse(text).expect("parses");
        assert_eq!(l.slots(), 3, "index 1 is retired but still occupies a slot");
        assert_eq!(assign(&mut l, "oracle-d", "D"), 3);
    }

    #[test]
    fn round_trips_through_the_file() {
        let mut l = IndexLedger::default();
        assign(&mut l, "oracle-a", "Ancestral Recall");
        assign(&mut l, "oracle-b", "Fire // Ice");
        let reparsed = IndexLedger::parse(&l.render()).expect("parses");
        assert_eq!(reparsed, l);
        assert_eq!(l.entries()[1].constant, "FIRE", "the front face names it");
        assert_eq!(l.entries()[1].name, "Fire // Ice", "the whole name is kept");
    }

    #[test]
    fn rejects_a_duplicate_index_card_or_constant() {
        assert!(IndexLedger::parse("0\toracle-a\tA\tlea\tA\n1\toracle-b\tB\tlea\tB\n").is_ok());
        // same index
        assert!(IndexLedger::parse("0\toracle-a\tA\tlea\tA\n0\toracle-b\tB\tlea\tB\n").is_err());
        // same card
        assert!(IndexLedger::parse("0\toracle-a\tA\tlea\tA\n1\toracle-a\tB\tlea\tB\n").is_err());
        // same constant
        assert!(IndexLedger::parse("0\toracle-a\tA\tlea\tA\n1\toracle-b\tA\tlea\tB\n").is_err());
        assert!(IndexLedger::parse("nope\toracle-a\tA\tlea\tA\n").is_err());
        assert!(IndexLedger::parse("0\t\tA\tlea\tA\n").is_err());
        assert!(IndexLedger::parse("0\toracle-a\t\tlea\tA\n").is_err());
    }

    /// Two cards of one name take the bare constant and then the set; the
    /// *earlier* one keeps the bare form, so nothing already assigned moves.
    #[test]
    fn a_second_card_of_the_same_name_takes_its_set() {
        let mut l = IndexLedger::default();
        l.assign("oracle-a", "Bind", "inv").expect("assigns");
        l.assign("oracle-b", "Bind", "cmb1").expect("assigns");
        assert_eq!(l.entries()[0].constant, "BIND");
        assert_eq!(l.entries()[1].constant, "BIND_CMB1");
        // A third in the same set falls through to the index, which is unique
        // by construction.
        l.assign("oracle-c", "Bind", "cmb1").expect("assigns");
        assert_eq!(l.entries()[2].constant, "BIND_CMB1_2");
    }

    /// A letter the slug table cannot spell stops the assignment instead of
    /// freezing a mangled constant in the file.
    #[test]
    fn a_name_that_cannot_be_spelt_in_ascii_is_refused() {
        let mut l = IndexLedger::default();
        assert!(l.assign("oracle-a", "稲妻", "jp").is_err());
        assert!(l.assign("oracle-b", "Barad-dûr", "ltr").is_ok());
        assert_eq!(l.entries()[0].constant, "BARAD_DUR");
    }

    #[test]
    fn comments_and_blank_lines_are_not_entries() {
        let l = IndexLedger::parse("# a comment\n\n0\toracle-a\tA\tlea\tA\n").expect("parses");
        assert_eq!(l.entries().len(), 1);
    }

    /// The migration, held against itself while both halves exist.
    ///
    /// `data/card-index.tsv` is being retired in favour of the compiled
    /// table, and the whole claim is that the move carries every column. So
    /// the table is read back, rendered as the file it replaces, and compared
    /// to the committed bytes. This test is the reason the TSV may be deleted
    /// afterwards — and it goes with it, because nothing then remains to
    /// compare against.
    #[test]
    fn the_compiled_table_says_exactly_what_the_file_it_replaces_says() {
        let from_table = IndexLedger::from_rows(&baylee_cards_index::ROWS).expect("reads");
        let committed = include_str!("../../../data/card-index.tsv");
        assert_eq!(from_table.entries().len(), 33_694);
        assert_eq!(from_table.render(), committed);
        assert_eq!(from_table, IndexLedger::parse(committed).expect("parses"));
    }

    /// The table is written without running rustfmt over it — 5.4 MB through
    /// a process to change nothing — so `cargo fmt --all` has to be made to
    /// leave it alone, or the two would undo each other on every run. That is
    /// `#[rustfmt::skip]`'s whole job here, and it is in the header, where
    /// nothing else would notice it going missing.
    #[test]
    fn the_table_tells_rustfmt_to_keep_its_hands_off() {
        assert!(
            IndexLedger::default()
                .render_rows()
                .contains("#[rustfmt::skip]")
        );
    }
}
