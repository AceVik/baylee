//! Kyren Archive — {3} — Artifact — Book
//! Oracle: At the beginning of your upkeep, you may exile the top card of your library face down.
//! Oracle: {5}, Discard your hand, Sacrifice this artifact: Put all cards exiled with this artifact into their owner's hand.
//! Set: MMQ #302 — Mercadian Masques | Scryfall ID: 5e65a06a-e7af-422a-9481-446731009935 | Oracle ID: f8c7da00-69e1-459d-9b66-6de649c5c511
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KYREN_ARCHIVE,
    oracle_id = "f8c7da00-69e1-459d-9b66-6de649c5c511",
    scryfall_id = "5e65a06a-e7af-422a-9481-446731009935",
    faces = &[face!(
        name = "Kyren Archive",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
