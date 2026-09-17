//! Thopter Foundry — {W/B}{U} — Artifact
//! Oracle: {1}, Sacrifice a nontoken artifact: Create a 1/1 blue Thopter artifact creature token with flying. You gain 1 life.
//! Set: 2XM #222 — Double Masters | Scryfall ID: 3d8dcd5e-d4d0-49ae-a591-78d80322105d | Oracle ID: 88bef744-550e-4f33-b1ff-a8ee990ec754
// PARTIAL — the {1}, Sacrifice a nontoken artifact activation and its
// "You gain 1 life." half are built; the Thopter has no entry in
// `crate::tokens`, so the create half is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THOPTER_FOUNDRY,
    oracle_id = "88bef744-550e-4f33-b1ff-a8ee990ec754",
    scryfall_id = "3d8dcd5e-d4d0-49ae-a591-78d80322105d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    coverage = Coverage::Partial("no Thopter token in `crate::tokens` to name in CreateToken"),
    faces = &[face!(
        name = "Thopter Foundry",
        mana_cost = mana!("{W/B}{U}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", Sacrifice(&f!(nontoken ARTIFACT))),
        // NOT SUPPORTED: "Create a 1/1 blue Thopter artifact creature token
        // with flying." — Effect::CreateToken names a &'static TokenDef and
        // the pool has no Thopter; a card file may not declare one either
        // (`crate::tokens` owns every TokenDef, and the id in `ALL` is the
        // token's art key).
        &[Effect::gain_life(1)]
    )],
);
