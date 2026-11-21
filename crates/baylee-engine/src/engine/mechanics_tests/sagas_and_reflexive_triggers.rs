//! Rule tests for generalized reflexive triggers (`CR 603.12`, `CR 603.12a`),
//! generalized delayed triggers (`CR 603.7`, `CR 603.4`), multi-chapter Sagas
//! (`CR 715.2c`), and Read Ahead (`CR 702.155`, `CR 715.3b`, `CR 715.4`).
#![allow(clippy::too_many_lines)] // End-to-end game scenarios keep positive and negative cases together.

use super::*;
use crate::choice::NumberPrompt;
use baylee_cards_dsl::{
    Amount, Condition, CounterKind, DelayedDuration, DelayedStep, DelayedTriggerWhen, Filter,
    PlayerRel, ReflexiveEvent, ReflexiveWhen, ReplacementRule, TargetSpec, ThisWay, Trigger,
    TriggerRemember, chapter, chapters,
};
use baylee_core::generated::subtypes::enchantment::SAGA;

const PROBE: u32 = 99_850;
const FODDER_5_2: u32 = 99_851;
const ARTIFACT_RELIC: u32 = 99_852;
const STIFLE_ABILITY: u32 = 99_853;
const DESTROY_ARTIFACT: u32 = 99_854;
const BOUNCE_CREATURE: u32 = 99_855;
const KILL_CREATURE: u32 = 99_856;
const PING_INSTANT: u32 = 99_857;
const READ_AHEAD_SAGA: u32 = 99_858;
const MULTI_CHAPTER_SAGA: u32 = 99_859;
const READ_AHEAD_MULTI_SAGA: u32 = 99_860;
const LORE_BOOSTER: u32 = 99_861;
const DOUBLER: u32 = 99_862;

static SAGA_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[SAGA];

fn saga_face(name: &'static str, cost: &'static str) -> FaceDef {
    FaceDef {
        subtypes: SAGA_SUBTYPES,
        ..face(name, cost, TypeSet::ENCHANTMENT)
    }
}

fn cast_from_hand(engine: &mut Bench, seat: PlayerId, card_idx: u32) -> ObjectId {
    let id = the(engine, ZoneLocation::Hand(seat), card_idx);
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {card_idx} ({id:?}) to be castable, got {:?}",
        legal.castable
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast starts");
    id
}

#[test]
fn reflexive_trigger_sacrificed_lki_power_and_priority_window() {
    static SAC_REFLEXIVE: &[AbilityDef] = &[free(
        &[
            Effect::MayDo {
                effects: &[Effect::SacrificeFilter {
                    who: PlayerRel::You,
                    filter: &Filter::CREATURE,
                }],
            },
            Effect::ReflexiveTrigger {
                when: ReflexiveWhen::Event(ReflexiveEvent::Sacrificed(&Filter::CREATURE)),
                once: true,
                condition: None,
                targets: Some(TargetReq::one(TargetSpec::AnyTarget)),
                remember: TriggerRemember::SacrificedObject,
                effects: &[Effect::DealDamage {
                    amount: Amount::EventPower,
                    target: TargetSpec::AnyTarget,
                }],
            },
        ],
        None,
    )];
    static COUNTER_ABILITY: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::CounterTargetAbility],
        targets: Some(TargetReq::one(TargetSpec::AbilityOnStack(&Filter::Any))),
        second_targets: None,
        condition: None,
    }];

    let cards = vec![
        card(
            PROBE,
            face("Altar Probe", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SAC_REFLEXIVE,
        ),
        card(
            FODDER_5_2,
            creature_face("Five Power", "{0}", 5, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            STIFLE_ABILITY,
            face("Hush", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            COUNTER_ABILITY,
        ),
    ];

    // 1. Declining MayDo creates no reflexive trigger.
    let mut engine = bench(
        99_850,
        cards.clone(),
        [Seat::with(&[PROBE, FODDER_5_2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let probe = the(&engine, ZoneLocation::Battlefield, PROBE);
    activate(&mut engine, me(), probe, 0);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass");
    assert!(matches!(engine.pending(), Pending::YesNo { player, .. } if *player == me()));
    engine
        .apply(me(), PlayerAction::YesNo(false))
        .expect("decline sacrifice");
    assert!(engine.state().zones.stack_is_empty());
    assert_eq!(life(&engine, them()), 20);

    // 2. Accepting sacrifices the 5/2, queues reflexive trigger for target choice,
    // puts an independent ability on the stack, and deals 5 LKI damage when uncountered.
    activate(&mut engine, me(), probe, 0);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("pass");
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept sacrifice");
    let fodder = the(&engine, ZoneLocation::Battlefield, FODDER_5_2);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .expect("choose fodder");
    }
    assert!(matches!(
        engine.pending(),
        Pending::ChooseTargets { player, .. } if *player == me()
    ));
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .expect("aim reflexive damage at opponent");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "reflexive trigger uses the stack as its own object (CR 603.12)"
    );
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, them()),
        15,
        "deals sacrificed creature's 5 LKI power"
    );

    // 3. Opponent can counter the reflexive trigger on the stack in its priority window.
    let mut countered = bench(
        99_851,
        cards,
        [
            Seat::with(&[PROBE, FODDER_5_2]),
            Seat::default().holding(&[STIFLE_ABILITY]),
        ],
    );
    to_main(&mut countered, me());
    let probe = the(&countered, ZoneLocation::Battlefield, PROBE);
    let fodder = the(&countered, ZoneLocation::Battlefield, FODDER_5_2);
    activate(&mut countered, me(), probe, 0);
    countered.apply(me(), PlayerAction::PassPriority).unwrap();
    countered.apply(them(), PlayerAction::PassPriority).unwrap();
    countered.apply(me(), PlayerAction::YesNo(true)).unwrap();
    if matches!(countered.pending(), Pending::ChooseCards { .. }) {
        countered
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .unwrap();
    }
    countered
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    let reflex_on_stack = countered.state().zones.list(ZoneLocation::Stack)[0];
    countered.apply(me(), PlayerAction::PassPriority).unwrap();
    let _ = cast_from_hand(&mut countered, them(), STIFLE_ABILITY);
    if matches!(countered.pending(), Pending::ChooseTargets { .. }) {
        countered
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![reflex_on_stack],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut countered, me());
    assert_eq!(
        life(&countered, them()),
        20,
        "countered reflexive trigger deals 0"
    );
}

#[test]
fn reflexive_trigger_this_way_once_vs_per_occurrence_and_intervening_if() {
    static MILL_ONCE_AND_EACH: &[AbilityDef] = &[
        free(
            &[
                Effect::Mill {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::You,
                },
                Effect::ReflexiveTrigger {
                    when: ReflexiveWhen::ThisWay(ThisWay::Milled(&Filter::LAND)),
                    once: true,
                    condition: None,
                    targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
                        &Filter::LAND,
                        PlayerRel::You,
                    ))),
                    remember: TriggerRemember::None,
                    effects: &[Effect::GraveyardToHand {
                        target: TargetSpec::CardInGraveyard(&Filter::LAND, PlayerRel::You),
                    }],
                },
            ],
            None,
        ),
        free(
            &[
                Effect::Mill {
                    amount: Amount::Fixed(2),
                    target: PlayerRel::You,
                },
                Effect::ReflexiveTrigger {
                    when: ReflexiveWhen::ThisWay(ThisWay::Milled(&Filter::LAND)),
                    once: false,
                    condition: None,
                    targets: None,
                    remember: TriggerRemember::None,
                    effects: &[Effect::gain_life(3)],
                },
            ],
            None,
        ),
        free(
            &[Effect::ReflexiveTrigger {
                when: ReflexiveWhen::Always,
                once: true,
                condition: Some(&Condition::ControlCount(&Filter::CREATURE, 1)),
                targets: None,
                remember: TriggerRemember::None,
                effects: &[Effect::gain_life(7)],
            }],
            None,
        ),
    ];
    static KILL: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];

    let cards = vec![
        card(
            PROBE,
            face("Mill Probe", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            MILL_ONCE_AND_EACH,
        ),
        card(
            FODDER_5_2,
            creature_face("Watcher", "{0}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            KILL_CREATURE,
            face("Slay", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            KILL,
        ),
    ];

    // A) `once: true` fires 1 trigger targeting one of the newly milled Forests;
    // `once: false` (CR 603.12a) fires 2 separate triggers (+6 life total).
    let mut engine = bench(
        99_852,
        cards.clone(),
        [Seat::with(&[PROBE]), Seat::default()],
    );
    to_main(&mut engine, me());
    let probe = the(&engine, ZoneLocation::Battlefield, PROBE);
    activate(&mut engine, me(), probe, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected graveyard target choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        options.len(),
        2,
        "both milled Forests are already in graveyard when targeted"
    );
    let picked_forest = options[0];
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![picked_forest],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&picked_forest)
    );

    activate(&mut engine, me(), probe, 1);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        26,
        "once: false triggers once per milled land (2 * 3 = 6)"
    );

    // B) Intervening-if (CR 603.4) with no creature: never triggers at all.
    let before_events = engine.journal().len();
    activate(&mut engine, me(), probe, 2);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 26);
    let synthetic_fires = engine.journal().entries()[before_events..]
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered {
                    ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                    ..
                }
            )
        })
        .count();
    assert_eq!(synthetic_fires, 0, "CR 603.4 first check prevents queuing");

    // C) Intervening-if holds when stacked, then creature is destroyed in response ->
    // CR 603.4 second check aborts resolution!
    let mut gated = bench(
        99_853,
        cards,
        [
            Seat::with(&[PROBE, FODDER_5_2]),
            Seat::default().holding(&[KILL_CREATURE]),
        ],
    );
    to_main(&mut gated, me());
    let probe = the(&gated, ZoneLocation::Battlefield, PROBE);
    let creature = the(&gated, ZoneLocation::Battlefield, FODDER_5_2);
    activate(&mut gated, me(), probe, 2);
    gated.apply(me(), PlayerAction::PassPriority).unwrap();
    gated.apply(them(), PlayerAction::PassPriority).unwrap();
    assert_eq!(
        gated.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "reflexive trigger stacked while creature existed"
    );
    gated.apply(me(), PlayerAction::PassPriority).unwrap();
    let _ = cast_from_hand(&mut gated, them(), KILL_CREATURE);
    if matches!(gated.pending(), Pending::ChooseTargets { .. }) {
        gated
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![creature],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut gated, me());
    assert_eq!(
        life(&gated, me()),
        20,
        "CR 603.4 second check removes synthetic reflexive trigger when condition becomes false"
    );
}

#[test]
fn reflexive_graveyard_exile_remembers_only_the_card_moved_from_that_zone() {
    const GRAVEYARD_TARGET: TargetSpec = TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You);
    static EXILE: &[AbilityDef] = &[free(
        &[
            Effect::ExileSource,
            Effect::exile(GRAVEYARD_TARGET),
            Effect::ReflexiveTrigger {
                when: ReflexiveWhen::ThisWay(ThisWay::ExiledFromGraveyard(&Filter::CREATURE)),
                once: true,
                condition: None,
                targets: None,
                remember: TriggerRemember::None,
                effects: &[Effect::GainLife {
                    amount: Amount::EventManaValue,
                }],
            },
        ],
        Some(TargetReq::one(GRAVEYARD_TARGET)),
    )];
    let cards = vec![
        card(
            PROBE,
            creature_face("Sigil", "{5}", 2, 2),
            KeywordSet::EMPTY,
            EXILE,
        ),
        card(
            FODDER_5_2,
            creature_face("Lost Mite", "{3}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ARTIFACT_RELIC,
            face("Grave Relic", "{4}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    for (fodder, expected_life) in [(FODDER_5_2, 23), (ARTIFACT_RELIC, 20)] {
        let mut engine = bench(
            99_863,
            cards.clone(),
            [Seat::with(&[PROBE]).holding(&[fodder]), Seat::default()],
        );
        to_main(&mut engine, me());
        let source = the(&engine, ZoneLocation::Battlefield, PROBE);
        let target = the(&engine, ZoneLocation::Hand(me()), fodder);
        engine
            .state
            .move_object(
                target,
                ZoneLocation::Graveyard(me()),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        engine.refresh_offer();
        activate(&mut engine, me(), source, 0);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
        settle(&mut engine, me());
        assert_eq!(
            engine.state().object(source).unwrap().zone,
            crate::zone::Zone::Exile
        );
        assert_eq!(
            engine.state().object(target).unwrap().zone,
            crate::zone::Zone::Exile
        );
        assert_eq!(life(&engine, me()), expected_life);
    }
}

#[test]
fn delayed_trigger_at_step_targeting_expiry_and_intervening_if() {
    static DELAYED_PROBE: &[AbilityDef] = &[
        // Ability 0: At the beginning of an opponent's next end step, if you control an artifact,
        // deal 4 damage to any target.
        free(
            &[Effect::RegisterDelayedTrigger {
                when: DelayedTriggerWhen::AtStep {
                    step: DelayedStep::EndStep,
                    whose: Some(PlayerRel::Opponent),
                    this_turn_only: false,
                    recurring: false,
                },
                condition: Some(&Condition::ControlCount(&Filter::ARTIFACT, 1)),
                targets: Some(TargetReq::one(TargetSpec::AnyTarget)),
                remember: TriggerRemember::None,
                effects: &[Effect::DealDamage {
                    amount: Amount::Fixed(4),
                    target: TargetSpec::AnyTarget,
                }],
            }],
            None,
        ),
        // Ability 1: At end of combat this turn, gain 5 life.
        free(
            &[Effect::RegisterDelayedTrigger {
                when: DelayedTriggerWhen::AtStep {
                    step: DelayedStep::CombatEnd,
                    whose: None,
                    this_turn_only: true,
                    recurring: false,
                },
                condition: None,
                targets: None,
                remember: TriggerRemember::None,
                effects: &[Effect::gain_life(5)],
            }],
            None,
        ),
    ];
    static SHATTER: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
        second_targets: None,
        condition: None,
    }];

    let cards = vec![
        card(
            PROBE,
            face("Chronicle", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DELAYED_PROBE,
        ),
        card(
            ARTIFACT_RELIC,
            face("Relic", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            DESTROY_ARTIFACT,
            face("Shatter", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SHATTER,
        ),
    ];

    // 1. Fires on opponent's end step (NOT our end step), asks target when stacked,
    // and fires only once. Also verify `this_turn_only` registered in SecondMain expires at cleanup.
    let mut engine = bench(
        99_854,
        cards.clone(),
        [Seat::with(&[PROBE, ARTIFACT_RELIC]), Seat::default()],
    );
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });
    let probe = the(&engine, ZoneLocation::Battlefield, PROBE);
    activate(&mut engine, me(), probe, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), probe, 1);
    settle(&mut engine, me());

    // Advance past our own end step into opponent's turn: neither trigger may fire at our end step!
    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        life(&engine, me()),
        20,
        "CombatEnd this_turn_only expired at cleanup"
    );
    assert_eq!(
        life(&engine, them()),
        20,
        "Opponent end-step delayed trigger waited past our end step"
    );

    // Walk into opponent's end step: delayed trigger fires and asks `me()` for target!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == me()),
    );
    assert_eq!(engine.state().turn.active, them());
    assert_eq!(engine.state().turn.step, crate::turn::Step::End);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
    });
    assert_eq!(
        life(&engine, them()),
        16,
        "delayed trigger dealt 4 at opponent's end step"
    );

    // Walk through a second opponent turn: one-shot delayed trigger does NOT fire again!
    walk_until(&mut engine, |e| {
        e.state().turn.number >= 5 && holds_priority_in(e, me(), crate::turn::Step::Main)
    });
    assert_eq!(
        life(&engine, them()),
        16,
        "one-shot delayed trigger fired only once"
    );

    // 2. Intervening-if on delayed trigger fails on resolution if artifact is shattered in response!
    let mut shattered = bench(
        99_855,
        cards,
        [
            Seat::with(&[PROBE, ARTIFACT_RELIC]),
            Seat::default().holding(&[DESTROY_ARTIFACT]),
        ],
    );
    to_main(&mut shattered, me());
    let probe = the(&shattered, ZoneLocation::Battlefield, PROBE);
    let relic = the(&shattered, ZoneLocation::Battlefield, ARTIFACT_RELIC);
    activate(&mut shattered, me(), probe, 0);
    settle(&mut shattered, me());
    walk_until(
        &mut shattered,
        |e| matches!(e.pending(), Pending::ChooseTargets { player, .. } if *player == me()),
    );
    shattered
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![them()],
            },
        )
        .unwrap();
    // Opponent holds priority first on their own end step! Shatter the Relic in response!
    let _ = cast_from_hand(&mut shattered, them(), DESTROY_ARTIFACT);
    if matches!(shattered.pending(), Pending::ChooseTargets { .. }) {
        shattered
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![relic],
                    players: vec![],
                },
            )
            .unwrap();
    }
    walk_until(&mut shattered, |e| {
        holds_priority_in(e, me(), crate::turn::Step::Main)
    });
    assert_eq!(
        life(&shattered, them()),
        20,
        "CR 603.4 second check removes delayed trigger when condition fails before resolution"
    );
}

#[test]
fn delayed_trigger_on_event_one_shot_vs_recurring_and_watched_departure() {
    static EVENT_WATCHES: &[AbilityDef] = &[
        // Ability 0: Watch target creature; when it dies this turn, return it to the battlefield.
        free(
            &[Effect::RegisterDelayedTrigger {
                when: DelayedTriggerWhen::OnTrigger {
                    trigger: &Trigger::Dies(&Filter::This),
                    watch: TriggerRemember::FirstTarget,
                    duration: DelayedDuration::OneShotThisTurn,
                },
                condition: None,
                targets: None,
                remember: TriggerRemember::None,
                effects: &[Effect::reanimate(TargetSpec::EventObject)],
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        // Ability 1: When you next cast an instant or sorcery spell this turn, gain 3 life.
        free(
            &[Effect::RegisterDelayedTrigger {
                when: DelayedTriggerWhen::OnTrigger {
                    trigger: &Trigger::SpellCast(&Filter::And(&[
                        Filter::INSTANT_OR_SORCERY,
                        Filter::ControlledByYou,
                    ])),
                    watch: TriggerRemember::None,
                    duration: DelayedDuration::OneShotThisTurn,
                },
                condition: None,
                targets: None,
                remember: TriggerRemember::None,
                effects: &[Effect::gain_life(3)],
            }],
            None,
        ),
        // Ability 2: Whenever you cast an instant or sorcery spell this turn, gain 2 life.
        free(
            &[Effect::RegisterDelayedTrigger {
                when: DelayedTriggerWhen::OnTrigger {
                    trigger: &Trigger::SpellCast(&Filter::And(&[
                        Filter::INSTANT_OR_SORCERY,
                        Filter::ControlledByYou,
                    ])),
                    watch: TriggerRemember::None,
                    duration: DelayedDuration::ThisTurn,
                },
                condition: None,
                targets: None,
                remember: TriggerRemember::None,
                effects: &[Effect::gain_life(2)],
            }],
            None,
        ),
    ];
    static BOUNCE: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::bounce(TargetSpec::Object(&Filter::CREATURE))],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    static SLAY: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
        targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        second_targets: None,
        condition: None,
    }];
    static CANTRIP_PING: &[AbilityDef] = &[AbilityDef::Spell {
        effects: &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::Opponent),
        }],
        targets: None,
        second_targets: None,
        condition: None,
    }];

    let cards = vec![
        card(
            PROBE,
            face("Watchstone", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            EVENT_WATCHES,
        ),
        card(
            FODDER_5_2,
            creature_face("Mortal", "{0}", 3, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BOUNCE_CREATURE,
            face("Unsummon", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BOUNCE,
        ),
        card(
            KILL_CREATURE,
            face("Murder", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SLAY,
        ),
        card(
            PING_INSTANT,
            face("Spark", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            CANTRIP_PING,
        ),
    ];

    // 1. Watched creature dies -> returns to battlefield.
    let mut engine = bench(
        99_856,
        cards.clone(),
        [
            Seat::with(&[PROBE, FODDER_5_2]).holding(&[KILL_CREATURE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let probe = the(&engine, ZoneLocation::Battlefield, PROBE);
    let mortal = the(&engine, ZoneLocation::Battlefield, FODDER_5_2);
    activate(&mut engine, me(), probe, 0);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![mortal],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());

    let _ = cast_from_hand(&mut engine, me(), KILL_CREATURE);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![mortal],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, FODDER_5_2).len(),
        1,
        "watched creature returned to battlefield after dying"
    );

    // 2. Negative (CR 400.7 / CR 603.7b): watched creature bounced to hand, re-cast, then killed ->
    // watch expired on the bounce and does NOT return the new incarnation!
    let mut bounced = bench(
        99_857,
        cards.clone(),
        [
            Seat::with(&[PROBE, FODDER_5_2]).holding(&[BOUNCE_CREATURE, KILL_CREATURE]),
            Seat::default(),
        ],
    );
    to_main(&mut bounced, me());
    let probe = the(&bounced, ZoneLocation::Battlefield, PROBE);
    let mortal = the(&bounced, ZoneLocation::Battlefield, FODDER_5_2);
    activate(&mut bounced, me(), probe, 0);
    if matches!(bounced.pending(), Pending::ChooseTargets { .. }) {
        bounced
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![mortal],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut bounced, me());

    let _ = cast_from_hand(&mut bounced, me(), BOUNCE_CREATURE);
    if matches!(bounced.pending(), Pending::ChooseTargets { .. }) {
        bounced
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![mortal],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut bounced, me());
    let recast = cast_from_hand(&mut bounced, me(), FODDER_5_2);
    settle(&mut bounced, me());

    let _ = cast_from_hand(&mut bounced, me(), KILL_CREATURE);
    if matches!(bounced.pending(), Pending::ChooseTargets { .. }) {
        bounced
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![recast],
                    players: vec![],
                },
            )
            .unwrap();
    }
    settle(&mut bounced, me());
    assert_eq!(
        objects(&bounced, ZoneLocation::Battlefield, FODDER_5_2).len(),
        0,
        "bounce spent the watch on the earlier incarnation (CR 400.7)"
    );
    assert_eq!(
        objects(&bounced, ZoneLocation::Graveyard(me()), FODDER_5_2).len(),
        1
    );

    // 3. OneShotThisTurn vs ThisTurn spell-cast watches:
    let mut spells = bench(
        99_858,
        cards,
        [
            Seat::with(&[PROBE]).holding(&[PING_INSTANT, PING_INSTANT, PING_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut spells, me());
    let probe = the(&spells, ZoneLocation::Battlefield, PROBE);
    activate(&mut spells, me(), probe, 1); // OneShotThisTurn (+3)
    settle(&mut spells, me());
    activate(&mut spells, me(), probe, 2); // ThisTurn (+2 each)
    settle(&mut spells, me());

    // First instant cast triggers BOTH (+3 + +2 = +5 -> 25 life).
    let first = objects(&spells, ZoneLocation::Hand(me()), PING_INSTANT)[0];
    spells
        .apply(me(), PlayerAction::CastSpell { card: first })
        .unwrap();
    settle(&mut spells, me());
    assert_eq!(life(&spells, me()), 25);

    // Second instant cast this turn triggers ONLY the recurring ThisTurn watch (+2 -> 27 life)!
    let second = objects(&spells, ZoneLocation::Hand(me()), PING_INSTANT)[0];
    spells
        .apply(me(), PlayerAction::CastSpell { card: second })
        .unwrap();
    settle(&mut spells, me());
    assert_eq!(
        life(&spells, me()),
        27,
        "OneShotThisTurn fired once; ThisTurn fired again"
    );

    // Next turn, both ThisTurn watches have lapsed -> third instant gives +0 life.
    walk_until(&mut spells, |e| {
        e.state().turn.active == them() && holds_priority_in(e, them(), crate::turn::Step::Main)
    });
    spells.apply(them(), PlayerAction::PassPriority).unwrap();
    let third = objects(&spells, ZoneLocation::Hand(me()), PING_INSTANT)[0];
    spells
        .apply(me(), PlayerAction::CastSpell { card: third })
        .unwrap();
    walk_until(&mut spells, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Main)
    });
    assert_eq!(
        life(&spells, me()),
        27,
        "ThisTurn watch lapsed with the turn"
    );
}

#[test]
fn read_ahead_saga_skips_earlier_chapters_on_entry_turn_and_resumes_normal_crossing() {
    static THREE_CHAPTERS: &[AbilityDef] = &[
        chapter!(1, &[Effect::gain_life(2)]),
        chapter!(2, &[Effect::gain_life(4)]),
        chapter!(3, &[Effect::gain_life(8)]),
    ];
    static BOOST_TWO_LORE: &[AbilityDef] = &[free(
        &[Effect::AddCounter {
            kind: CounterKind::Lore,
            amount: Amount::Fixed(2),
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
    )];
    static DOUBLE_COUNTERS: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::ControlledByYou,
        },
    )];

    let cards = vec![
        card(
            READ_AHEAD_SAGA,
            saga_face("Chronicle of Skipping", "{0}"),
            KeywordSet::READ_AHEAD,
            THREE_CHAPTERS,
        ),
        card(
            LORE_BOOSTER,
            face("Lore Jump", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BOOST_TWO_LORE,
        ),
        card(
            DOUBLER,
            face("Season of Two", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLE_COUNTERS,
        ),
    ];

    // 1. Choose chapter 2 on entry: enters with 2 lore counters; chapter I is suppressed
    // (CR 702.155a), ONLY chapter II triggers (+4 life -> 24).
    let mut ch2 = bench(
        99_859,
        cards.clone(),
        [Seat::default().holding(&[READ_AHEAD_SAGA]), Seat::default()],
    );
    to_main(&mut ch2, me());
    let saga_id = cast_from_hand(&mut ch2, me(), READ_AHEAD_SAGA);
    ch2.apply(me(), PlayerAction::PassPriority).unwrap();
    ch2.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        ch2.pending(),
        Pending::ChooseNumber {
            player,
            min: 1,
            max: 3,
            reason: NumberPrompt::ReadAhead { saga },
        } if *player == me() && *saga == saga_id
    ));
    ch2.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut ch2, me());
    assert_eq!(
        life(&ch2, me()),
        24,
        "choosing chapter 2 triggers only chapter II (+4), not chapter I (+2)"
    );
    assert_eq!(
        ch2.state()
            .object(saga_id)
            .unwrap()
            .counters
            .get(CounterKind::Lore),
        2
    );

    // 2. Choose chapter 1 on entry (+2 -> 22), then on the SAME turn jump +2 lore (`1 -> 3`):
    // because it is still the entry turn and the Saga has Read Ahead, chapter II (`2 != 3`)
    // does NOT trigger — only chapter III (`3 == 3`, +8 -> 30) triggers, then SBA sacrifices!
    let mut same_turn_jump = bench(
        99_860,
        cards.clone(),
        [
            Seat::with(&[LORE_BOOSTER]).holding(&[READ_AHEAD_SAGA]),
            Seat::default(),
        ],
    );
    to_main(&mut same_turn_jump, me());
    let saga_id = cast_from_hand(&mut same_turn_jump, me(), READ_AHEAD_SAGA);
    same_turn_jump
        .apply(me(), PlayerAction::PassPriority)
        .unwrap();
    same_turn_jump
        .apply(them(), PlayerAction::PassPriority)
        .unwrap();
    same_turn_jump
        .apply(me(), PlayerAction::ChooseNumber(1))
        .unwrap();
    settle(&mut same_turn_jump, me());
    assert_eq!(life(&same_turn_jump, me()), 22);

    let booster = the(&same_turn_jump, ZoneLocation::Battlefield, LORE_BOOSTER);
    activate(&mut same_turn_jump, me(), booster, 0);
    if matches!(same_turn_jump.pending(), Pending::ChooseTargets { .. }) {
        same_turn_jump
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![saga_id],
                    players: vec![],
                },
            )
            .unwrap();
    }
    same_turn_jump
        .apply(me(), PlayerAction::PassPriority)
        .unwrap();
    same_turn_jump
        .apply(them(), PlayerAction::PassPriority)
        .unwrap();
    // Chapter III is now on the stack -> CR 715.4 holds back sacrifice while on stack!
    assert_eq!(
        objects(&same_turn_jump, ZoneLocation::Battlefield, READ_AHEAD_SAGA).len(),
        1,
        "CR 715.4 keeps Saga on battlefield while chapter III is on the stack"
    );
    settle(&mut same_turn_jump, me());
    assert_eq!(
        life(&same_turn_jump, me()),
        30,
        "on entry turn, jumping 1 -> 3 with Read Ahead skips chapter II and triggers only III (+8)"
    );
    assert_eq!(
        objects(
            &same_turn_jump,
            ZoneLocation::Graveyard(me()),
            READ_AHEAD_SAGA
        )
        .len(),
        1,
        "CR 715.4 sacrifices Saga once chapter III leaves the stack"
    );

    // 3. On a SUBSEQUENT turn (opponent's turn), jumping +2 lore (`1 -> 3`) triggers BOTH
    // chapter II (+4) and chapter III (+8) (+12 -> 34 total, CR 702.155a / CR 715.2b)!
    let mut later_turn_jump = bench(
        99_861,
        cards.clone(),
        [
            Seat::with(&[LORE_BOOSTER]).holding(&[READ_AHEAD_SAGA]),
            Seat::default(),
        ],
    );
    to_main(&mut later_turn_jump, me());
    let saga_id = cast_from_hand(&mut later_turn_jump, me(), READ_AHEAD_SAGA);
    later_turn_jump
        .apply(me(), PlayerAction::PassPriority)
        .unwrap();
    later_turn_jump
        .apply(them(), PlayerAction::PassPriority)
        .unwrap();
    later_turn_jump
        .apply(me(), PlayerAction::ChooseNumber(1))
        .unwrap();
    settle(&mut later_turn_jump, me());
    assert_eq!(life(&later_turn_jump, me()), 22);

    walk_until(&mut later_turn_jump, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Main)
    });
    later_turn_jump
        .apply(them(), PlayerAction::PassPriority)
        .unwrap();
    let booster = the(&later_turn_jump, ZoneLocation::Battlefield, LORE_BOOSTER);
    activate(&mut later_turn_jump, me(), booster, 0);
    if matches!(later_turn_jump.pending(), Pending::ChooseTargets { .. }) {
        later_turn_jump
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![saga_id],
                    players: vec![],
                },
            )
            .unwrap();
    }
    walk_until(&mut later_turn_jump, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Main)
    });
    assert_eq!(
        life(&later_turn_jump, me()),
        34,
        "after the entry turn, jumping 1 -> 3 triggers both chapter II (+4) and chapter III (+8)"
    );

    // 4. Doubling Season + Read Ahead choosing 2 -> enters with 4 lore counters ->
    // NO chapters equal 4 (CR 702.155a), so 0 life gained and Saga is sacrificed by SBA!
    let mut doubled = bench(
        99_862,
        cards,
        [
            Seat::with(&[DOUBLER]).holding(&[READ_AHEAD_SAGA]),
            Seat::default(),
        ],
    );
    to_main(&mut doubled, me());
    let _ = cast_from_hand(&mut doubled, me(), READ_AHEAD_SAGA);
    doubled.apply(me(), PlayerAction::PassPriority).unwrap();
    doubled.apply(them(), PlayerAction::PassPriority).unwrap();
    doubled.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut doubled, me());
    assert_eq!(
        life(&doubled, me()),
        20,
        "4 lore counters on entry turn matches no chapter 1..=3 under Read Ahead"
    );
    assert_eq!(
        objects(&doubled, ZoneLocation::Graveyard(me()), READ_AHEAD_SAGA).len(),
        1,
        "SBA sacrifices 3-chapter Saga with 4 lore counters and no chapter on stack"
    );
}

#[test]
fn multi_chapter_saga_ability_triggers_per_crossed_symbol_and_supports_read_ahead() {
    static SHARED_CHAPTERS: &[AbilityDef] = &[
        chapters!(&[1, 2], &[Effect::gain_life(3)]),
        chapter!(3, &[Effect::gain_life(10)]),
    ];
    static DOUBLE_COUNTERS: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::ControlledByYou,
        },
    )];

    let cards = vec![
        card(
            MULTI_CHAPTER_SAGA,
            saga_face("Twin Verse", "{0}"),
            KeywordSet::EMPTY,
            SHARED_CHAPTERS,
        ),
        card(
            READ_AHEAD_MULTI_SAGA,
            saga_face("Ahead Verse", "{0}"),
            KeywordSet::READ_AHEAD,
            SHARED_CHAPTERS,
        ),
        card(
            DOUBLER,
            face("Season of Two", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLE_COUNTERS,
        ),
    ];

    // 1. Plain multi-chapter Saga entering under DoubleCounters (`0 -> 2`) crosses both
    // I and II on `Trigger::Chapter(&[1, 2])`, triggering TWICE (+6 life, CR 715.2c)!
    let mut plain = bench(
        99_863,
        cards.clone(),
        [
            Seat::with(&[DOUBLER]).holding(&[MULTI_CHAPTER_SAGA]),
            Seat::default(),
        ],
    );
    to_main(&mut plain, me());
    let _ = cast_from_hand(&mut plain, me(), MULTI_CHAPTER_SAGA);
    settle(&mut plain, me());
    assert_eq!(
        life(&plain, me()),
        26,
        "crossing chapters I and II on shared ability triggers twice (2 * 3 = 6, CR 715.2c)"
    );

    // 2. Read-Ahead multi-chapter Saga choosing 2 on entry (`0 -> 2`) triggers
    // `chapters!(&[1, 2], ...)` ONCE (+3 life, CR 702.155a)!
    let mut ahead = bench(
        99_864,
        cards,
        [
            Seat::default().holding(&[READ_AHEAD_MULTI_SAGA]),
            Seat::default(),
        ],
    );
    to_main(&mut ahead, me());
    let _ = cast_from_hand(&mut ahead, me(), READ_AHEAD_MULTI_SAGA);
    ahead.apply(me(), PlayerAction::PassPriority).unwrap();
    ahead.apply(them(), PlayerAction::PassPriority).unwrap();
    ahead.apply(me(), PlayerAction::ChooseNumber(2)).unwrap();
    settle(&mut ahead, me());
    assert_eq!(
        life(&ahead, me()),
        23,
        "Read Ahead choosing 2 triggers shared I/II ability only for chapter II (+3)"
    );
}
