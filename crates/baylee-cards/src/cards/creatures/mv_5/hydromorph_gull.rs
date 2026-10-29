//! Hydromorph Gull — {3}{U}{U} — Creature — Elemental Bird
//! Oracle: Flying
//! Oracle: {U}, Sacrifice this creature: Counter target spell that targets a creature you control.
//! Set: TOR #40 — Torment | Scryfall ID: 06b1c8fc-e09b-479b-ac15-f659acb2f50f | Oracle ID: 124fea1b-c0e9-4e56-b026-82dddee0d36e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HYDROMORPH_GULL,
    oracle_id = "124fea1b-c0e9-4e56-b026-82dddee0d36e",
    scryfall_id = "06b1c8fc-e09b-479b-ac15-f659acb2f50f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hydromorph Gull",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::BIRD],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
