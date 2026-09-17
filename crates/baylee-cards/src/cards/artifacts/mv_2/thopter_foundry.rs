//! Thopter Foundry — {W/B}{U} — Artifact
//! Oracle: {1}, Sacrifice a nontoken artifact: Create a 1/1 blue Thopter artifact creature token with flying. You gain 1 life.
//! Set: 2XM #222 — Double Masters | Scryfall ID: 3d8dcd5e-d4d0-49ae-a591-78d80322105d | Oracle ID: 88bef744-550e-4f33-b1ff-a8ee990ec754
// IMPLEMENTED — {1}, sacrifice a nontoken artifact → a blue Thopter and a life.

use baylee_cards_dsl::prelude::*;

// The ledger and not `crate::tokens`, because a reader wrote this one: the
// pool reaches for the Thopter through this very card's reference script.
use crate::generated_tokens::THOPTER_ARTIFACT_1_1_BLUE_FLYING as THOPTER;

card!(
    index = index::THOPTER_FOUNDRY,
    oracle_id = "88bef744-550e-4f33-b1ff-a8ee990ec754",
    scryfall_id = "3d8dcd5e-d4d0-49ae-a591-78d80322105d",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Thopter Foundry",
        mana_cost = mana!("{W/B}{U}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", Sacrifice(&f!(nontoken ARTIFACT))),
        // In the printed order: the Thopter, then the life. The token comes
        // out of the ledger rather than out of this file — a `TokenDef`
        // written here would have no place in `tokens::ALL`, and that place
        // is the id a client keys its art off.
        &[
            Effect::CreateToken { token: &THOPTER },
            Effect::gain_life(1)
        ]
    )],
);
