//! Twisted Abomination — {5}{B} — Creature — Zombie Mutant
//! Oracle: {B}: Regenerate this creature. (The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Oracle: Swampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)
//! Set: CMM #194 — Commander Masters | Scryfall ID: 7bcaee2c-fad7-4ee2-8fe7-20b2c1eee5c5 | Oracle ID: 139af10d-a3e5-4ade-b876-87324e6c8017
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TWISTED_ABOMINATION,
    oracle_id = "139af10d-a3e5-4ade-b876-87324e6c8017",
    scryfall_id = "7bcaee2c-fad7-4ee2-8fe7-20b2c1eee5c5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Twisted Abomination",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::MUTANT],
        power = Some(5),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
