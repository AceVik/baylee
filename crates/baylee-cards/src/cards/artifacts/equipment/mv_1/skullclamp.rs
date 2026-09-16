//! Skullclamp — {1} — Artifact — Equipment
//! Oracle: Equipped creature gets +1/-1.
//! Oracle: Whenever equipped creature dies, draw two cards.
//! Oracle: Equip {1}
//! Set: MSC #210 — Marvel Super Heroes Commander | Scryfall ID: 1d8b007b-3169-4ee3-80c7-781fc096fc7a | Oracle ID: 65986c1b-8e51-4604-b685-d82fa7d1263a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKULLCLAMP,
    oracle_id = "65986c1b-8e51-4604-b685-d82fa7d1263a",
    scryfall_id = "1d8b007b-3169-4ee3-80c7-781fc096fc7a",
    faces = &[face!(
        name = "Skullclamp",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
