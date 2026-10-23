//! Argothian Wurm — {3}{G} — Creature — Wurm
//! Oracle: Trample
//! Oracle: When this creature enters, any player may sacrifice a land of their choice. If a player does, put this creature on top of its owner's library.
//! Set: USG #236 — Urza's Saga | Scryfall ID: 93294349-75ae-4a6b-896d-b403a5d69e98 | Oracle ID: f8aa1893-60e5-439e-9316-872a9f6b6c94
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARGOTHIAN_WURM,
    oracle_id = "f8aa1893-60e5-439e-9316-872a9f6b6c94",
    scryfall_id = "93294349-75ae-4a6b-896d-b403a5d69e98",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Argothian Wurm",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
