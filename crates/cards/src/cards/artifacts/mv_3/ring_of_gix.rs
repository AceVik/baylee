//! Ring of Gix — {3} — Artifact
//! Oracle: Echo {3} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: {1}, {T}: Tap target artifact, creature, or land.
//! Set: VMA #280 — Vintage Masters | Scryfall ID: b3e75332-4f26-42c9-818f-f8f9396b4210 | Oracle ID: e01fe9e9-a76c-4909-924c-a0cba403f975
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const ARTIFACT_CREATURE_OR_LAND: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);

card!(
    index = index::RING_OF_GIX,
    oracle_id = "e01fe9e9-a76c-4909-924c-a0cba403f975",
    scryfall_id = "b3e75332-4f26-42c9-818f-f8f9396b4210",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ring of Gix",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        AbilityDef::Echo { cost: mana!("{3}") },
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::TapTarget],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &ARTIFACT_CREATURE_OR_LAND,
            ))),
        ),
    ],
);
