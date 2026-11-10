//! King Crab — {4}{U}{U} — Creature — Crab
//! Oracle: {1}{U}, {T}: Put target green creature on top of its owner's library.
//! Set: ULG #34 — Urza's Legacy | Scryfall ID: aedea953-b5f1-4ec7-bd9f-b7827f9d40fe | Oracle ID: 24982bfe-5276-44a8-99a5-43b497ebf51f
// PARTIAL — putting a green creature on top of its owner's library is unsupported; ability omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KING_CRAB,
    oracle_id = "24982bfe-5276-44a8-99a5-43b497ebf51f",
    scryfall_id = "aedea953-b5f1-4ec7-bd9f-b7827f9d40fe",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Putting a target green creature on top of its owner's library is unsupported; the activated ability is omitted.",
    ),
    faces = &[face!(
        name = "King Crab",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CRAB],
        power = Some(4),
        toughness = Some(5),
    ),],
    // NOT SUPPORTED: Put target green creature on top of its owner's library.
);
