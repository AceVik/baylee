//! Mikaeus, the Unhallowed — {3}{B}{B}{B} — Legendary Creature — Zombie Cleric
//! Oracle: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
//! Oracle: Whenever a Human deals damage to you, destroy it.
//! Oracle: Other non-Human creatures you control get +1/+1 and have undying. (When a creature with undying dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)
//! Set: CMM #173 — Commander Masters | Scryfall ID: bc1f42a2-fe11-45da-9552-069803b4068a | Oracle ID: 5d27c63e-d1ef-48af-b51d-01ebc6daeac9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIKAEUS_THE_UNHALLOWED,
    oracle_id = "5d27c63e-d1ef-48af-b51d-01ebc6daeac9",
    scryfall_id = "bc1f42a2-fe11-45da-9552-069803b4068a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Mikaeus, the Unhallowed",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::CLERIC],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
