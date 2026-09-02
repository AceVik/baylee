//! Bristling Backwoods — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, it deals 1 damage to target opponent.
//! Oracle: {T}: Add {R} or {G}.
//! Set: OTJ #253 — Outlaws of Thunder Junction | Scryfall ID: d61dfeb7-7f6b-4601-8396-2cbb98165489 | Oracle ID: 9cbc9f83-8979-42a5-a466-a8d89c8e6de8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 313,
    oracle_id: "9cbc9f83-8979-42a5-a466-a8d89c8e6de8",
    scryfall_id: "d61dfeb7-7f6b-4601-8396-2cbb98165489",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces: &[
    face! {
        name: "Bristling Backwoods",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
