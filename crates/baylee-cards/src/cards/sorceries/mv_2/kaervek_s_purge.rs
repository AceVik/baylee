//! Kaervek's Purge — {X}{B}{R} — Sorcery
//! Oracle: Destroy target creature with mana value X. If that creature dies this way, Kaervek's Purge deals damage equal to the creature's power to the creature's controller.
//! Set: MIR #270 — Mirage | Scryfall ID: 7a42ef95-92ec-40fe-ab30-a476f012a525 | Oracle ID: daa86c76-aacb-4244-9639-81f758c634b2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const CREATURE_MV_X: Filter = Filter::And(&[Filter::CREATURE, Filter::CmcExactlyX]);

card!(
    index = index::KAERVEK_S_PURGE,
    oracle_id = "daa86c76-aacb-4244-9639-81f758c634b2",
    scryfall_id = "7a42ef95-92ec-40fe-ab30-a476f012a525",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kaervek's Purge",
        mana_cost = mana!("{X}{B}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&CREATURE_MV_X)),
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Died(&Filter::CREATURE)),
                    1,
                ),
                then: &[Effect::DealDamageToTargetController {
                    amount: Amount::TargetPower,
                }],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&CREATURE_MV_X))),
    )],
);
