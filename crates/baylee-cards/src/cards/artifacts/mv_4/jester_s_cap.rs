//! Jester's Cap — {4} — Artifact
//! Oracle: {2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles.
//! Set: DMR #227 — Dominaria Remastered | Scryfall ID: b364e7e7-83e5-474b-9768-8b70a77e5ffa | Oracle ID: 8783a94a-d0b2-4291-a550-dbcf8e16bf4b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JESTER_S_CAP,
    oracle_id = "8783a94a-d0b2-4291-a550-dbcf8e16bf4b",
    scryfall_id = "b364e7e7-83e5-474b-9768-8b70a77e5ffa",
    faces = &[face!(
        name = "Jester's Cap",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
