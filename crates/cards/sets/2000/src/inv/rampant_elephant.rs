//! Rampant Elephant — {3}{W} — Creature — Elephant
//! Oracle: {G}: Target creature blocks this creature this turn if able.
//! Set: INV #28 — Invasion | Scryfall ID: 752642d2-3dad-4f58-b154-beb5982141dc | Oracle ID: aaaaf370-6486-4e17-9a93-92e05ebcf29f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAMPANT_ELEPHANT,
    oracle_id = "aaaaf370-6486-4e17-9a93-92e05ebcf29f",
    scryfall_id = "752642d2-3dad-4f58-b154-beb5982141dc",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Rampant Elephant",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEPHANT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
