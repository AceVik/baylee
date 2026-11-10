//! Pardic Miner — {1}{R} — Creature — Dwarf
//! Oracle: Sacrifice this creature: Target player can't play lands this turn.
//! Set: ODY #212 — Odyssey | Scryfall ID: 4a734abe-7bfd-4153-be2d-1c289fb60996 | Oracle ID: e1788b54-4cd9-459f-b2c9-06773e68e9e3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PARDIC_MINER,
    oracle_id = "e1788b54-4cd9-459f-b2c9-06773e68e9e3",
    scryfall_id = "4a734abe-7bfd-4153-be2d-1c289fb60996",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pardic Miner",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
