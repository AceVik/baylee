//! Shivan Dragon — {4}{R}{R} — Creature — Dragon
//! Oracle: Flying
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: FDN #763 — Foundations | Scryfall ID: 702c4781-670b-49ae-b511-90ed119841b0 | Oracle ID: 711eea87-0fa3-46e0-a42b-fa5a86455f04
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIVAN_DRAGON,
    oracle_id = "711eea87-0fa3-46e0-a42b-fa5a86455f04",
    scryfall_id = "702c4781-670b-49ae-b511-90ed119841b0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Shivan Dragon",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
