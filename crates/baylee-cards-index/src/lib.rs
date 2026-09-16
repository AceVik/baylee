//! The `CardIndex` ledger, as Rust the compiler checks.
//!
//! This crate *is* the ledger: [`ROWS`] is the assignment itself, not a cache
//! of one. It used to be `data/card-index.tsv`, and the reason it is no
//! longer a data file is that a data file is a second truth beside the code —
//! nobody reads it and the compiler does not check it. A generated table it
//! does check: a row with a missing column, a duplicated field name or a
//! `CardIndex` that is not a number is a build failure rather than a line
//! somebody has to notice.
//!
//! `cargo xtask ledger` is the only thing that writes [`generated`], and it
//! links this crate to read what is already assigned — the same shape as the
//! orphan guard, which links `baylee_cards::all()` to ask what this repo
//! compiles. So the file the assigner writes is the source of the binary that
//! writes it, and a run that would emit a table that does not build is caught
//! by the build, one step later, with the previous table still committed.
//!
//! # Why this is its own crate
//!
//! The table carries every card's `oracle_id` and printed name: about 2.7 MB
//! of strings that the rules engine has no use for whatever. A feature on
//! `baylee-core` would not have kept them out of it — Cargo unifies features
//! across a workspace build, so one tool asking for the data would compile it
//! into everything. A crate the engine does not link keeps that structural
//! instead of conventional.
//!
//! Consequently this crate is **not** in the set that must compile for
//! `wasm32-unknown-unknown`: the client resolves a card by `CardIndex`, which
//! is a number, and never by name.
//!
//! The constants — `baylee_core::generated::index::MOX_OPAL` — are a separate
//! rendering of this same table, and live in `baylee-core` because card files
//! name them. They cost the engine nothing: a `pub const` that nothing uses
//! compiles to nothing at all. `cargo xtask codegen --check` is what holds
//! the two in step.

pub mod generated;

pub use generated::ROWS;

use baylee_core::ids::CardIndex;

/// One assignment: a card, and the index it owns for good.
///
/// Four of the five fields are **frozen** — `index`, `oracle_id`, `constant`
/// and `set` are what the rest of the workspace names, and changing one is a
/// breaking change to stored data or to card files. `name` follows Scryfall,
/// so a rename shows up in the diff instead of drifting silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// The permanent index. Rows are in ascending order and none is ever
    /// removed, so this is also the row's position — but it is written down
    /// rather than counted, because a row deleted by hand would otherwise
    /// renumber every card after it in silence, which is the one accident the
    /// ledger exists to prevent.
    pub index: CardIndex,
    /// Scryfall oracle id — the card's rules identity, and the key.
    pub oracle_id: &'static str,
    /// The generated constant's name (`LIGHTNING_BOLT`), frozen on
    /// assignment.
    pub constant: &'static str,
    /// Set code of the printing this card first appeared in, frozen on
    /// assignment. It decides which generated file the constant lives in.
    pub set: &'static str,
    /// The card's whole name (`Fire // Ice`), following Scryfall.
    pub name: &'static str,
}

#[cfg(test)]
mod tests {
    use super::ROWS;

    #[test]
    fn the_table_is_dense_and_in_order() {
        for (position, row) in ROWS.iter().enumerate() {
            assert_eq!(
                row.index.get() as usize,
                position,
                "{} sits at {position}",
                row.name
            );
        }
    }

    #[test]
    fn no_card_and_no_constant_is_claimed_twice() {
        // Cheap enough to do by sorting rather than by hashing 33 694 strings
        // twice, and this is what the codegen-side guard is checking too — it
        // is here as well because the compiler cannot see a duplicate
        // `&'static str` the way it sees a duplicate field.
        let mut ids: Vec<&str> = ROWS.iter().map(|r| r.oracle_id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "an oracle id is in the table twice");

        let mut constants: Vec<&str> = ROWS.iter().map(|r| r.constant).collect();
        constants.sort_unstable();
        let before = constants.len();
        constants.dedup();
        assert_eq!(
            constants.len(),
            before,
            "a constant is claimed twice — a glob re-export would hide one of \
             the two cards rather than refuse"
        );
    }

    /// A deck row fences its owner's note off with
    /// [`baylee_core::deckrow::NOTE_FENCE`], and everything before the fence
    /// is the card. That is only safe while no card is named with one, so the
    /// claim is measured against every card there is rather than against the
    /// pool — this table is the one place that population exists.
    ///
    /// The bare `#` is checked too, and it is the more useful half: it is
    /// what a new set would have to print before the spelling of the fence
    /// mattered at all, and it fails here with the card's name in hand
    /// instead of in somebody's deck.
    #[test]
    fn no_card_name_could_be_mistaken_for_a_note() {
        let fenced: Vec<&str> = ROWS
            .iter()
            .map(|r| r.name)
            .filter(|n| n.contains(baylee_core::deckrow::NOTE_FENCE))
            .collect();
        assert!(
            fenced.is_empty(),
            "a card name holds the note fence and would be cut in half: {fenced:?}"
        );

        let hashed: Vec<&str> = ROWS
            .iter()
            .map(|r| r.name)
            .filter(|n| n.contains('#'))
            .collect();
        assert!(
            hashed.is_empty(),
            "a card name holds a '#'; the deck row's note fence is one space \
             away from eating it: {hashed:?}"
        );

        // The counter-half: the filter above is capable of finding one.
        assert!(
            "Card # 1".contains(baylee_core::deckrow::NOTE_FENCE),
            "the fence is spelled the way this test looks for it"
        );
    }
}
