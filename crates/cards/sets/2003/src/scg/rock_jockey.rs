//! Rock Jockey — {2}{R} — Creature — Goblin
//! Oracle: You can't cast Rock Jockey if you've played a land this turn.
//! Oracle: You can't play lands if this creature was cast this turn.
//! Set: SCG #101 — Scourge | Scryfall ID: 8f836e90-3255-48bd-a174-6a127528551e | Oracle ID: 09fff6a3-6a54-4101-a587-55d7113b9639
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROCK_JOCKEY,
    oracle_id = "09fff6a3-6a54-4101-a587-55d7113b9639",
    scryfall_id = "8f836e90-3255-48bd-a174-6a127528551e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rock Jockey",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
