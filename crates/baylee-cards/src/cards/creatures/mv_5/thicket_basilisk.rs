//! Thicket Basilisk — {3}{G}{G} — Creature — Basilisk
//! Oracle: Whenever this creature blocks or becomes blocked by a non-Wall creature, destroy that creature at end of combat.
//! Set: ME1 #134 — Masters Edition | Scryfall ID: 2ac6ded0-4ff0-4827-a870-1a7851cd37b0 | Oracle ID: c4822813-cd81-465d-9fe8-3a4c2dcd31ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THICKET_BASILISK,
    oracle_id = "c4822813-cd81-465d-9fe8-3a4c2dcd31ef",
    scryfall_id = "2ac6ded0-4ff0-4827-a870-1a7851cd37b0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Thicket Basilisk",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BASILISK],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
