//! Metathran Soldier — {1}{U} — Creature — Metathran Soldier
//! Oracle: This creature can't be blocked.
//! Set: UDS #39 — Urza's Destiny | Scryfall ID: 650d40d0-78ec-4b6e-8ea0-28d43ce175d5 | Oracle ID: b723fcec-f208-4397-a178-e1e47b729cf1
// IMPLEMENTED — 1/1 Metathran Soldier that can't be blocked.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::METATHRAN_SOLDIER,
    oracle_id = "b723fcec-f208-4397-a178-e1e47b729cf1",
    scryfall_id = "650d40d0-78ec-4b6e-8ea0-28d43ce175d5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::UNBLOCKABLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Metathran Soldier",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::METATHRAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
);
