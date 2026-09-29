//! Goblin King — {1}{R}{R} — Creature — Goblin
//! Oracle: Other Goblins get +1/+1 and have mountainwalk.
//! Set: 10E #207 — Tenth Edition | Scryfall ID: 4b266935-25ea-49c5-a1da-57c22a4362fd | Oracle ID: d236b3fc-0d3f-4d99-875d-e32a33fe5767
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_KING,
    oracle_id = "d236b3fc-0d3f-4d99-875d-e32a33fe5767",
    scryfall_id = "4b266935-25ea-49c5-a1da-57c22a4362fd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin King",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
