//! Thopter Foundry — {W/B}{U} — Artifact
//! Oracle: {1}, Sacrifice a nontoken artifact: Create a 1/1 blue Thopter artifact creature token with flying. You gain 1 life.
//! Set: 2XM #222 — Double Masters | Scryfall ID: 3d8dcd5e-d4d0-49ae-a591-78d80322105d | Oracle ID: 88bef744-550e-4f33-b1ff-a8ee990ec754
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THOPTER_FOUNDRY,
    oracle_id = "88bef744-550e-4f33-b1ff-a8ee990ec754",
    scryfall_id = "3d8dcd5e-d4d0-49ae-a591-78d80322105d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Thopter Foundry",
        mana_cost = mana!("{W/B}{U}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
