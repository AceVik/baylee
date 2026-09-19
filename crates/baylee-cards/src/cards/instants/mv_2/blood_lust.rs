//! Blood Lust — {1}{R} — Instant
//! Oracle: If target creature has toughness 5 or greater, it gets +4/-4 until end of turn. Otherwise, it gets +4/-X until end of turn, where X is its toughness minus 1.
//! Set: ME3 #88 — Masters Edition III | Scryfall ID: 51d30379-bf83-4334-8556-bc23a81dbcd1 | Oracle ID: 55a253f9-f44d-4898-af83-fea68b7b279d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOOD_LUST,
    oracle_id = "55a253f9-f44d-4898-af83-fea68b7b279d",
    scryfall_id = "51d30379-bf83-4334-8556-bc23a81dbcd1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blood Lust",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
