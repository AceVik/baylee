//! Two-Headed Giant of Foriys — {4}{R} — Creature — Giant
//! Oracle: Trample
//! Oracle: This creature can block an additional creature each combat.
//! Set: ME4 #139 — Masters Edition IV | Scryfall ID: 203186e6-843d-4296-ab27-10ac444e651c | Oracle ID: 38aa31bd-7145-43b9-9409-463d9ad6cd69
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TWO_HEADED_GIANT_OF_FORIYS,
    oracle_id = "38aa31bd-7145-43b9-9409-463d9ad6cd69",
    scryfall_id = "203186e6-843d-4296-ab27-10ac444e651c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Two-Headed Giant of Foriys",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
