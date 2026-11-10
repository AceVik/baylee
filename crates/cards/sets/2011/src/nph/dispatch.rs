//! Dispatch — {W} — Instant
//! Oracle: Tap target creature.
//! Oracle: Metalcraft — If you control three or more artifacts, exile that creature.
//! Set: MSC #130 — Marvel Super Heroes Commander | Scryfall ID: 99d30a21-b003-4a17-a4c5-97811f230896 | Oracle ID: 133c99c0-3652-410f-8100-68015a47af9f

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISPATCH,
    oracle_id = "133c99c0-3652-410f-8100-68015a47af9f",
    scryfall_id = "99d30a21-b003-4a17-a4c5-97811f230896",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dispatch",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    // Metalcraft is an ability word (CR 207.2c): the count is written out,
    // and it is asked as the spell resolves, so "that creature" is the one
    // just tapped whichever way the answer goes.
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            Effect::IfCondition {
                condition: Condition::ControlCount(&Filter::ARTIFACT, 3),
                then: &[Effect::exile(TargetSpec::Object(&Filter::CREATURE))],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
