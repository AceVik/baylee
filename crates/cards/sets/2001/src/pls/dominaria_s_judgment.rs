//! Dominaria's Judgment — {2}{W} — Instant
//! Oracle: Until end of turn, creatures you control gain protection from white if you control a Plains, from blue if you control an Island, from black if you control a Swamp, from red if you control a Mountain, and from green if you control a Forest.
//! Set: PLS #4 — Planeshift | Scryfall ID: 9703d090-b415-48e2-8158-dd8fc57ecc50 | Oracle ID: 41998d4d-ca01-4a9c-8bd6-399e4c3e25b1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DOMINARIA_S_JUDGMENT,
    oracle_id = "41998d4d-ca01-4a9c-8bd6-399e4c3e25b1",
    scryfall_id = "9703d090-b415-48e2-8158-dd8fc57ecc50",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dominaria's Judgment",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
