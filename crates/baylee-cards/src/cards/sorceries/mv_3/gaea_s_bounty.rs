//! Gaea's Bounty — {2}{G} — Sorcery
//! Oracle: Search your library for up to two Forest cards, reveal those cards, put them into your hand, then shuffle.
//! Set: USG #254 — Urza's Saga | Scryfall ID: 66b55e37-fcad-4d50-89d4-5d88269fee66 | Oracle ID: c5a61e61-d5fe-41ef-afd6-ba5e0399b986
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAEA_S_BOUNTY,
    oracle_id = "c5a61e61-d5fe-41ef-afd6-ba5e0399b986",
    scryfall_id = "66b55e37-fcad-4d50-89d4-5d88269fee66",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gaea's Bounty",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::HasSubtype(subtypes::land::FOREST),
        finds: &[Find::HAND, Find::HAND],
        optional: true,
    }])],
);
