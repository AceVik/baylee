//! Nightmare — {5}{B} — Creature — Nightmare Horse
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: Nightmare's power and toughness are each equal to the number of Swamps you control.
//! Set: W17 #17 — Welcome Deck 2017 | Scryfall ID: 052022ff-795f-4f50-a45c-91cf8be9fbe9 | Oracle ID: 375932e6-1b3e-48dc-8154-9b664c3add34
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NIGHTMARE,
    oracle_id = "375932e6-1b3e-48dc-8154-9b664c3add34",
    scryfall_id = "052022ff-795f-4f50-a45c-91cf8be9fbe9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nightmare",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORSE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
