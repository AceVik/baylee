//! Carrion Feeder — {B} — Creature — Zombie
//! Oracle: This creature can't block.
//! Oracle: Sacrifice a creature: Put a +1/+1 counter on this creature.
//! Set: MH1 #81 — Modern Horizons | Scryfall ID: 0a19da90-880e-4eca-8cf7-6d7baf090d53 | Oracle ID: a1cc5e37-b09a-4b7f-afd5-77c1c35aa425
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARRION_FEEDER,
    oracle_id = "a1cc5e37-b09a-4b7f-afd5-77c1c35aa425",
    scryfall_id = "0a19da90-880e-4eca-8cf7-6d7baf090d53",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Carrion Feeder",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
