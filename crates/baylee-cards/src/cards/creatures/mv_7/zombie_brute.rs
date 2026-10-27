//! Zombie Brute — {6}{B} — Creature — Zombie
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Zombie card you reveal in your hand.)
//! Oracle: Trample
//! Set: LGN #87 — Legions | Scryfall ID: b37db470-3aef-4fc4-98ce-63b5fb2546f6 | Oracle ID: 377177f3-65e5-4909-bf12-48a473b1861d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_BRUTE,
    oracle_id = "377177f3-65e5-4909-bf12-48a473b1861d",
    scryfall_id = "b37db470-3aef-4fc4-98ce-63b5fb2546f6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Zombie Brute",
        mana_cost = mana!("{6}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(5),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
