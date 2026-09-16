//! Delighted Halfling — {G} — Creature — Halfling Citizen
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a legendary spell, and that spell can't be countered.
//! Set: LTR #158 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 71384418-173a-4f77-adab-56e52fa23692 | Oracle ID: f9d3b046-0b95-4103-a630-4b3fb88bb60b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DELIGHTED_HALFLING,
    oracle_id = "f9d3b046-0b95-4103-a630-4b3fb88bb60b",
    scryfall_id = "71384418-173a-4f77-adab-56e52fa23692",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Delighted Halfling",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HALFLING, subtypes::creature::CITIZEN],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
