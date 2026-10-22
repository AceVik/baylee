//! Reality Ripple — {1}{U} — Instant
//! Oracle: Target artifact, creature, or land phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: MIR #87 — Mirage | Scryfall ID: c5b94eed-2a45-4a6b-a06b-a2021b174bc5 | Oracle ID: 504d5c29-7c37-4c31-8549-2a49eeef74c8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const ARTIFACT_CREATURE_OR_LAND: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);

card!(
    index = index::REALITY_RIPPLE,
    oracle_id = "504d5c29-7c37-4c31-8549-2a49eeef74c8",
    scryfall_id = "c5b94eed-2a45-4a6b-a06b-a2021b174bc5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reality Ripple",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PhaseOut {
            target: Some(TargetSpec::Object(&ARTIFACT_CREATURE_OR_LAND)),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_CREATURE_OR_LAND
        ))),
    )],
);
