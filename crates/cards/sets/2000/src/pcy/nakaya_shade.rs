//! Nakaya Shade — {1}{B} — Creature — Shade
//! Oracle: {B}: This creature gets +1/+1 until end of turn unless any player pays {2}.
//! Set: PCY #69 — Prophecy | Scryfall ID: eefd9315-9b7c-4c6b-8a15-a6af873dab6f | Oracle ID: ce969713-fdaa-4c31-b164-76a9b0b9ff81
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NAKAYA_SHADE,
    oracle_id = "ce969713-fdaa-4c31-b164-76a9b0b9ff81",
    scryfall_id = "eefd9315-9b7c-4c6b-8a15-a6af873dab6f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nakaya Shade",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHADE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
