//! Daru Stinger — {3}{W} — Creature — Soldier
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Soldier card you reveal in your hand.)
//! Oracle: {T}: This creature deals damage equal to the number of +1/+1 counters on it to target attacking or blocking creature.
//! Set: LGN #10 — Legions | Scryfall ID: ff5866a4-f4c0-45bc-9b33-b77387441d34 | Oracle ID: 696bff10-4e46-48a7-b2af-13387b9d9da0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARU_STINGER,
    oracle_id = "696bff10-4e46-48a7-b2af-13387b9d9da0",
    scryfall_id = "ff5866a4-f4c0-45bc-9b33-b77387441d34",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daru Stinger",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
