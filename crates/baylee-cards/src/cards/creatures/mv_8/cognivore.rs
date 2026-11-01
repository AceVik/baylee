//! Cognivore — {6}{U}{U} — Creature — Lhurgoyf
//! Oracle: Flying
//! Oracle: Cognivore's power and toughness are each equal to the number of instant cards in all graveyards.
//! Set: ODY #77 — Odyssey | Scryfall ID: 7de76ff6-d065-4db1-b28d-4a4ccb1cc0fa | Oracle ID: 8b4aae48-be01-4b42-8d32-45071650da03
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COGNIVORE,
    oracle_id = "8b4aae48-be01-4b42-8d32-45071650da03",
    scryfall_id = "7de76ff6-d065-4db1-b28d-4a4ccb1cc0fa",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cognivore",
        mana_cost = mana!("{6}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LHURGOYF],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
