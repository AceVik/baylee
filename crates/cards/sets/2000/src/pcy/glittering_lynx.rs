//! Glittering Lynx — {W} — Creature — Cat
//! Oracle: Prevent all damage that would be dealt to this creature.
//! Oracle: {2}: Until end of turn, this creature loses "Prevent all damage that would be dealt to this creature." Any player may activate this ability.
//! Set: PCY #11 — Prophecy | Scryfall ID: a3f26c7e-c525-4191-a542-b81343ae95bb | Oracle ID: 890ffe31-642f-46e3-9f09-c744351653b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GLITTERING_LYNX,
    oracle_id = "890ffe31-642f-46e3-9f09-c744351653b5",
    scryfall_id = "a3f26c7e-c525-4191-a542-b81343ae95bb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Glittering Lynx",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
