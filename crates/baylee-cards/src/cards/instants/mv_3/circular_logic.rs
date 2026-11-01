//! Circular Logic — {2}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {1} for each card in your graveyard.
//! Oracle: Madness {U} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: DMR #42 — Dominaria Remastered | Scryfall ID: 4b19fdac-da3e-4ebd-ae7b-6f31a97b2a0f | Oracle ID: b071bbaf-79fa-413b-807b-2cc86c8a8125
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CIRCULAR_LOGIC,
    oracle_id = "b071bbaf-79fa-413b-807b-2cc86c8a8125",
    scryfall_id = "4b19fdac-da3e-4ebd-ae7b-6f31a97b2a0f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Circular Logic",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
