//! Riptide Chronologist — {3}{U}{U} — Creature — Human Wizard
//! Oracle: {U}, Sacrifice this creature: Untap all creatures of the creature type of your choice.
//! Set: ONS #107 — Onslaught | Scryfall ID: 3767f568-36b1-4064-835e-4dd7576b7b8b | Oracle ID: e1c6e606-c4e4-4067-8cb9-46753785dfeb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RIPTIDE_CHRONOLOGIST,
    oracle_id = "e1c6e606-c4e4-4067-8cb9-46753785dfeb",
    scryfall_id = "3767f568-36b1-4064-835e-4dd7576b7b8b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Riptide Chronologist",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
