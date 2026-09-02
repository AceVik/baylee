//! The Shire — (no cost) — Legendary Land
//! Oracle: The Shire enters tapped unless you control a legendary creature.
//! Oracle: {T}: Add {G}.
//! Oracle: {1}{G}, {T}, Tap an untapped creature you control: Create a Food token.
//! Set: LTR #260 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: d5178a1b-588b-4414-a370-ac6eed51187a | Oracle ID: 9abf9a0e-8e7d-406b-a01d-d4870b30134e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1180,
    oracle_id: "9abf9a0e-8e7d-406b-a01d-d4870b30134e",
    scryfall_id: "d5178a1b-588b-4414-a370-ac6eed51187a",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "The Shire",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
