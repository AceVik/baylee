//! Treespring Lorian — {5}{G} — Creature — Beast
//! Oracle: Morph {5}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #293 — Onslaught | Scryfall ID: f525d7ce-37d3-4989-beb4-173447cb5294 | Oracle ID: 6f3ed8a1-ff5f-411b-9d87-0cce52f42344
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TREESPRING_LORIAN,
    oracle_id = "6f3ed8a1-ff5f-411b-9d87-0cce52f42344",
    scryfall_id = "f525d7ce-37d3-4989-beb4-173447cb5294",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Treespring Lorian",
        mana_cost = mana!("{5}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
