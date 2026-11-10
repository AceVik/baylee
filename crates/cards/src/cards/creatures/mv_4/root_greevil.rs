//! Root Greevil — {3}{G} — Creature — Beast
//! Oracle: {2}{G}, {T}, Sacrifice this creature: Destroy all enchantments of the color of your choice.
//! Set: PLS #91 — Planeshift | Scryfall ID: 306e3429-b3b4-4186-935b-18cfc308d22c | Oracle ID: e43b25be-fe67-4eaa-80df-85718f38f904
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOT_GREEVIL,
    oracle_id = "e43b25be-fe67-4eaa-80df-85718f38f904",
    scryfall_id = "306e3429-b3b4-4186-935b-18cfc308d22c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Root Greevil",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
