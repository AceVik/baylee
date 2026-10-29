//! Riptide Mangler — {1}{U} — Creature — Beast
//! Oracle: {1}{U}: This creature's base power becomes equal to target creature's power. (This effect lasts indefinitely.)
//! Set: LGN #51 — Legions | Scryfall ID: 5314a802-85d6-4d7b-ae9a-ca64eec652cf | Oracle ID: d5a4edda-fb04-43fa-8e0e-cc9f19b0776b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RIPTIDE_MANGLER,
    oracle_id = "d5a4edda-fb04-43fa-8e0e-cc9f19b0776b",
    scryfall_id = "5314a802-85d6-4d7b-ae9a-ca64eec652cf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Riptide Mangler",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(0),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
