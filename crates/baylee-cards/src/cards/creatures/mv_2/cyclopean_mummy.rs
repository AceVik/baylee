//! Cyclopean Mummy — {1}{B} — Creature — Zombie
//! Oracle: When this creature dies, exile it.
//! Set: ME4 #72 — Masters Edition IV | Scryfall ID: 8de50323-7ce8-4ca6-a90b-b4be9d639752 | Oracle ID: f5f2c98c-dd89-4b60-83e5-07f5fdc699ef
// IMPLEMENTED — a dies trigger exiles the Mummy out of the graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CYCLOPEAN_MUMMY,
    oracle_id = "f5f2c98c-dd89-4b60-83e5-07f5fdc699ef",
    scryfall_id = "8de50323-7ce8-4ca6-a90b-b4be9d639752",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cyclopean Mummy",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    // The trigger's source is the card that died, so `ExileSource` reaches
    // it in the graveyard (Academy Rector's shape, without the "may").
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::ExileSource]
    )],
);
