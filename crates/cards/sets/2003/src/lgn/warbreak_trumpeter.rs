//! Warbreak Trumpeter — {R} — Creature — Goblin
//! Oracle: Morph {X}{X}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, create X 1/1 red Goblin creature tokens.
//! Set: LGN #116 — Legions | Scryfall ID: fc942957-1067-428c-8ee1-01f9e260efe1 | Oracle ID: 40e5c7fe-4d7a-41c0-8b90-ea0b46a01c9d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARBREAK_TRUMPETER,
    oracle_id = "40e5c7fe-4d7a-41c0-8b90-ea0b46a01c9d",
    scryfall_id = "fc942957-1067-428c-8ee1-01f9e260efe1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Warbreak Trumpeter",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
