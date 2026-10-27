//! Wayward Angel — {4}{W}{W} — Creature — Angel Horror
//! Oracle: Flying, vigilance
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature gets +3/+3, is black, has trample, and has "At the beginning of your upkeep, sacrifice a creature."
//! Set: ODY #57 — Odyssey | Scryfall ID: 1fb726e8-162d-4143-9778-32476c0e1ab1 | Oracle ID: bf169a7a-e3cf-44b2-83eb-c2597bcf22d9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WAYWARD_ANGEL,
    oracle_id = "bf169a7a-e3cf-44b2-83eb-c2597bcf22d9",
    scryfall_id = "1fb726e8-162d-4143-9778-32476c0e1ab1",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wayward Angel",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL, subtypes::creature::HORROR],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
