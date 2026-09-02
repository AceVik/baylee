//! Billiard Room — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #13 — Ravnica: Clue Edition | Scryfall ID: dc2a3de1-01ac-4425-8534-e5019e01f2cd | Oracle ID: a4fc174e-7fa6-41a8-ae03-255f226840f9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 271,
    oracle_id: "a4fc174e-7fa6-41a8-ae03-255f226840f9",
    scryfall_id: "dc2a3de1-01ac-4425-8534-e5019e01f2cd",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
    face! {
        name: "Billiard Room",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
