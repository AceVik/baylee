//! Quiet Speculation — {1}{U} — Sorcery
//! Oracle: Search target player's library for up to three cards with flashback and put them into that player's graveyard. Then the player shuffles.
//! Set: EMA #68 — Eternal Masters | Scryfall ID: cd12fd06-26c5-42d8-936c-fe10b778a561 | Oracle ID: 938f48a9-766e-41d3-a425-9c3a6adf675b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::QUIET_SPECULATION,
    oracle_id = "938f48a9-766e-41d3-a425-9c3a6adf675b",
    scryfall_id = "cd12fd06-26c5-42d8-936c-fe10b778a561",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Quiet Speculation",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
