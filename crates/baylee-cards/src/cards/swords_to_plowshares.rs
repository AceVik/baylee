//! Swords to Plowshares — {W} — Instant
//! Oracle: Exile target creature. Its controller gains life equal to its power.
//! Set: MSC #143 — Marvel Super Heroes Commander | Scryfall ID: b4e9c870-23c0-413a-ae39-265f09da16d1 | Oracle ID: b1544f21-7e98-461b-aed5-e748b0168c52
// IMPLEMENTED — exile removal + controller gains power as life.

use baylee_cards_dsl::prelude::*;

card! {
    index: 164,
    oracle_id: "b1544f21-7e98-461b-aed5-e748b0168c52",
    scryfall_id: "b4e9c870-23c0-413a-ae39-265f09da16d1",
    faces: &[face! {
        name: "Swords to Plowshares",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::Exile {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
            Effect::GainLifeFor {
                amount: Amount::TargetPower,
                who: PlayerRel::ControllerOfTarget,
            },
        ], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))],
}

// Engine-level coverage via s4 scenario tests: the creature is exiled
// (not destroyed) and its controller gains life equal to its power.
