//! Bitter Triumph — {1}{B} — Instant
//! Oracle: As an additional cost to cast this spell, discard a card or pay 3 life.
//! Oracle: Destroy target creature or planeswalker.
//! Set: TRC #445 — Star Trek Commander | Scryfall ID: cfe118f0-f727-41a6-aaaf-9638fe1dad49 | Oracle ID: 776341cb-d2ec-423f-9250-92dc8bd8d503
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BITTER_TRIUMPH,
    oracle_id = "776341cb-d2ec-423f-9250-92dc8bd8d503",
    scryfall_id = "cfe118f0-f727-41a6-aaaf-9638fe1dad49",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bitter Triumph",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
