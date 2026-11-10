//! Mana Web — {3} — Artifact
//! Oracle: Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce.
//! Set: WTH #152 — Weatherlight | Scryfall ID: 2c72ec90-dacc-496f-a7f5-f18bfce5eb3e | Oracle ID: 82909ff4-f6f7-44f1-bbd0-6e54afa5ee6d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_WEB,
    oracle_id = "82909ff4-f6f7-44f1-bbd0-6e54afa5ee6d",
    scryfall_id = "2c72ec90-dacc-496f-a7f5-f18bfce5eb3e",
    faces = &[face!(
        name = "Mana Web",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
