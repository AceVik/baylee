//! Thrive — {X}{G} — Sorcery
//! Oracle: Put a +1/+1 counter on each of X target creatures.
//! Set: 2X2 #161 — Double Masters 2022 | Scryfall ID: 43c8bdb1-fa35-462e-ac3f-6953fe7f3476 | Oracle ID: fe3c524e-10b7-485a-9285-973092eeb214
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRIVE,
    oracle_id = "fe3c524e-10b7-485a-9285-973092eeb214",
    scryfall_id = "43c8bdb1-fa35-462e-ac3f-6953fe7f3476",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Thrive",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
