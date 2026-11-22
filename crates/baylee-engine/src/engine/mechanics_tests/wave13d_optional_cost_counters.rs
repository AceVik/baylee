//! Beat 18 / Engine Wave 13d deterministic rule tests:
//! 1. Non-Kicker Optional Additional Spell Costs (`FaceDef::optional_cost`,
//!    `CastModeKind::OptionalCost`, `Condition::OptionalCostPaid`, `Amount::IfCondition`,
//!    `CostPart::Behold`, `CostPart::CollectEvidence` — `CR 118.8b`, `CR 118.9d`,
//!    `CR 701.4a`, `CR 701.59`, `CR 707.10`).
//! 2. Choice of Counter Kind (`Effect::PutCounterChoice`, `Pending::ChooseCounterKind`,
//!    `PlayerAction::ChooseCounterKind`, CR 122.1b keyword counters, and `CR 614.16`
//!    counter-replacement effects + `Trigger::CountersPlaced`).

#![allow(clippy::too_many_lines)] // comprehensive multi-case rule tests

use super::*;
use crate::choice::{AnswerFault, ArrangePrompt, CastModeKind, ChoicePrompt};
use crate::event::{Cause, GameEvent};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{spell, triggered};
use baylee_cards_dsl::{
    AltCondition, AlternativeCost, Amount, Condition, CostPart, CounterKind, CounterReplaceOp,
    Filter, PlayerRel, ReplacementRule, TargetSpec, Trigger,
};
use baylee_core::generated::subtypes::creature;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_910;
const C1: u32 = 99_911;
const C2: u32 = 99_912;
const C3: u32 = 99_913;
const C4: u32 = 99_914;
const C5: u32 = 99_915;

static DRAGON_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[creature::DRAGON];

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    if let Pending::Priority { player: p, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player: p,
            legal: Box::new(engine.compute_legal(p)),
        };
    }
}

fn aim_object(engine: &mut Bench, seat: PlayerId, obj: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![obj],
                players: vec![],
            },
        )
        .expect("target accepted");
}

fn choose_cast_mode_kind(engine: &mut Bench, seat: PlayerId, kind: CastModeKind) {
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    let pos = options
        .iter()
        .position(|opt| opt.kind == kind)
        .unwrap_or_else(|| panic!("expected {kind:?} in {options:?}"));
    engine
        .apply(seat, PlayerAction::ChooseMode(pos))
        .expect("cast mode accepted");
}

fn counters_on(engine: &Bench, id: ObjectId, kind: CounterKind) -> u16 {
    engine.state().object(id).unwrap().counters.get(kind)
}

#[test]
fn optional_cost_unpaid_vs_paid_and_never_triggers_kicker_synergies() {
    static SURGE_EFFECTS: &[Effect] = &[
        Effect::DealDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::IfCondition {
                condition: &Condition::OptionalCostPaid,
                then: &Amount::Fixed(4),
                otherwise: &Amount::Fixed(2),
            },
        },
        Effect::IfKicked {
            then: &[Effect::GainLife {
                amount: Amount::Fixed(10),
            }],
            otherwise: &[],
        },
    ];
    static SURGE_ABILITIES: &[AbilityDef] = &[spell!(
        SURGE_EFFECTS,
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static SAC_ARTIFACT_PARTS: &[CostPart] = &[CostPart::Sacrifice(&Filter::And(&[
        Filter::ARTIFACT,
        Filter::ControlledByYou,
    ]))];
    static KICKER_WATCHER_AB: &[AbilityDef] = &[triggered!(
        Trigger::SpellCast(&Filter::Kicked),
        &[Effect::GainLife {
            amount: Amount::Fixed(7),
        }]
    )];

    let surge_face = FaceDef {
        optional_cost: Some(Cost {
            mana: ManaCost::ZERO,
            parts: SAC_ARTIFACT_PARTS,
        }),
        ..face("Spark Surge", "{R}", TypeSet::INSTANT)
    };
    let surge = card(C0, surge_face, KeywordSet::EMPTY, SURGE_ABILITIES);
    let relic = card(
        C1,
        face("Scrap Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let watcher = card(
        C2,
        face("Roil Watcher", "{1}{U}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        KICKER_WATCHER_AB,
    );
    let ox = card(
        C3,
        creature_face("Sturdy Ox", "{3}{G}", 2, 4),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18_130,
        vec![surge, relic, watcher, ox],
        [Seat::with(&[C1, C2]).holding(&[C0, C0]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());

    let watcher_id = the(&engine, ZoneLocation::Battlefield, C2);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C1);
    let ox_id = the(&engine, ZoneLocation::Battlefield, C3);
    let surges = objects(&engine, ZoneLocation::Hand(me()), C0);
    assert_eq!(surges.len(), 2);

    // 1. Cast unpaid (`CastModeKind::Normal`): deals 2 damage to 2/4 Ox, relic stays.
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: surges[0] })
        .unwrap();
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::Normal);
    aim_object(&mut engine, me(), ox_id);
    settle(&mut engine, me());

    assert_eq!(
        engine.state().object(ox_id).unwrap().damage,
        2,
        "unpaid optional cost deals the otherwise amount (2)"
    );
    assert!(
        objects(&engine, ZoneLocation::Battlefield, C1).contains(&relic_id),
        "artifact was not sacrificed when Normal mode was chosen"
    );
    assert_eq!(triggered(&engine, watcher_id), 0);
    assert_eq!(life(&engine, me()), 20);

    // Reset Ox damage to 0 so the paid cast alone must deal all 4 lethal damage.
    engine.state.object_mut(ox_id).unwrap().damage = 0;

    // 2. Cast paid (`CastModeKind::OptionalCost`): sacrifices relic, deals 4 damage,
    //    destroys 2/4 Ox, and NEVER fires Kicker triggers or `Effect::IfKicked`!
    add_pool(&mut engine, me(), ManaColor::Red, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: surges[1] })
        .unwrap();
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);
    aim_object(&mut engine, me(), ox_id);
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostSacrifice,
                ..
            }
        ),
        "paying optional cost prompts for artifact sacrifice: {:?}",
        engine.pending()
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert!(
        objects(&engine, ZoneLocation::Battlefield, C1).is_empty(),
        "relic was sacrificed to pay optional additional cost"
    );
    assert!(
        objects(&engine, ZoneLocation::Battlefield, C3).is_empty(),
        "4 damage from paid optional cost destroyed the 2/4 Ox"
    );
    assert_eq!(
        triggered(&engine, watcher_id),
        0,
        "CR 118.8b: non-kicker optional additional cost must NEVER trigger Kicker abilities"
    );
    assert_eq!(
        life(&engine, me()),
        20,
        "Effect::IfKicked must remain false when non-kicker optional_cost was paid"
    );
}

#[test]
fn optional_cost_composes_with_alternative_cost_cr_118_9d() {
    static ALT_SURGE_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            target: TargetSpec::AnyTarget,
            amount: Amount::IfCondition {
                condition: &Condition::OptionalCostPaid,
                then: &Amount::Fixed(5),
                otherwise: &Amount::Fixed(2),
            },
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static PAY_LIFE_PARTS: &[CostPart] = &[CostPart::PayLife(2)];
    static ALT_COSTS: &[AlternativeCost] = &[AlternativeCost {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: PAY_LIFE_PARTS,
        },
        condition: AltCondition::Always,
    }];
    static SAC_ARTIFACT_PARTS: &[CostPart] = &[CostPart::Sacrifice(&Filter::And(&[
        Filter::ARTIFACT,
        Filter::ControlledByYou,
    ]))];

    let alt_surge = card(
        C0,
        FaceDef {
            alternative_costs: ALT_COSTS,
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: SAC_ARTIFACT_PARTS,
            }),
            ..face("Rift Bolt Surge", "{2}{R}", TypeSet::SORCERY)
        },
        KeywordSet::EMPTY,
        ALT_SURGE_AB,
    );
    let relic = card(
        C1,
        face("Scrap Relic", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18_131,
        vec![alt_surge, relic],
        [Seat::with(&[C1]).holding(&[C0]), Seat::default()],
    );
    to_main(&mut engine, me());

    let spell_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let relic_id = the(&engine, ZoneLocation::Battlefield, C1);

    // Put {R}{R}{R} in pool so both Normal ({2}{R}) and Alternative(0) (Pay 2 life) are offered,
    // pick `Alternative(0)`, and verify CR 118.9d offers `OptionalCost` on top of `Alternative(0)`!
    add_pool(&mut engine, me(), ManaColor::Red, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: spell_id })
        .unwrap();
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::Alternative(0));
    // CR 118.9d: optional additional cost is offered on top of the chosen alternative cost!
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![relic_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(life(&engine, me()), 18, "paid 2 life for alternative cost");
    assert_eq!(
        engine.state().players[usize::from(me().get())]
            .mana_pool
            .total(),
        3,
        "alternative cost + zero-mana optional cost spent none of the floating red mana"
    );
    assert_eq!(
        life(&engine, them()),
        15,
        "CR 118.9d: optional additional cost paid on top of alternative cost dealt 5 damage"
    );
    assert!(objects(&engine, ZoneLocation::Battlefield, C1).is_empty());
}

#[test]
fn behold_or_pay_names_the_surcharge_and_only_reveals_the_card_beheld() {
    static ABILITIES: &[AbilityDef] = &[spell!(&[Effect::GainLife {
        amount: Amount::Fixed(1),
    }])];
    let cost = baylee_core::mana!("{1}");
    let spell = card(
        C0,
        FaceDef {
            behold_or_pay: Some(baylee_cards_dsl::BeholdOrPay {
                filter: &Filter::HasSubtype(creature::DRAGON),
                cost,
            }),
            ..face("Dragon's Price", "{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        ABILITIES,
    );
    let dragon = card(
        C1,
        FaceDef {
            subtypes: DRAGON_SUBTYPES,
            ..creature_face("Stormscale Dragon", "{4}{R}", 4, 4)
        },
        KeywordSet::FLYING,
        &[],
    );

    for (mana, hand, pay) in [(1, true, false), (2, false, false), (2, true, true)] {
        let caster = if hand {
            Seat::default().holding(&[C0, C1])
        } else {
            Seat::with(&[C1]).holding(&[C0])
        };
        let mut engine = bench(
            18_139,
            vec![spell, dragon],
            [caster, Seat::default().holding(&[C1])],
        );
        to_main(&mut engine, me());
        let card = the(&engine, ZoneLocation::Hand(me()), C0);
        let beheld = the(
            &engine,
            if hand {
                ZoneLocation::Hand(me())
            } else {
                ZoneLocation::Battlefield
            },
            C1,
        );
        add_pool(&mut engine, me(), ManaColor::Green, mana);
        engine
            .apply(me(), PlayerAction::CastSpell { card })
            .unwrap();
        let Pending::ChooseCards {
            options,
            min,
            max: 1,
            prompt: ChoicePrompt::BeholdOrPay { mana: surcharge },
            ..
        } = engine.pending()
        else {
            panic!(
                "expected a behold-or-pay choice, got {:?}",
                engine.pending()
            );
        };
        assert_eq!(*surcharge, cost);
        assert_eq!(*min, u8::from(mana == 1));
        assert_eq!(options, &[beheld], "only the caster can offer a Dragon");
        let since = engine.state().journal.last_seq();
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: if pay { vec![] } else { vec![beheld] },
                },
            )
            .unwrap();
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            u64::from(mana - 1 - u32::from(pay))
        );
        let revealed = engine.state().journal.entries().iter().any(|entry| {
            entry.seq > since && matches!(&entry.event, GameEvent::Revealed { cards, .. } if cards.contains(&beheld))
        });
        assert_eq!(revealed, hand && !pay);
    }
}

#[test]
fn optional_behold_from_battlefield_and_hand_reveals_only_hand_card_cr_701_4a() {
    static EXHALE_AB: &[AbilityDef] = &[spell!(
        &[
            Effect::DealDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::Fixed(3),
            },
            Effect::IfCondition {
                condition: Condition::OptionalCostPaid,
                then: &[Effect::Surveil {
                    amount: Amount::Fixed(2),
                }],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static BEHOLD_DRAGON_PARTS: &[CostPart] =
        &[CostPart::Behold(&Filter::HasSubtype(creature::DRAGON))];

    let exhale = card(
        C0,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: BEHOLD_DRAGON_PARTS,
            }),
            ..face("Dragon Exhale", "{1}{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        EXHALE_AB,
    );
    let dragon = card(
        C1,
        FaceDef {
            subtypes: DRAGON_SUBTYPES,
            ..creature_face("Stormscale Dragon", "{4}{R}", 4, 4)
        },
        KeywordSet::FLYING,
        &[],
    );
    let target_bear = card(
        C2,
        creature_face("Target Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // Case A: No Dragon on battlefield or in hand -> `OptionalCost` is NOT offered; casts straight to targets.
    {
        let mut engine = bench(
            18_132,
            vec![exhale, dragon, target_bear],
            [Seat::default().holding(&[C0]), Seat::with(&[C2])],
        );
        to_main(&mut engine, me());
        let exhale_id = the(&engine, ZoneLocation::Hand(me()), C0);
        let bear_id = the(&engine, ZoneLocation::Battlefield, C2);
        add_pool(&mut engine, me(), ManaColor::Green, 2);
        engine
            .apply(me(), PlayerAction::CastSpell { card: exhale_id })
            .unwrap();
        assert!(
            matches!(engine.pending(), Pending::ChooseTargets { .. }),
            "without a Dragon to behold, OptionalCost is not offered: {:?}",
            engine.pending()
        );
        aim_object(&mut engine, me(), bear_id);
        settle(&mut engine, me());
        assert!(objects(&engine, ZoneLocation::Battlefield, C2).is_empty());
    }

    // Case B: Dragon on battlefield -> beholding it does NOT emit `Revealed`, runs Surveil 2.
    {
        let mut engine = bench(
            18_133,
            vec![exhale, dragon, target_bear],
            [Seat::with(&[C1]).holding(&[C0]), Seat::with(&[C2])],
        );
        to_main(&mut engine, me());
        let exhale_id = the(&engine, ZoneLocation::Hand(me()), C0);
        let dragon_id = the(&engine, ZoneLocation::Battlefield, C1);
        let bear_id = the(&engine, ZoneLocation::Battlefield, C2);
        add_pool(&mut engine, me(), ManaColor::Green, 2);
        engine
            .apply(me(), PlayerAction::CastSpell { card: exhale_id })
            .unwrap();
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);
        aim_object(&mut engine, me(), bear_id);
        assert!(matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Behold,
                ..
            }
        ));
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![dragon_id],
                },
            )
            .unwrap();
        walk_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::Arrange {
                    prompt: ArrangePrompt::Surveil,
                    ..
                }
            )
        });
        let Pending::Arrange { cards, .. } = engine.pending().clone() else {
            unreachable!();
        };
        assert_eq!(
            cards.len(),
            2,
            "Surveil 2 ran because OptionalCostPaid held"
        );
        engine
            .apply(
                me(),
                PlayerAction::Arrange {
                    piles: vec![vec![], cards],
                },
            )
            .unwrap();
        settle(&mut engine, me());
        let revealed_any = engine
            .journal()
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::Revealed { .. }));
        assert!(
            !revealed_any,
            "CR 701.4a: choosing a Dragon on the battlefield does not reveal from hand"
        );
        assert!(objects(&engine, ZoneLocation::Battlefield, C1).contains(&dragon_id));
    }

    // Case C: Dragon in hand -> beholding it emits `GameEvent::Revealed` and keeps Dragon in hand.
    {
        let mut engine = bench(
            18_134,
            vec![exhale, dragon, target_bear],
            [Seat::default().holding(&[C0, C1]), Seat::with(&[C2])],
        );
        to_main(&mut engine, me());
        let exhale_id = the(&engine, ZoneLocation::Hand(me()), C0);
        let dragon_id = the(&engine, ZoneLocation::Hand(me()), C1);
        let bear_id = the(&engine, ZoneLocation::Battlefield, C2);
        add_pool(&mut engine, me(), ManaColor::Green, 2);
        engine
            .apply(me(), PlayerAction::CastSpell { card: exhale_id })
            .unwrap();
        choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);
        aim_object(&mut engine, me(), bear_id);
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![dragon_id],
                },
            )
            .unwrap();
        walk_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::Arrange {
                    prompt: ArrangePrompt::Surveil,
                    ..
                }
            )
        });
        let Pending::Arrange { cards, .. } = engine.pending().clone() else {
            unreachable!();
        };
        engine
            .apply(
                me(),
                PlayerAction::Arrange {
                    piles: vec![cards, vec![]],
                },
            )
            .unwrap();
        settle(&mut engine, me());
        let revealed_dragon = engine.journal().entries().iter().any(|e| {
            matches!(
                &e.event,
                GameEvent::Revealed { cards, .. } if cards.contains(&dragon_id)
            )
        });
        assert!(
            revealed_dragon,
            "CR 701.4a: beholding a Dragon card from hand reveals that card"
        );
        assert!(
            objects(&engine, ZoneLocation::Hand(me()), C1).contains(&dragon_id),
            "beheld card stays in hand"
        );
    }
}

#[test]
fn optional_collect_evidence_exiles_graveyard_cards_and_fires_etb_trigger_cr_701_59() {
    static INSPECTOR_AB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[
            Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::GainLife {
                amount: Amount::Fixed(2),
            },
        ],
        condition = Some(Condition::OptionalCostPaid)
    )];
    static COLLECT_6_PARTS: &[CostPart] = &[CostPart::CollectEvidence(6)];

    let inspector = card(
        C0,
        FaceDef {
            optional_cost: Some(Cost {
                mana: ManaCost::ZERO,
                parts: COLLECT_6_PARTS,
            }),
            ..creature_face("Guild Inspector", "{1}{G}", 1, 3)
        },
        KeywordSet::REACH,
        INSPECTOR_AB,
    );
    let clue_four = card(
        C1,
        face("Four-Cost Tome", "{4}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let clue_two = card(
        C2,
        face("Two-Cost Dagger", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        18_135,
        vec![inspector, clue_four, clue_two],
        [Seat::with(&[C1, C2]).holding(&[C0, C0]), Seat::default()],
    );
    to_main(&mut engine, me());

    // Move only the MV-4 card into graveyard first (total MV = 4 < 6).
    let tome_bf = the(&engine, ZoneLocation::Battlefield, C1);
    let dagger_bf = the(&engine, ZoneLocation::Battlefield, C2);
    let tome_gy = engine
        .state
        .move_object(
            tome_bf,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let inspectors = objects(&engine, ZoneLocation::Hand(me()), C0);

    // 1. With only MV 4 in graveyard, `OptionalCost` is NOT offered; ETB trigger does not fire.
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: inspectors[0],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let first_inspector = the(&engine, ZoneLocation::Battlefield, C0);
    assert_eq!(counters_on(&engine, first_inspector, CounterKind::P1P1), 0);
    assert_eq!(life(&engine, me()), 20);

    // 2. Put the MV-2 card in graveyard as well (total MV = 4 + 2 = 6).
    let dagger_gy = engine
        .state
        .move_object(
            dagger_bf,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: inspectors[1],
            },
        )
        .unwrap();
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);

    let Pending::ChooseCards {
        prompt: ChoicePrompt::CollectEvidence { mana_value: 6 },
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected CollectEvidence(6) prompt, got {:?}",
            engine.pending()
        );
    };

    // Selecting only MV 4 is rejected with `AnswerFault::TotalTooLow`!
    assert_eq!(
        engine.pending().answer_fault(&PlayerAction::ChooseObjects {
            objects: vec![tome_gy],
        }),
        Some(AnswerFault::TotalTooLow),
        "CR 701.59: total mana value 4 < 6 must be rejected"
    );

    // Selecting both MV 4 + MV 2 = 6 succeeds, exiles both, and fires the ETB trigger!
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![tome_gy, dagger_gy],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert!(objects(&engine, ZoneLocation::Graveyard(me()), C1).is_empty());
    assert!(objects(&engine, ZoneLocation::Graveyard(me()), C2).is_empty());
    assert_eq!(objects(&engine, ZoneLocation::Exile(me()), C1).len(), 1);
    assert_eq!(objects(&engine, ZoneLocation::Exile(me()), C2).len(), 1);

    let on_bf = objects(&engine, ZoneLocation::Battlefield, C0);
    assert_eq!(on_bf.len(), 2);
    let second_inspector = on_bf.into_iter().find(|&id| id != first_inspector).unwrap();
    assert_eq!(
        counters_on(&engine, second_inspector, CounterKind::P1P1),
        1,
        "permanent entered from spell with OptionalCostPaid and fired its intervening-if ETB trigger"
    );
    assert_eq!(life(&engine, me()), 22);
}

#[test]
fn spell_copy_preserves_optional_cost_paid_cr_707_10() {
    static SURGE_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            target: TargetSpec::AnyTarget,
            amount: Amount::IfCondition {
                condition: &Condition::OptionalCostPaid,
                then: &Amount::Fixed(4),
                otherwise: &Amount::Fixed(1),
            },
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static FORK_AB: &[AbilityDef] = &[free(
        &[Effect::CopyTargetSpell { mods: &[] }],
        Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )];
    static EXTRA_MANA_OPT: Cost = Cost {
        mana: ManaCost::parse("{1}"),
        parts: &[],
    };

    let surge = card(
        C0,
        FaceDef {
            optional_cost: Some(EXTRA_MANA_OPT),
            ..face("Echoing Surge", "{R}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        SURGE_AB,
    );
    let fork_ring = card(
        C1,
        face("Echo Fork", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        FORK_AB,
    );

    let mut engine = bench(
        18_136,
        vec![surge, fork_ring],
        [Seat::with(&[C1]).holding(&[C0]), Seat::default()],
    );
    to_main(&mut engine, me());

    let surge_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let ring_id = the(&engine, ZoneLocation::Battlefield, C1);

    add_pool(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: surge_id })
        .unwrap();
    choose_cast_mode_kind(&mut engine, me(), CastModeKind::OptionalCost);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();

    // Hold priority / activate Echo Fork targeting the spell on the stack!
    let stack_spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("spell on stack");
    activate(&mut engine, me(), ring_id, 0);
    aim_object(&mut engine, me(), stack_spell);

    // Copy resolves and asks whether to change its target; keep the original target (empty pick).
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets {
                reason: crate::choice::TargetPrompt::Retarget { .. },
                ..
            }
        )
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        life(&engine, them()),
        12,
        "CR 707.10: both original spell and its copy saw OptionalCostPaid == true (4 + 4 = 8 damage)"
    );
}

#[test]
fn put_counter_choice_keyword_and_p1p1_with_replacements_and_triggers_cr_122_1_cr_614_16() {
    static KEYWORD_GRANTER_CHOICES: &[CounterKind] = &[
        CounterKind::Flying,
        CounterKind::FirstStrike,
        CounterKind::Lifelink,
        CounterKind::Vigilance,
    ];
    static MIXED_CHOICES: &[CounterKind] = &[CounterKind::P1P1, CounterKind::Trample];

    static KEYWORD_GRANTER_AB: &[AbilityDef] = &[free(
        &[Effect::PutCounterChoice {
            target: TargetSpec::Object(&Filter::CREATURE),
            choices: KEYWORD_GRANTER_CHOICES,
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static MIXED_GRANTER_AB: &[AbilityDef] = &[free(
        &[Effect::PutCounterChoice {
            target: TargetSpec::Object(&Filter::CREATURE),
            choices: MIXED_CHOICES,
            amount: Amount::Fixed(1),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )];
    static SCALES_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
            kind: Some(CounterKind::P1P1),
            placer: None,
            effect_only: false,
            op: CounterReplaceOp::Add(1),
        },
    )];
    static DOUBLER_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::ControlledByYou,
        },
    )];
    static COUNTER_TRIGGER_AB: &[AbilityDef] = &[triggered!(
        Trigger::CountersPlaced {
            filter: &Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]),
            kind: CounterKind::FirstStrike,
            by: PlayerRel::You,
            first_this_turn: false,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]
    )];
    static SOLEMNITY_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ModifyCounterPlacement {
            object_filter: &Filter::CREATURE,
            kind: None,
            placer: None,
            effect_only: false,
            op: CounterReplaceOp::Multiply(0),
        },
    )];

    let keyword_shrine = card(
        C0,
        face("Resplendent Shrine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        KEYWORD_GRANTER_AB,
    );
    let mixed_shrine = card(
        C1,
        face("Boseiju Shrine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MIXED_GRANTER_AB,
    );
    let scales = card(
        C2,
        face("Scales", "{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        SCALES_AB,
    );
    let doubler = card(
        C3,
        face("Season", "{4}{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        DOUBLER_AB,
    );
    let tracker = card(
        C4,
        face("Counter Tracker", "{W}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        COUNTER_TRIGGER_AB,
    );
    let bear = card(
        C5,
        creature_face("Squire", "{1}{W}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    // Part 1: Keyword counter choice grants layer-6 keyword + Hardened Scales + Doubling Season + CounterPlaced trigger!
    {
        let mut engine = bench(
            18_137,
            vec![keyword_shrine, mixed_shrine, scales, doubler, tracker, bear],
            [Seat::with(&[C0, C1, C2, C4, C5]), Seat::default()],
        );
        to_main(&mut engine, me());

        let kw_shrine_id = the(&engine, ZoneLocation::Battlefield, C0);
        let mix_shrine_id = the(&engine, ZoneLocation::Battlefield, C1);
        let tracker_id = the(&engine, ZoneLocation::Battlefield, C4);
        let bear_id = the(&engine, ZoneLocation::Battlefield, C5);

        // Activate keyword shrine -> suspends at `Pending::ChooseCounterKind`.
        activate(&mut engine, me(), kw_shrine_id, 0);
        aim_object(&mut engine, me(), bear_id);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCounterKind { .. })
        });

        // Choosing an unoffered counter kind (`Trample`) is rejected!
        assert!(
            engine
                .apply(me(), PlayerAction::ChooseCounterKind(CounterKind::Trample))
                .is_err(),
            "engine must reject a CounterKind not in `choices`"
        );

        // Choosing `FirstStrike` places 1 FirstStrike counter (Hardened Scales only affects +1/+1!)
        // and grants `KeywordSet::FIRST_STRIKE` in layer 6 (CR 122.1b)!
        engine
            .apply(
                me(),
                PlayerAction::ChooseCounterKind(CounterKind::FirstStrike),
            )
            .unwrap();
        settle(&mut engine, me());

        assert_eq!(counters_on(&engine, bear_id, CounterKind::FirstStrike), 1);
        assert!(
            engine
                .state()
                .object(bear_id)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::FIRST_STRIKE),
            "CR 122.1b: FirstStrike counter grants First Strike in layer 6"
        );
        assert_eq!(triggered(&engine, tracker_id), 1);
        assert_eq!(life(&engine, me()), 21);

        // Activate mixed shrine and choose `P1P1` -> Hardened Scales adds +1 so Bear gets 2 +1/+1 counters!
        activate(&mut engine, me(), mix_shrine_id, 0);
        aim_object(&mut engine, me(), bear_id);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCounterKind { .. })
        });
        engine
            .apply(me(), PlayerAction::ChooseCounterKind(CounterKind::P1P1))
            .unwrap();
        settle(&mut engine, me());

        assert_eq!(
            counters_on(&engine, bear_id, CounterKind::P1P1),
            2,
            "CR 614.16: Hardened Scales increased chosen +1/+1 counter from 1 to 2"
        );

        // Now add Doubling Season and choose `Vigilance` -> places 2 Vigilance counters!
        let mut engine2 = bench(
            18_138,
            vec![keyword_shrine, doubler, bear],
            [Seat::with(&[C0, C3, C5]), Seat::default()],
        );
        to_main(&mut engine2, me());
        let kw2 = the(&engine2, ZoneLocation::Battlefield, C0);
        let bear2 = the(&engine2, ZoneLocation::Battlefield, C5);
        activate(&mut engine2, me(), kw2, 0);
        aim_object(&mut engine2, me(), bear2);
        walk_until(&mut engine2, |e| {
            matches!(e.pending(), Pending::ChooseCounterKind { .. })
        });
        engine2
            .apply(
                me(),
                PlayerAction::ChooseCounterKind(CounterKind::Vigilance),
            )
            .unwrap();
        settle(&mut engine2, me());
        assert_eq!(
            counters_on(&engine2, bear2, CounterKind::Vigilance),
            2,
            "CR 614.16: Doubling Season doubled chosen Vigilance counter to 2"
        );
        assert!(
            engine2
                .state()
                .object(bear2)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::VIGILANCE)
        );
    }

    // Part 2: Zero-multiplier replacement (`Solemnity` / `Blightbeetle`) prevents counter placement cleanly!
    {
        let solemnity = card(
            C2,
            face("Solemnity", "{2}{W}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            SOLEMNITY_AB,
        );
        let mut engine = bench(
            18_139,
            vec![keyword_shrine, solemnity, tracker, bear],
            [Seat::with(&[C0, C2, C4, C5]), Seat::default()],
        );
        to_main(&mut engine, me());
        let kw_id = the(&engine, ZoneLocation::Battlefield, C0);
        let tracker_id = the(&engine, ZoneLocation::Battlefield, C4);
        let bear_id = the(&engine, ZoneLocation::Battlefield, C5);

        activate(&mut engine, me(), kw_id, 0);
        aim_object(&mut engine, me(), bear_id);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCounterKind { .. })
        });
        engine
            .apply(
                me(),
                PlayerAction::ChooseCounterKind(CounterKind::FirstStrike),
            )
            .unwrap();
        settle(&mut engine, me());

        assert_eq!(
            counters_on(&engine, bear_id, CounterKind::FirstStrike),
            0,
            "zero-multiplier counter replacement prevented counter placement"
        );
        assert_eq!(triggered(&engine, tracker_id), 0);
    }
}
