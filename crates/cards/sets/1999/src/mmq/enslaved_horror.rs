//! Enslaved Horror — {3}{B} — Creature — Horror
//! Oracle: When this creature enters, each other player may return a creature card from their graveyard to the battlefield.
//! Set: MMQ #134 — Mercadian Masques | Scryfall ID: dffca723-360d-48de-a0a8-32288627f3df | Oracle ID: f7920a2c-d14a-4ed6-8341-96751322f4e7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENSLAVED_HORROR,
    oracle_id = "f7920a2c-d14a-4ed6-8341-96751322f4e7",
    scryfall_id = "dffca723-360d-48de-a0a8-32288627f3df",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Enslaved Horror",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
