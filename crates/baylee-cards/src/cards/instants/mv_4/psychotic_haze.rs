//! Psychotic Haze — {2}{B}{B} — Instant
//! Oracle: Psychotic Haze deals 1 damage to each creature and each player.
//! Oracle: Madness {1}{B} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: TOR #76 — Torment | Scryfall ID: 8d3f6cd2-0138-40e7-a975-3f7c68db0d93 | Oracle ID: 4b6aa5e2-b539-46cd-804b-cd9e996a9456
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHOTIC_HAZE,
    oracle_id = "4b6aa5e2-b539-46cd-804b-cd9e996a9456",
    scryfall_id = "8d3f6cd2-0138-40e7-a975-3f7c68db0d93",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Psychotic Haze",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
