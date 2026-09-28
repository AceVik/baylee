//! Massacre Wurm — {3}{B}{B}{B} — Creature — Phyrexian Wurm
//! Oracle: When this creature enters, creatures your opponents control get -2/-2 until end of turn.
//! Oracle: Whenever a creature an opponent controls dies, that player loses 2 life.
//! Set: FDN #754 — Foundations | Scryfall ID: 670a36cc-34e1-4d11-808e-1b6bc88eb5d8 | Oracle ID: 93cf50cf-0ecc-4d3e-abea-778c1ebacec4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MASSACRE_WURM,
    oracle_id = "93cf50cf-0ecc-4d3e-abea-778c1ebacec4",
    scryfall_id = "670a36cc-34e1-4d11-808e-1b6bc88eb5d8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Massacre Wurm",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::WURM],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
