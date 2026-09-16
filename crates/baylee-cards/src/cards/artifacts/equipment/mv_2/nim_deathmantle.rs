//! Nim Deathmantle — {2} — Artifact — Equipment
//! Oracle: Equipped creature gets +2/+2, has intimidate, and is a black Zombie. (A creature with intimidate can't be blocked except by artifact creatures and/or creatures that share a color with it.)
//! Oracle: Whenever a nontoken creature is put into your graveyard from the battlefield, you may pay {4}. If you do, return that card to the battlefield and attach this Equipment to it.
//! Oracle: Equip {4}
//! Set: 2X2 #309 — Double Masters 2022 | Scryfall ID: 787b1cc8-42b4-4d3e-9b8a-a252de297b1a | Oracle ID: 66d41377-626d-4ae6-ba86-17bf0c8b3362
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NIM_DEATHMANTLE,
    oracle_id = "66d41377-626d-4ae6-ba86-17bf0c8b3362",
    scryfall_id = "787b1cc8-42b4-4d3e-9b8a-a252de297b1a",
    faces = &[face!(
        name = "Nim Deathmantle",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
