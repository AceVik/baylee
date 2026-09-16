//! Dwalin, Weaponmaster — {1}{R/W} — Legendary Creature — Dwarf Warrior
//! Oracle: First strike
//! Oracle: Whenever Dwalin enters or attacks, put a hone counter on each Equipment you control. (Each hone counter on an Equipment grants +1/+0 to equipped creature.)
//! Set: HOB #154 — The Hobbit | Scryfall ID: 196d9287-a37d-4b27-a83b-a5489a54f081 | Oracle ID: cee583b7-7cc3-40ea-a227-b760839ec291
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWALIN_WEAPONMASTER,
    oracle_id = "cee583b7-7cc3-40ea-a227-b760839ec291",
    scryfall_id = "196d9287-a37d-4b27-a83b-a5489a54f081",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Dwalin, Weaponmaster",
        mana_cost = mana!("{1}{R/W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::WARRIOR],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
