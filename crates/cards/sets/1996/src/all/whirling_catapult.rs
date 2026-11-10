//! Whirling Catapult — {4} — Artifact
//! Oracle: {2}, Exile the top two cards of your library: This artifact deals 1 damage to each creature with flying and each player.
//! Set: ME2 #224 — Masters Edition II | Scryfall ID: 5a405387-3018-40c8-9836-c8f5ef7a1986 | Oracle ID: 0987b5e9-0012-4189-8745-45f10e9557f3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHIRLING_CATAPULT,
    oracle_id = "0987b5e9-0012-4189-8745-45f10e9557f3",
    scryfall_id = "5a405387-3018-40c8-9836-c8f5ef7a1986",
    faces = &[face!(
        name = "Whirling Catapult",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
