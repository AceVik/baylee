//! Consuming Ferocity — {1}{R} — Enchantment — Aura
//! Oracle: Enchant non-Wall creature
//! Oracle: Enchanted creature gets +1/+0.
//! Oracle: At the beginning of your upkeep, put a +1/+0 counter on enchanted creature. If that creature has three or more +1/+0 counters on it, it deals damage equal to its power to its controller, then destroy that creature and it can't be regenerated.
//! Set: MIR #166 — Mirage | Scryfall ID: 05835e35-f4f8-4513-b5a1-9e31b21168f0 | Oracle ID: 8efbe68c-622c-4986-a375-7e0bf780eb2e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);
const ENCHANTED_NON_WALL: Filter = Filter::And(&[NON_WALL_CREATURE, Filter::AttachedToBySource]);
const THREE_OR_MORE_P1P0: Filter = Filter::CountersAtLeast(
    CounterKind::Plus {
        power: 1,
        toughness: 0,
    },
    3,
);

card!(
    index = index::CONSUMING_FEROCITY,
    oracle_id = "8efbe68c-622c-4986-a375-7e0bf780eb2e",
    scryfall_id = "05835e35-f4f8-4513-b5a1-9e31b21168f0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Consuming Ferocity",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&NON_WALL_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&NON_WALL_CREATURE))),
        ),
        static_ability!(ENCHANTED_NON_WALL, Modifier::ModifyPT(1, 0)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::ChangeAttachedCounters {
                    kind: CounterKind::Plus {
                        power: 1,
                        toughness: 0,
                    },
                    amount: Amount::Fixed(1),
                    remove: false,
                },
                Effect::IfCondition {
                    condition: Condition::AttachedMatches(&THREE_OR_MORE_P1P0),
                    then: &[
                        // "It deals damage equal to its power to its
                        // controller": the enchanted creature is the source of
                        // the damage and its controller is the one dealt to.
                        Effect::ForEachMatching {
                            filter: &Filter::AttachedToBySource,
                            effects: &[Effect::EventObjectDealsDamageEqualToPower {
                                target: TargetSpec::Player(PlayerRel::EventPlayer),
                            }],
                        },
                        Effect::DestroyAll {
                            filter: &Filter::AttachedToBySource,
                            no_regen: true,
                        },
                    ],
                    otherwise: &[],
                },
            ],
        ),
    ],
);
