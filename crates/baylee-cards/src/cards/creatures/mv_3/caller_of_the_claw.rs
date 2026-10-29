//! Caller of the Claw — {2}{G} — Creature — Elf
//! Oracle: Flash
//! Oracle: When this creature enters, create a 2/2 green Bear creature token for each nontoken creature put into your graveyard from the battlefield this turn.
//! Set: C15 #177 — Commander 2015 | Scryfall ID: 917c5e37-c15e-4260-b380-f1c73df2bf41 | Oracle ID: ec79255f-4b4d-4a0a-89d8-c59ba640275b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CALLER_OF_THE_CLAW,
    oracle_id = "ec79255f-4b4d-4a0a-89d8-c59ba640275b",
    scryfall_id = "917c5e37-c15e-4260-b380-f1c73df2bf41",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Caller of the Claw",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
