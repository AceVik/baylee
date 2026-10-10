//! How far the pool is through each set, for the admin console
//! (`GET /admin/sets`, `GET /admin/sets/{code}`; `docs/protocol.md` §"The
//! admin console").
//!
//! One source of truth in two halves this build already links: the ledger
//! (`baylee_cards_index::ROWS`, every card of the corpus with the set it was
//! first printed in) and the compiled pool (`baylee_cards::pool::rows()`,
//! the cards this build plays and how far each one is). A ledger row with no
//! pool row is a card nothing has been written for ("absent"); for the rest
//! the pool says `implemented`, `partial` or `unimplemented` (a stub). Sets
//! come in the ledger's order, which is release order by first printing, the
//! order the owner implements them in (CLAUDE.md §"Cards"), with promo sets
//! where their first card falls. Nothing is written or committed to say
//! any of this: the numbers are whatever this binary was built with.
//!
//! A set is counted as "the cards first printed in it", which is what the
//! ledger knows. Scryfall's set pages count every printing, so a set's
//! total here is smaller than Scryfall's for any set with reprints.

use std::collections::HashMap;
use std::sync::LazyLock;

use baylee_cards::pool::PoolCard;
use baylee_cards_index::ROWS;

/// One set's progress, in cards first printed there.
#[derive(Clone, Debug, serde::Serialize)]
pub struct SetProgress {
    /// Scryfall's set code, lower case.
    pub code: &'static str,
    /// Where it falls in release order, from 0.
    pub position: usize,
    /// Cards first printed in this set.
    pub total: usize,
    /// Pool cards the deckbuilder offers as playable.
    pub implemented: usize,
    /// Pool cards with a `Coverage::Partial` note.
    pub partial: usize,
    /// Pool cards that compile as stubs.
    pub unimplemented: usize,
    /// Ledger rows with no pool card at all.
    pub absent: usize,
}

impl SetProgress {
    fn new(code: &'static str, position: usize) -> Self {
        Self {
            code,
            position,
            total: 0,
            implemented: 0,
            partial: 0,
            unimplemented: 0,
            absent: 0,
        }
    }

    fn count(&mut self, coverage: &str) {
        self.total += 1;
        match coverage {
            "implemented" => self.implemented += 1,
            "partial" => self.partial += 1,
            "unimplemented" => self.unimplemented += 1,
            _ => self.absent += 1,
        }
    }
}

/// One card of a set, as the drill-down lists it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct SetCard {
    /// The ledger index (`CardIndex`).
    pub index: u32,
    /// The English name, as the ledger has it.
    pub name: &'static str,
    /// `implemented`, `partial`, `unimplemented`, or `absent` for a card
    /// with no pool row.
    pub coverage: &'static str,
    /// Why a partial card is only partly there, in its author's words.
    pub note: Option<&'static str>,
    /// The printed type line; empty for an absent card.
    pub type_line: String,
    /// The Scryfall id of the printing the pool card was generated from, or
    /// `None` for an absent card. The admin's browser builds the picture's
    /// address from it itself (`docs/legal.md` §3).
    pub scryfall_id: Option<&'static str>,
    /// The oracle id, which an absent card is still found by.
    pub oracle_id: &'static str,
}

/// The pool's rows by ledger index.
fn pool_by_index() -> HashMap<u32, &'static PoolCard> {
    baylee_cards::pool::rows()
        .iter()
        .map(|card| (card.index, card))
        .collect()
}

/// The table, built once: every set in release order with its counts.
struct Table {
    sets: Vec<SetProgress>,
    by_code: HashMap<&'static str, usize>,
    pool: HashMap<u32, &'static PoolCard>,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| {
    let pool = pool_by_index();
    let mut sets: Vec<SetProgress> = Vec::new();
    let mut by_code: HashMap<&'static str, usize> = HashMap::new();
    for row in &ROWS {
        let at = *by_code.entry(row.set).or_insert_with(|| {
            sets.push(SetProgress::new(row.set, sets.len()));
            sets.len() - 1
        });
        let coverage = pool
            .get(&row.index.get())
            .map_or("absent", |card| card.coverage);
        sets[at].count(coverage);
    }
    Table {
        sets,
        by_code,
        pool,
    }
});

/// Every set in release order.
pub fn all() -> &'static [SetProgress] {
    &TABLE.sets
}

/// One set by its code (any case), or `None`.
pub fn one(code: &str) -> Option<&'static SetProgress> {
    let code = code.to_ascii_lowercase();
    TABLE.by_code.get(code.as_str()).map(|&at| &TABLE.sets[at])
}

/// The cards first printed in `set`, in ledger order.
pub fn cards(set: &SetProgress) -> Vec<SetCard> {
    ROWS.iter()
        .filter(|row| row.set == set.code)
        .map(|row| {
            let card = TABLE.pool.get(&row.index.get()).copied();
            SetCard {
                index: row.index.get(),
                name: row.name,
                coverage: card.map_or("absent", |c| c.coverage),
                note: card.and_then(|c| c.note),
                type_line: card.map(|c| c.type_line.clone()).unwrap_or_default(),
                scryfall_id: card.map(|c| c.scryfall_id),
                oracle_id: row.oracle_id,
            }
        })
        .collect()
}

/// The pool as a whole: how many cards, and how far.
pub fn pool_totals() -> SetProgress {
    let mut whole = SetProgress::new("", 0);
    for set in all() {
        whole.total += set.total;
        whole.implemented += set.implemented;
        whole.partial += set.partial;
        whole.unimplemented += set.unimplemented;
        whole.absent += set.absent;
    }
    whole
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every ledger row lands in exactly one set, and every pool card in
    /// exactly one of the three pool buckets: the counts are a partition,
    /// not an estimate.
    #[test]
    fn the_counts_partition_the_ledger_and_the_pool() {
        let whole = pool_totals();
        assert_eq!(whole.total, ROWS.len());
        assert_eq!(
            whole.implemented + whole.partial + whole.unimplemented + whole.absent,
            ROWS.len()
        );
        let pool = baylee_cards::pool::rows();
        assert_eq!(
            whole.implemented + whole.partial + whole.unimplemented,
            pool.len(),
            "a pool card without a ledger row, or counted twice"
        );
        assert!(whole.implemented > 0);
        assert!(whole.absent > 0, "the whole corpus is not built yet");
    }

    /// Release order: Alpha first, then Beta, as the ledger is written.
    #[test]
    fn sets_come_in_release_order_from_alpha() {
        let sets = all();
        assert_eq!(sets[0].code, "lea");
        assert_eq!(sets[1].code, "leb");
        for (at, set) in sets.iter().enumerate() {
            assert_eq!(set.position, at);
        }
        let codes: std::collections::HashSet<&str> = sets.iter().map(|s| s.code).collect();
        assert_eq!(codes.len(), sets.len(), "a set listed twice");
    }

    /// A set's list says the same as its counts, in any case of the code.
    #[test]
    fn a_sets_cards_agree_with_its_counts() {
        let alpha = one("LEA").expect("Alpha");
        let list = cards(alpha);
        assert_eq!(list.len(), alpha.total);
        let bucket = |coverage: &str| list.iter().filter(|c| c.coverage == coverage).count();
        assert_eq!(bucket("implemented"), alpha.implemented);
        assert_eq!(bucket("partial"), alpha.partial);
        assert_eq!(bucket("unimplemented"), alpha.unimplemented);
        assert_eq!(bucket("absent"), alpha.absent);
        let bolt = list
            .iter()
            .find(|c| c.name == "Lightning Bolt")
            .expect("Alpha has Lightning Bolt");
        assert_ne!(bolt.coverage, "absent");
        assert!(bolt.scryfall_id.is_some());
        assert!(!bolt.type_line.is_empty());
        for absent in list.iter().filter(|c| c.coverage == "absent") {
            assert!(absent.scryfall_id.is_none());
            assert!(absent.note.is_none());
            assert!(!absent.oracle_id.is_empty());
        }
        assert!(one("nope").is_none());
    }
}
