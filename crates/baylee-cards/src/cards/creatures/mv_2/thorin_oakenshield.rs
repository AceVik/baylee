//! Thorin Oakenshield — {R}{W} — Legendary Creature — Dwarf Noble
//! Oracle: Trample
//! Oracle: Storied (If you control three or more artifacts, legendaries, and/or Sagas, you have an enduring story for the rest of the game.)
//! Oracle: As long as you have an enduring story, artifacts and creatures you control have ward {1}.
//! Set: HOB #165 — The Hobbit | Scryfall ID: c7e18609-d1ed-4829-be11-f2ce2cfcbc49 | Oracle ID: bdd41af0-bbd1-4ecd-a699-99f006f5e5ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THORIN_OAKENSHIELD,
    oracle_id = "bdd41af0-bbd1-4ecd-a699-99f006f5e5ce",
    scryfall_id = "c7e18609-d1ed-4829-be11-f2ce2cfcbc49",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Thorin Oakenshield",
        mana_cost = mana!("{R}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::NOBLE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
