//! White Plume Adventurer — {2}{W} — Creature — Orc Cleric
//! Oracle: When this creature enters, you take the initiative.
//! Oracle: At the beginning of each opponent's upkeep, untap a creature you control. If you've completed a dungeon, untap all creatures you control instead.
//! Set: CLB #49 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: b256ddc8-8b12-434c-a610-1a872e948f2f | Oracle ID: 51f091a7-9b0b-4362-8e9c-c174752f369b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WHITE_PLUME_ADVENTURER,
    oracle_id = "51f091a7-9b0b-4362-8e9c-c174752f369b",
    scryfall_id = "b256ddc8-8b12-434c-a610-1a872e948f2f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "White Plume Adventurer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
