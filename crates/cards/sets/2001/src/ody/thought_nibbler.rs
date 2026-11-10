//! Thought Nibbler — {U} — Creature — Beast
//! Oracle: Flying
//! Oracle: Your maximum hand size is reduced by two.
//! Set: ODY #107 — Odyssey | Scryfall ID: 7284a7fd-cda8-43ac-b119-ad47b33c2ec4 | Oracle ID: 9a640695-fc17-4ccc-ac88-ce48b6c6ad53
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THOUGHT_NIBBLER,
    oracle_id = "9a640695-fc17-4ccc-ac88-ce48b6c6ad53",
    scryfall_id = "7284a7fd-cda8-43ac-b119-ad47b33c2ec4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Thought Nibbler",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
