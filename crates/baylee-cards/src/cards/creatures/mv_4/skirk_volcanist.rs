//! Skirk Volcanist — {3}{R} — Creature — Goblin
//! Oracle: Morph—Sacrifice two Mountains. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, it deals 3 damage divided as you choose among one, two, or three target creatures.
//! Set: SCG #104 — Scourge | Scryfall ID: 8cdfb7e3-e077-400a-868d-3f3811e7a35c | Oracle ID: cbcf6924-3e97-4940-867a-3cedd4052c9f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKIRK_VOLCANIST,
    oracle_id = "cbcf6924-3e97-4940-867a-3cedd4052c9f",
    scryfall_id = "8cdfb7e3-e077-400a-868d-3f3811e7a35c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skirk Volcanist",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
