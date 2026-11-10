//! Cephalid Sage — {3}{U} — Creature — Octopus
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "When this creature enters, draw three cards, then discard two cards."
//! Set: EMA #41 — Eternal Masters | Scryfall ID: f4186eb4-5c79-4616-9a20-c48a6c3f5a82 | Oracle ID: 29182a24-3bdb-4fd0-8a1d-a196340c0858
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CEPHALID_SAGE,
    oracle_id = "29182a24-3bdb-4fd0-8a1d-a196340c0858",
    scryfall_id = "f4186eb4-5c79-4616-9a20-c48a6c3f5a82",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cephalid Sage",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OCTOPUS],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
