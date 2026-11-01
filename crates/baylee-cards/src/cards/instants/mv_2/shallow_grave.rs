//! Shallow Grave — {1}{B} — Instant
//! Oracle: Return the top creature card of your graveyard to the battlefield. That creature gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: MIR #141 — Mirage | Scryfall ID: d5c782cc-c951-4c6f-a93f-774ae6c1c214 | Oracle ID: 9843d8a6-bcbf-40c2-8c41-316e97db9c62
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHALLOW_GRAVE,
    oracle_id = "9843d8a6-bcbf-40c2-8c41-316e97db9c62",
    scryfall_id = "d5c782cc-c951-4c6f-a93f-774ae6c1c214",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shallow Grave",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
