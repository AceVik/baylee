//! Zombie Master — {1}{B}{B} — Creature — Zombie
//! Oracle: Other Zombie creatures have swampwalk. (They can't be blocked as long as defending player controls a Swamp.)
//! Oracle: Other Zombies have "{B}: Regenerate this permanent."
//! Set: ME4 #105 — Masters Edition IV | Scryfall ID: c25eb8c9-4209-4fe4-8b02-be16d7d7bdf5 | Oracle ID: 5446c92f-ff22-4e9b-a2f6-e64c8560c1e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_MASTER,
    oracle_id = "5446c92f-ff22-4e9b-a2f6-e64c8560c1e0",
    scryfall_id = "c25eb8c9-4209-4fe4-8b02-be16d7d7bdf5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Zombie Master",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
