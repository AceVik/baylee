//! Boneknitter — {1}{B} — Creature — Zombie Cleric
//! Oracle: {1}{B}: Regenerate target Zombie.
//! Oracle: Morph {2}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #128 — Onslaught | Scryfall ID: c9d58030-a95a-4221-93bc-30a59344e30b | Oracle ID: 34b2f465-0b15-428a-a1c2-bcd41af07d01
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BONEKNITTER,
    oracle_id = "34b2f465-0b15-428a-a1c2-bcd41af07d01",
    scryfall_id = "c9d58030-a95a-4221-93bc-30a59344e30b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Boneknitter",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
