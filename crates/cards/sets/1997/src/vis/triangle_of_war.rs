//! Triangle of War — {1} — Artifact
//! Oracle: {2}, Sacrifice this artifact: Target creature you control fights target creature an opponent controls. (Each deals damage equal to its power to the other.)
//! Set: VMA #288 — Vintage Masters | Scryfall ID: f59f9255-8b76-43b6-9365-d41f9618c2c3 | Oracle ID: 6d4daf9c-e2f4-4d0e-b2e4-ded04d644b11
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRIANGLE_OF_WAR,
    oracle_id = "6d4daf9c-e2f4-4d0e-b2e4-ded04d644b11",
    scryfall_id = "f59f9255-8b76-43b6-9365-d41f9618c2c3",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Triangle of War",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", SacrificeSelf),
        &[Effect::Fight {
            fighter: TargetSlot::First,
            foe: TargetSlot::Second,
        }],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        second_targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::OPPONENT_CREATURE,
        ))),
    )],
);
