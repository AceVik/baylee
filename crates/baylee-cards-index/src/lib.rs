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

/// Finds a card by its printed name — **every** card there is, not the pool.
///
/// This is deliberately not `baylee_cards::decks::by_name`, and the
/// difference is the whole point of it: that one answers "can this build play
/// the card" and is what every other caller wants, while this one answers
/// "is that a card at all". Asking both separates the two facts a single
/// `None` used to carry — a name that is nothing, and a real card this build
/// compiles no `CardDef` for, which is 30 978 of the 33 694 rows here. A
/// player who mistypes and a player who names Black Lotus were told the same
/// thing, and the second of those is most of them.
///
/// Two tiers, because the two tables spell a two-faced card differently: this
/// one follows Scryfall (`Sheoldred // The True Scriptures`) and the pool
/// names the front face alone (`Sheoldred`), so a whole-name match is tried
/// first and the part before the ` // ` after it. That is safe rather than
/// merely convenient, and
/// [`the_two_tiers_cannot_disagree_about_a_card`](self) is what keeps it so:
/// no front face is also some other card's whole name and no front face is
/// claimed twice, across all 33 694 rows. Without the second tier 753 of the
/// 874 two-faced cards outside the pool would be reported as no card at all.
///
/// It is a **linear scan**, and it stays one: the only caller is an error
/// path that has already missed in the pool's perfect hash, so a valid deck
/// reaches this nought times and a rejected one reaches it once. Measured in
/// release, per call: 126 µs for a miss, which is the worst case and compares
/// every row twice; 41 µs for a whole name in the last row; 87 µs for a front
/// face. A perfect hash like the pool's would buy three orders of magnitude
/// on an answer nobody is waiting for, and cost every build that links this
/// crate a table to carry.
///
/// What the second tier must **not** do is ask each row for its
/// [`front_face`](Row::front_face), which splits on ` // ` and therefore
/// searches the whole of all 33 694 names: that spelling measured 1.43 ms,
/// eleven times the cost of the one above. `strip_prefix` answers the same
/// question and gives up on the first byte that differs, which for nearly
/// every row is the first one.
#[must_use]
pub fn row_by_name(name: &str) -> Option<&'static Row> {
    ROWS.iter().find(|row| row.name == name).or_else(|| {
        ROWS.iter().find(|row| {
            row.name
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with(" // "))
        })
    })
}

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

impl Row {
    /// The card's name up to the ` // `, which for a single-faced card is
    /// the whole of it.
    ///
    /// This is the spelling `baylee-cards` uses — the pool names a two-faced
    /// card by its front face and this table follows Scryfall — so it is
    /// what joins a row to a `CardDef`.
    #[must_use]
    pub fn front_face(&self) -> &'static str {
        self.name.split(" // ").next().unwrap_or(self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::{ROWS, row_by_name};

    /// [`row_by_name`] falls back to the front face, and that second tier is
    /// only an answer while it is an *unambiguous* one. Two ways it could
    /// stop being: a front face that is also some other card's whole name
    /// (the fallback would hand out the wrong row), and a front face two
    /// cards share (it would hand out whichever came first). Neither holds
    /// today over 874 two-faced names, and neither is something this repo
    /// controls — Wizards print the names — so it is measured here rather
    /// than assumed, and a set that breaks it fails the build with the card
    /// in hand.
    #[test]
    fn the_two_tiers_cannot_disagree_about_a_card() {
        let whole: std::collections::HashSet<&str> = ROWS.iter().map(|r| r.name).collect();
        let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        let mut two_faced = 0;
        for row in ROWS.iter().filter(|r| r.name.contains(" // ")) {
            two_faced += 1;
            let front = row.front_face();
            assert!(
                !whole.contains(front),
                "{front} is both the front face of {} and a card of its own",
                row.name
            );
            if let Some(other) = seen.insert(front, row.name) {
                panic!("{front} is the front face of both {other} and {}", row.name);
            }
        }
        assert!(
            two_faced > 800,
            "only {two_faced} two-faced names — this test found nothing to check"
        );
    }

    /// The two tiers, each reached on purpose. `Sheoldred` is the pool's own
    /// spelling of a card this table calls `Sheoldred // The True
    /// Scriptures`, so it is the case the second tier exists for.
    #[test]
    fn a_card_is_found_by_either_spelling_and_a_non_card_by_neither() {
        let whole = row_by_name("Sheoldred // The True Scriptures").expect("the whole name");
        let front = row_by_name("Sheoldred").expect("the front face alone");
        assert_eq!(whole.index, front.index, "two spellings, one card");
        assert!(
            row_by_name("Not A Real Card").is_none(),
            "a name that is no card answers None"
        );
        assert!(
            row_by_name("Sheoldred // ").is_none(),
            "a partial spelling is not a match"
        );
    }

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
