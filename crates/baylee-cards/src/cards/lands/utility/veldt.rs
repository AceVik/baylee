//! Veldt — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {G} or {W}. Put a depletion counter on this land.
//! Set: ICE #363 — Ice Age | Scryfall ID: 987534fb-74a9-46a3-805f-fe2fe2df4a90 | Oracle ID: 10b9bfb3-c478-45c6-b227-9c66b63bc79b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 1284,
    oracle_id: "10b9bfb3-c478-45c6-b227-9c66b63bc79b",
    scryfall_id: "987534fb-74a9-46a3-805f-fe2fe2df4a90",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    faces: &[
    face! {
        name: "Veldt",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
