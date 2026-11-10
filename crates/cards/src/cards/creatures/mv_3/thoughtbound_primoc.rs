//! Thoughtbound Primoc — {2}{R} — Creature — Bird Beast
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, if a player controls more Wizards than each other player, the player who controls the most Wizards gains control of this creature.
//! Set: ONS #240 — Onslaught | Scryfall ID: e89156b5-8bdb-41d1-a7aa-63f770a9b070 | Oracle ID: abedc360-e1dc-46ca-9ea6-5bb154ece547
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THOUGHTBOUND_PRIMOC,
    oracle_id = "abedc360-e1dc-46ca-9ea6-5bb154ece547",
    scryfall_id = "e89156b5-8bdb-41d1-a7aa-63f770a9b070",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Thoughtbound Primoc",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
