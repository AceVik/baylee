//! Rockshard Elemental — {5}{R}{R} — Creature — Elemental
//! Oracle: Double strike (This creature deals both first-strike and regular combat damage.)
//! Oracle: Morph {4}{R}{R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: LGN #109 — Legions | Scryfall ID: 2d6343c0-3fb5-4bac-bea7-cba36498cd69 | Oracle ID: 82cfba28-283f-44f6-8478-fd39ea5a1a29
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROCKSHARD_ELEMENTAL,
    oracle_id = "82cfba28-283f-44f6-8478-fd39ea5a1a29",
    scryfall_id = "2d6343c0-3fb5-4bac-bea7-cba36498cd69",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Rockshard Elemental",
        mana_cost = mana!("{5}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
