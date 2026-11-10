//! Cyclone — {2}{G}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, put a wind counter on this enchantment, then sacrifice this enchantment unless you pay {G} for each wind counter on it. If you pay, this enchantment deals damage equal to the number of wind counters on it to each creature and each player.
//! Set: ME4 #148 — Masters Edition IV | Scryfall ID: e5b304be-916f-46bc-8b09-cf1bbeb1872b | Oracle ID: fc9e82b2-f148-431b-9312-97b7101cffe4
// IMPLEMENTED — upkeep wind counter, then pay {G} per counter or sacrifice;
// paid, damage equal to the counters to each creature and each player.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CYCLONE,
    oracle_id = "fc9e82b2-f148-431b-9312-97b7101cffe4",
    scryfall_id = "e5b304be-916f-46bc-8b09-cf1bbeb1872b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Cyclone",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[
            Effect::AddCounterSource {
                kind: counters::WIND,
                amount: Amount::Fixed(1)
            },
            Effect::PlayerMayPayManaEachThenElse {
                player: PlayerRel::You,
                each: mana!("{G}"),
                times: Amount::CountersOnSource(counters::WIND),
                paid: &[
                    Effect::DealDamageEach {
                        amount: Amount::CountersOnSource(counters::WIND),
                        filter: &Filter::CREATURE
                    },
                    Effect::DealDamage {
                        amount: Amount::CountersOnSource(counters::WIND),
                        target: TargetSpec::Player(PlayerRel::EachPlayer)
                    },
                ],
                unpaid: &[Effect::SacrificeSelf],
            },
        ]
    )],
);
