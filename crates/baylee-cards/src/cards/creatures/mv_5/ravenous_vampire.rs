//! Ravenous Vampire — {3}{B}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, you may sacrifice a nonartifact creature. If you do, put a +1/+1 counter on this creature. If you don't, tap this creature.
//! Set: MIR #136 — Mirage | Scryfall ID: 5ee67033-7f0f-49b4-8472-3bc4fbb8ffe1 | Oracle ID: 2c029419-3601-4158-b10f-45a19de7faa3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAVENOUS_VAMPIRE,
    oracle_id = "2c029419-3601-4158-b10f-45a19de7faa3",
    scryfall_id = "5ee67033-7f0f-49b4-8472-3bc4fbb8ffe1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ravenous Vampire",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
