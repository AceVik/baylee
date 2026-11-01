//! Living Death — {3}{B}{B} — Sorcery
//! Oracle: Each player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield.
//! Set: TDC #185 — Tarkir: Dragonstorm Commander | Scryfall ID: 4ac29c46-f8ee-42ba-a98c-d785ba65485e | Oracle ID: 9e6a3df4-67a3-452e-a6ef-f04dbadb21ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIVING_DEATH,
    oracle_id = "9e6a3df4-67a3-452e-a6ef-f04dbadb21ef",
    scryfall_id = "4ac29c46-f8ee-42ba-a98c-d785ba65485e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Living Death",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
