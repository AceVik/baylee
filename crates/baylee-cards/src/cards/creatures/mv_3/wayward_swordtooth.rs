//! Wayward Swordtooth — {2}{G} — Creature — Dinosaur
//! Oracle: Ascend (If you control ten or more permanents, you get the city's blessing for the rest of the game.)
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: This creature can't attack or block unless you have the city's blessing.
//! Set: LCC #263 — The Lost Caverns of Ixalan Commander | Scryfall ID: 95a5d742-187a-4eba-82e4-7a4cc5c4e6f3 | Oracle ID: 3875aef0-3102-4fbf-be90-e4139f7a2348
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WAYWARD_SWORDTOOTH,
    oracle_id = "3875aef0-3102-4fbf-be90-e4139f7a2348",
    scryfall_id = "95a5d742-187a-4eba-82e4-7a4cc5c4e6f3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wayward Swordtooth",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DINOSAUR],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
