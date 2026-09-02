//! Dining Room — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {G}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #15 — Ravnica: Clue Edition | Scryfall ID: 1bf8dcb2-6fe7-4ab3-b290-fb427d116c74 | Oracle ID: 0880461e-8943-443b-90e7-ff84eef46550
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 429,
    oracle_id: "0880461e-8943-443b-90e7-ff84eef46550",
    scryfall_id: "1bf8dcb2-6fe7-4ab3-b290-fb427d116c74",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces: &[
    face! {
        name: "Dining Room",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
