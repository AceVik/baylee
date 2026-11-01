//! Planar Birth — {1}{W} — Sorcery
//! Oracle: Return all basic land cards from all graveyards to the battlefield tapped under their owners' control.
//! Set: USG #31 — Urza's Saga | Scryfall ID: c7cacdff-aa83-4644-b2f0-ce8c89dddfbf | Oracle ID: 7a9d13b3-3823-463a-acd9-8b7a9d5e121f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_BIRTH,
    oracle_id = "7a9d13b3-3823-463a-acd9-8b7a9d5e121f",
    scryfall_id = "c7cacdff-aa83-4644-b2f0-ce8c89dddfbf",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Planar Birth",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
