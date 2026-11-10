//! Primordial Ooze — {R} — Creature — Ooze
//! Oracle: This creature attacks each combat if able.
//! Oracle: At the beginning of your upkeep, put a +1/+1 counter on this creature. Then you may pay {X}, where X is the number of +1/+1 counters on it. If you don't, tap this creature and it deals X damage to you.
//! Set: 5ED #261 — Fifth Edition | Scryfall ID: a53d8d6d-b8d3-4f71-a88a-5d639ce2925f | Oracle ID: 133109c3-a2ca-4231-b3b1-baa700800b07
// IMPLEMENTED — must attack, and the upkeep trigger adds the counter, taxes
// an X equal to the counters on it, and taps and damages you when unpaid.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PRIMORDIAL_OOZE,
    oracle_id = "133109c3-a2ca-4231-b3b1-baa700800b07",
    scryfall_id = "a53d8d6d-b8d3-4f71-a88a-5d639ce2925f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Primordial Ooze",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OOZE],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::AttacksEachCombat),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayOr {
                    player: PlayerRel::You,
                    mana: Amount::CountersOnSource(CounterKind::P1P1),
                    effect: &Effect::Sequence(&[
                        Effect::TapSelf,
                        Effect::DealDamage {
                            amount: Amount::CountersOnSource(CounterKind::P1P1),
                            target: TargetSpec::Player(PlayerRel::You),
                        },
                    ]),
                },
            ],
        ),
    ],
);
