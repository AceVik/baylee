//! Storm Cauldron — {5} — Artifact
//! Oracle: Each player may play an additional land during each of their turns.
//! Oracle: Whenever a land is tapped for mana, return it to its owner's hand.
//! Set: 7ED #320 — Seventh Edition | Scryfall ID: 0bb5bdd3-6ecd-49cd-bfa2-e7da1ee85d88 | Oracle ID: 5a51b168-02d1-4eb4-8ccc-614ab6f6cffa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORM_CAULDRON,
    oracle_id = "5a51b168-02d1-4eb4-8ccc-614ab6f6cffa",
    scryfall_id = "0bb5bdd3-6ecd-49cd-bfa2-e7da1ee85d88",
    faces = &[face!(
        name = "Storm Cauldron",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
