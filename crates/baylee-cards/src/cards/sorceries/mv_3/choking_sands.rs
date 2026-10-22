//! Choking Sands — {1}{B}{B} — Sorcery
//! Oracle: Destroy target non-Swamp land. If that land was nonbasic, Choking Sands deals 2 damage to the land's controller.
//! Set: VMA #109 — Vintage Masters | Scryfall ID: d63b5e4c-e6be-4df5-bb22-d0610067adf6 | Oracle ID: 3bcb7436-f7d9-4bb0-81be-7a3db9d8329f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NON_SWAMP_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::Not(&Filter::HasSubtype(subtypes::land::SWAMP)),
]);

const NONBASIC_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC)),
]);

card!(
    index = index::CHOKING_SANDS,
    oracle_id = "3bcb7436-f7d9-4bb0-81be-7a3db9d8329f",
    scryfall_id = "d63b5e4c-e6be-4df5-bb22-d0610067adf6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Choking Sands",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::IfTargetMatchesElse {
            filter: &NONBASIC_LAND,
            then: &[
                Effect::destroy(TargetSpec::Object(&NON_SWAMP_LAND)),
                Effect::DealDamageToTargetController {
                    amount: Amount::Fixed(2),
                },
            ],
            otherwise: &[Effect::destroy(TargetSpec::Object(&NON_SWAMP_LAND))],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&NON_SWAMP_LAND))),
    )],
);
