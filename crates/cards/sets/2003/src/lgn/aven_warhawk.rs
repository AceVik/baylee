//! Aven Warhawk — {4}{W} — Creature — Bird Soldier
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Bird and/or Soldier card you reveal in your hand.)
//! Oracle: Flying
//! Set: LGN #4 — Legions | Scryfall ID: 386a7062-6da8-4663-a218-75d894f7c0e0 | Oracle ID: 7d3e5d95-2658-4c1a-9ed3-df1f554d87ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVEN_WARHAWK,
    oracle_id = "7d3e5d95-2658-4c1a-9ed3-df1f554d87ea",
    scryfall_id = "386a7062-6da8-4663-a218-75d894f7c0e0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Warhawk",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
