//! Cinder Cloud — {3}{R}{R} — Instant
//! Oracle: Destroy target creature. If a white creature dies this way, Cinder Cloud deals damage to that creature's controller equal to the creature's power.
//! Set: MIR #165 — Mirage | Scryfall ID: f044c470-50ce-4a6c-b8ab-665357c3c11e | Oracle ID: f64ea1a7-0e10-4549-8056-25ebc2ce0175
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::CINDER_CLOUD,
    oracle_id = "f64ea1a7-0e10-4549-8056-25ebc2ce0175",
    scryfall_id = "f044c470-50ce-4a6c-b8ab-665357c3c11e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cinder Cloud",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::CREATURE)),
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Died(&WHITE_CREATURE)),
                    1,
                ),
                then: &[Effect::DealDamageToTargetController {
                    amount: Amount::TargetPower,
                }],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
