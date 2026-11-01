//! Dimensional Breach — {5}{W}{W} — Sorcery
//! Oracle: Exile all permanents. For as long as any of those cards remain exiled, at the beginning of each player's upkeep, that player returns one of the exiled cards they own to the battlefield.
//! Set: SCG #9 — Scourge | Scryfall ID: f18f2832-07c5-47be-8966-b250fb997f78 | Oracle ID: 70d4f764-fca1-477a-9b0f-bbfcc1f7dfb1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIMENSIONAL_BREACH,
    oracle_id = "70d4f764-fca1-477a-9b0f-bbfcc1f7dfb1",
    scryfall_id = "f18f2832-07c5-47be-8966-b250fb997f78",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dimensional Breach",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
