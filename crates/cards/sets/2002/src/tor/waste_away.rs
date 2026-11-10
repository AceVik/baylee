//! Waste Away — {4}{B} — Instant
//! Oracle: As an additional cost to cast this spell, discard a card.
//! Oracle: Target creature gets -5/-5 until end of turn.
//! Set: TOR #88 — Torment | Scryfall ID: 78e859c2-3457-4c4c-9b1c-0db647dcf259 | Oracle ID: d7f25f7f-3fa2-47ba-be33-ba0b08f26c94
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WASTE_AWAY,
    oracle_id = "d7f25f7f-3fa2-47ba-be33-ba0b08f26c94",
    scryfall_id = "78e859c2-3457-4c4c-9b1c-0db647dcf259",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Waste Away",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
