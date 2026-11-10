//! Riptide Shapeshifter — {3}{U}{U} — Creature — Shapeshifter
//! Oracle: {2}{U}{U}, Sacrifice this creature: Choose a creature type. Reveal cards from the top of your library until you reveal a creature card of that type. Put that card onto the battlefield and shuffle the rest into your library.
//! Set: ONS #109 — Onslaught | Scryfall ID: 85be34ac-7bc2-4da2-8d9c-2412b9946073 | Oracle ID: 49f292ab-0533-436b-a309-8bc795b66e5c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RIPTIDE_SHAPESHIFTER,
    oracle_id = "49f292ab-0533-436b-a309-8bc795b66e5c",
    scryfall_id = "85be34ac-7bc2-4da2-8d9c-2412b9946073",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Riptide Shapeshifter",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
