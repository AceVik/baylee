//! Laquatus's Disdain — {1}{U} — Instant
//! Oracle: Counter target spell cast from a graveyard.
//! Oracle: Draw a card.
//! Set: JUD #44 — Judgment | Scryfall ID: e2ea5448-2d72-42eb-814c-197153d8e06a | Oracle ID: 68f330ca-8279-46c5-900d-a9b443676104
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAQUATUS_S_DISDAIN,
    oracle_id = "68f330ca-8279-46c5-900d-a9b443676104",
    scryfall_id = "e2ea5448-2d72-42eb-814c-197153d8e06a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Laquatus's Disdain",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
