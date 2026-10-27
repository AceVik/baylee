//! Bloodcurdler — {1}{B} — Creature — Horror
//! Oracle: Flying
//! Oracle: At the beginning of your upkeep, mill a card.
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1 and has "At the beginning of your end step, exile two cards from your graveyard."
//! Set: ODY #116 — Odyssey | Scryfall ID: dd1452ff-cec6-4df3-8bdc-bfb7869553a1 | Oracle ID: 3f53fd3e-9116-4599-81df-a9390461b549
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOODCURDLER,
    oracle_id = "3f53fd3e-9116-4599-81df-a9390461b549",
    scryfall_id = "dd1452ff-cec6-4df3-8bdc-bfb7869553a1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bloodcurdler",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
