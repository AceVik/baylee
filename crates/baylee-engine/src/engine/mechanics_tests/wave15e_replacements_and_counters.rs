//! Engine Wave 15e deterministic rule tests:
//! 1. `ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard` (*Blightsteel
//!    Colossus*, *Darksteel Colossus*, *Progenitus*; CR 113.6, CR 614.6):
//!    mill, discard, combat death, and countered-on-stack all reveal and
//!    shuffle into the owner's library without touching the graveyard or
//!    triggering dies abilities.
//! 2. `ReplacementRule::EnterWithCountersFromSource` (*Blue, Loyal Raptor*;
//!    CR 614.1c, CR 614.16) and `Effect::PutEachCounterKindFromSource`
//!    (`PutCounter.EachFromSource`; CR 122.6, CR 608.2h), including counter
//!    doublers and LKI sacrifice sources.
//! 3. `Modifier::ModifyLifeGain` (`LifeGainOp::{Multiply, Add}` — *Boon
//!    Reflection*, *Alhammarret's Archive*, *Cleric Class*) alongside
//!    `LifeGainDrawsInstead` / `LifeGainNoLifeInstead`, including commuting
//!    pairs and CR 616.1 affected-player replacement-order choices.
//! 4. `DamageAmountReplacement::Subtract` (*Benevolent Unicorn*), per-recipient
//!    `Modifier::ReplaceOrPreventDamageWithThen` (*Crumbling Sanctuary*
//!    exile-top and damage-to-mill), and `Modifier::PreventDamageRemovingCounters`
//!    (*Bloatfly Swarm* with `Effect::Rad` `DamageRider`).
//! 5. `ReplacementRule::TriggerMultiplier` (`S: Mode$ Panharmonicon` shapes,
//!    including landfall and LKI power-filtered dies triggers).

#![allow(clippy::too_many_lines)] // comprehensive multi-case rule tests

use super::*;
use crate::choice::ChoicePrompt;
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, CounterKind, DamageAmountReplacement, DamageRecipients, DamageRider, EnterModifier,
    Filter, LifeGainOp, Modifier, PlayerRel, ReplacementRule, TargetSpec, ThisWay, Trigger,
    TriggerEventKind, ZoneRef,
};
use baylee_core::generated::subtypes;
use baylee_core::ids::Defender;

const W0: u32 = 99_960;
const W1: u32 = 99_961;
const W2: u32 = 99_962;
const W3: u32 = 99_963;
const W4: u32 = 99_964;
const W5: u32 = 99_965;

fn cast_spell(engine: &mut Bench, seat: PlayerId, id: ObjectId) {
    let (player, legal) = offer(engine);
    assert_eq!(player, seat);
    assert!(
        legal.castable.contains(&id),
        "expected {id:?} castable by {seat:?}, got {:?}",
        legal.castable
    );
    engine
        .apply(seat, PlayerAction::CastSpell { card: id })
        .expect("cast accepted");
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
        .expect("object target accepted");
}

fn aim_player(engine: &mut Bench, seat: PlayerId, target: PlayerId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .expect("player target accepted");
}

// ============================================================================
// 1. ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard (CR 113.6, CR 614.6)
// ============================================================================

#[test]
fn wave15e_blightsteel_shuffle_from_mill_discard_combat_death_and_counterspell() {
    static COLOSSUS_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard {
            filter: &Filter::This,
            reveal: true,
        },
    )];
    static MILL_THREE_AB: &[AbilityDef] = &[spell!(&[Effect::Mill {
        amount: Amount::Fixed(3),
        target: PlayerRel::You,
    }])];
    static DISCARD_ALL_AB: &[AbilityDef] = &[spell!(&[Effect::DiscardHand {
        who: PlayerRel::You,
    }])];
    static DEATH_WATCH_AB: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::CREATURE),
        &[Effect::gain_life(5)]
    )];
    static COUNTER_AB: &[AbilityDef] = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )];

    let defs = || {
        vec![
            card(
                W0,
                creature_face("Colossus Probe", "{0}", 4, 4),
                KeywordSet::EMPTY,
                COLOSSUS_AB,
            ),
            card(
                W1,
                face("Mill Three", "{0}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                MILL_THREE_AB,
            ),
            card(
                W2,
                face("Mind Purge", "{0}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                DISCARD_ALL_AB,
            ),
            card(
                W3,
                face("Death Watch", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                DEATH_WATCH_AB,
            ),
            card(
                W4,
                creature_face("Giant Blocker", "{0}", 5, 5),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                W5,
                face("Free Counter", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                COUNTER_AB,
            ),
        ]
    };

    // Case A: Milled from library — 2 Forests go to graveyard, Colossus reveals & shuffles in.
    {
        let mut engine = bench(
            15_501,
            defs(),
            [Seat::default().holding(&[W0, W1]), Seat::default()],
        );
        to_main(&mut engine, me());
        let colossus = the(&engine, ZoneLocation::Hand(me()), W0);
        engine
            .state
            .move_object(
                colossus,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                Cause::Setup,
            )
            .expect("setup library top");

        let since = engine.journal().last_seq() as usize;
        let mill = the(&engine, ZoneLocation::Hand(me()), W1);
        cast_spell(&mut engine, me(), mill);
        settle(&mut engine, me());

        assert!(
            objects(&engine, ZoneLocation::Graveyard(me()), W0).is_empty(),
            "milled Colossus never enters the graveyard"
        );
        assert_eq!(
            objects(&engine, ZoneLocation::Library(me()), W0),
            vec![colossus],
            "milled Colossus remains in its owner's library"
        );
        // 2 Forests milled + Mill Three spell itself = 3 cards in graveyard.
        assert_eq!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Graveyard(me()))
                .len(),
            3
        );
        let events = &engine.journal().entries()[since..];
        assert!(events.iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { player, cards } if *player == me() && cards == &[colossus]
        )));
        assert!(events.iter().any(|e| matches!(
            &e.event,
            GameEvent::Shuffled { player, zone: Zone::Library } if *player == me()
        )));
        assert!(!events.iter().any(|e| matches!(
            &e.event,
            GameEvent::ZoneChanged { object, to: Zone::Graveyard, .. } if *object == colossus
        )));
    }

    // Case B: Discarded from hand — reveals & shuffles into library instead of graveyard.
    {
        let mut engine = bench(
            15_502,
            defs(),
            [Seat::default().holding(&[W0, W2]), Seat::default()],
        );
        to_main(&mut engine, me());
        let colossus = the(&engine, ZoneLocation::Hand(me()), W0);
        let purge = the(&engine, ZoneLocation::Hand(me()), W2);
        let since = engine.journal().last_seq() as usize;
        cast_spell(&mut engine, me(), purge);
        settle(&mut engine, me());

        assert!(objects(&engine, ZoneLocation::Graveyard(me()), W0).is_empty());
        assert_eq!(
            objects(&engine, ZoneLocation::Library(me()), W0),
            vec![colossus]
        );
        let events = &engine.journal().entries()[since..];
        assert!(events.iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { player, cards } if *player == me() && cards == &[colossus]
        )));
        assert!(events.iter().any(|e| matches!(
            &e.event,
            GameEvent::Shuffled { player, zone: Zone::Library } if *player == me()
        )));
    }

    // Case C: Combat lethal damage — reveals & shuffles into library; `Dies` does NOT trigger!
    {
        let mut engine = bench(
            15_503,
            defs(),
            [Seat::with(&[W0, W3]).at(20), Seat::with(&[W4]).at(20)],
        );
        // Walk to turn 3 (me()'s second turn) so W0 has no summoning sickness.
        walk_until(&mut engine, |e| {
            e.state().turn.number == 3
                && e.state().turn.active == me()
                && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me())
        });
        let colossus = the(&engine, ZoneLocation::Battlefield, W0);
        let blocker = the(&engine, ZoneLocation::Battlefield, W4);
        let since = engine.journal().last_seq() as usize;
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(colossus, Defender::Player(them()))],
                },
            )
            .expect("attack declared");
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == them()),
        );
        engine
            .apply(
                them(),
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, colossus)],
                },
            )
            .expect("block declared");
        walk_until(&mut engine, |e| {
            e.state().turn.phase == Phase::SecondMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
        });

        assert!(objects(&engine, ZoneLocation::Battlefield, W0).is_empty());
        assert!(objects(&engine, ZoneLocation::Graveyard(me()), W0).is_empty());
        assert_eq!(
            objects(&engine, ZoneLocation::Library(me()), W0),
            vec![colossus]
        );
        assert_eq!(
            life(&engine, me()),
            20,
            "Colossus did not die (CR 700.4), so Death Watch never triggered"
        );
        let events = &engine.journal().entries()[since..];
        assert!(events.iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { player, cards } if *player == me() && cards == &[colossus]
        )));
        assert!(events.iter().any(|e| matches!(
            &e.event,
            GameEvent::Shuffled { player, zone: Zone::Library } if *player == me()
        )));
    }

    // Case D: Countered on the stack — reveals & shuffles into owner's library.
    {
        let mut engine = bench(
            15_504,
            defs(),
            [
                Seat::default().holding(&[W0]),
                Seat::default().holding(&[W5]),
            ],
        );
        to_main(&mut engine, me());
        let colossus = the(&engine, ZoneLocation::Hand(me()), W0);
        let counter = the(&engine, ZoneLocation::Hand(them()), W5);
        cast_spell(&mut engine, me(), colossus);
        // Pass priority to `them()` so they counter Colossus on the stack.
        engine.apply(me(), PlayerAction::PassPriority).unwrap();
        cast_spell(&mut engine, them(), counter);
        aim_object(&mut engine, them(), colossus);
        settle(&mut engine, me());

        assert!(objects(&engine, ZoneLocation::Battlefield, W0).is_empty());
        assert!(objects(&engine, ZoneLocation::Graveyard(me()), W0).is_empty());
        assert_eq!(
            objects(&engine, ZoneLocation::Library(me()), W0),
            vec![colossus]
        );
    }
}

// ============================================================================
// 2. EnterWithCountersFromSource & PutEachCounterKindFromSource (Blue, Loyal Raptor)
// ============================================================================

#[test]
fn wave15e_blue_loyal_raptor_enter_with_counters_and_put_each_counter_kind_from_source() {
    static OTHER_DINO_YOU_CTRL: Filter = Filter::And(&[
        Filter::Another,
        Filter::ControlledByYou,
        Filter::HasSubtype(subtypes::creature::DINOSAUR),
    ]);
    static BLUE_AB: &[AbilityDef] = &[
        AbilityDef::Replacement(ReplacementRule::EnterWithCountersFromSource {
            filter: &OTHER_DINO_YOU_CTRL,
        }),
        free(
            &[Effect::PutEachCounterKindFromSource {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        AbilityDef::Activated {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[baylee_cards_dsl::CostPart::SacrificeSelf],
            },
            effects: &[Effect::PutEachCounterKindFromSource {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            second_targets: None,
            third_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];
    static DOUBLER_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::ControlledByYou,
        },
    )];

    let dino_face = |name: &'static str| FaceDef {
        subtypes: &[subtypes::creature::DINOSAUR],
        ..creature_face(name, "{0}", 3, 3)
    };
    let defs = || {
        vec![
            card(W0, dino_face("Blue Raptor"), KeywordSet::EMPTY, BLUE_AB),
            card(W1, dino_face("Baby Raptor"), KeywordSet::EMPTY, &[]),
            card(
                W2,
                creature_face("Plain Bear", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                W3,
                face("Season Doubler", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                DOUBLER_AB,
            ),
        ]
    };

    // Part A: Without Doubling Season, entering Dinosaur gets 1 of each kind held by Blue;
    // non-Dinosaur gets 0.
    {
        let mut engine = bench(
            15_510,
            defs(),
            [Seat::with(&[W0]).holding(&[W1, W2]), Seat::default()],
        );
        to_main(&mut engine, me());
        let blue = the(&engine, ZoneLocation::Battlefield, W0);
        crate::replacement::record_counters(&mut engine.state, blue, CounterKind::P1P1, 3);
        crate::replacement::record_counters(&mut engine.state, blue, CounterKind::Shield, 2);

        let baby = the(&engine, ZoneLocation::Hand(me()), W1);
        cast_spell(&mut engine, me(), baby);
        settle(&mut engine, me());

        let baby_obj = engine.state().object(baby).expect("baby on battlefield");
        assert_eq!(baby_obj.counters.get(CounterKind::P1P1), 1);
        assert_eq!(baby_obj.counters.get(CounterKind::Shield), 1);

        let bear = the(&engine, ZoneLocation::Hand(me()), W2);
        cast_spell(&mut engine, me(), bear);
        settle(&mut engine, me());
        let bear_obj = engine.state().object(bear).expect("bear on battlefield");
        assert_eq!(bear_obj.counters.get(CounterKind::P1P1), 0);
        assert_eq!(bear_obj.counters.get(CounterKind::Shield), 0);
    }

    // Part B: With Doubling Season, both EnterWithCountersFromSource AND
    // PutEachCounterKindFromSource (including LKI sacrifice!) double cleanly.
    {
        let mut engine = bench(
            15_511,
            defs(),
            [Seat::with(&[W0, W2, W3]).holding(&[W1]), Seat::default()],
        );
        to_main(&mut engine, me());
        let blue = the(&engine, ZoneLocation::Battlefield, W0);
        let bear = the(&engine, ZoneLocation::Battlefield, W2);
        crate::replacement::record_counters(&mut engine.state, blue, CounterKind::P1P1, 3);
        crate::replacement::record_counters(&mut engine.state, blue, CounterKind::Shield, 1);
        crate::replacement::record_counters(&mut engine.state, blue, CounterKind::Charge, 2);

        // 1) Baby Raptor enters under Blue + Doubling Season -> 2 of each of the 3 counter kinds!
        let baby = the(&engine, ZoneLocation::Hand(me()), W1);
        cast_spell(&mut engine, me(), baby);
        settle(&mut engine, me());
        let baby_obj = engine.state().object(baby).expect("baby on battlefield");
        assert_eq!(baby_obj.counters.get(CounterKind::P1P1), 2);
        assert_eq!(baby_obj.counters.get(CounterKind::Shield), 2);
        assert_eq!(baby_obj.counters.get(CounterKind::Charge), 2);

        // 2) Sacrifice Blue (`Cost::SAC_SELF`, ability 2) targeting Plain Bear:
        //    Blue is in graveyard on resolution, so LKI counters (CR 608.2h) supply
        //    1 of each kind, doubled to 2 of each by Doubling Season!
        activate(&mut engine, me(), blue, 2);
        aim_object(&mut engine, me(), bear);
        settle(&mut engine, me());

        assert_eq!(
            engine.state().object(blue).expect("blue").zone,
            Zone::Graveyard
        );
        let bear_obj = engine.state().object(bear).expect("bear on battlefield");
        assert_eq!(bear_obj.counters.get(CounterKind::P1P1), 2);
        assert_eq!(bear_obj.counters.get(CounterKind::Shield), 2);
        assert_eq!(bear_obj.counters.get(CounterKind::Charge), 2);
    }
}

// ============================================================================
// 3. ModifyLifeGain (Boon Reflection ×2 + Cleric Class +1 ordering choice)
// ============================================================================

#[test]
fn wave15e_boon_reflection_and_cleric_class_commuting_and_contested_ordering() {
    static BOON_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ModifyLifeGain {
            who: PlayerRel::You,
            op: LifeGainOp::Multiply(2),
        }
    )];
    static CLERIC_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ModifyLifeGain {
            who: PlayerRel::You,
            op: LifeGainOp::Add(1),
        }
    )];
    static LICH_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::LifeGainDrawsInstead {
            who: PlayerRel::You,
        }
    )];
    static GAIN_THREE_AB: &[AbilityDef] = &[free(&[Effect::gain_life(3)], None)];

    let defs = || {
        vec![
            card(
                W0,
                face("Boon Reflection Probe", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                BOON_AB,
            ),
            card(
                W1,
                face("Cleric Class Probe", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                CLERIC_AB,
            ),
            card(
                W2,
                face("Lich Probe", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                LICH_AB,
            ),
            card(
                W3,
                face("Fountain of Three", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                GAIN_THREE_AB,
            ),
        ]
    };

    // Case A: Two Boon Reflections commute without asking a question: 3 * 2 * 2 = 12.
    {
        let mut engine = bench(
            15_520,
            defs(),
            [Seat::with(&[W0, W0, W3]).at(10), Seat::default().at(10)],
        );
        to_main(&mut engine, me());
        let fountain = the(&engine, ZoneLocation::Battlefield, W3);
        activate(&mut engine, me(), fountain, 0);
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 22);
        assert_eq!(engine.state().per_turn.life_gained_amount[0], 12);
    }

    // Case B: Boon Reflection (×2) + Cleric Class (+1) asks affected player (CR 616.1):
    //         Choosing Cleric Class first -> (3 + 1) * 2 = 8 life gained (10 -> 18).
    {
        let mut engine = bench(
            15_521,
            defs(),
            [Seat::with(&[W0, W1, W3]).at(10), Seat::default().at(10)],
        );
        to_main(&mut engine, me());
        let boon = the(&engine, ZoneLocation::Battlefield, W0);
        let cleric = the(&engine, ZoneLocation::Battlefield, W1);
        let fountain = the(&engine, ZoneLocation::Battlefield, W3);
        activate(&mut engine, me(), fountain, 0);
        walk_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::LifeGainReplacement,
                    ..
                }
            )
        });
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            unreachable!()
        };
        assert_eq!(options.len(), 2);
        assert!(options.contains(&boon) && options.contains(&cleric));

        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![cleric],
                },
            )
            .expect("cleric chosen first");
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 18);
        assert_eq!(engine.state().per_turn.life_gained_amount[0], 8);
    }

    // Case C: Boon Reflection (×2) + Cleric Class (+1):
    //         Choosing Boon Reflection first -> (3 * 2) + 1 = 7 life gained (10 -> 17).
    {
        let mut engine = bench(
            15_522,
            defs(),
            [Seat::with(&[W0, W1, W3]).at(10), Seat::default().at(10)],
        );
        to_main(&mut engine, me());
        let boon = the(&engine, ZoneLocation::Battlefield, W0);
        let fountain = the(&engine, ZoneLocation::Battlefield, W3);
        activate(&mut engine, me(), fountain, 0);
        walk_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::LifeGainReplacement,
                    ..
                }
            )
        });
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![boon],
                },
            )
            .expect("boon chosen first");
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 17);
        assert_eq!(engine.state().per_turn.life_gained_amount[0], 7);
    }

    // Case D: Boon Reflection (×2) + Lich (DrawsInstead):
    //         Choosing Boon Reflection first doubles 3 -> 6, then Lich draws 6 cards!
    {
        let mut engine = bench(
            15_523,
            defs(),
            [Seat::with(&[W0, W2, W3]).at(10), Seat::default().at(10)],
        );
        to_main(&mut engine, me());
        let boon = the(&engine, ZoneLocation::Battlefield, W0);
        let fountain = the(&engine, ZoneLocation::Battlefield, W3);
        activate(&mut engine, me(), fountain, 0);
        walk_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::LifeGainReplacement,
                    ..
                }
            )
        });
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![boon],
                },
            )
            .expect("boon chosen before lich");
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 10);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(me())).len(),
            6,
            "Lich saw the doubled 6-life gain (CR 616.2) and drew 6 cards"
        );
    }
}

// ============================================================================
// 4. Benevolent Unicorn (Spell Damage -1), Crumbling Sanctuary, Bloatfly Swarm
// ============================================================================

#[test]
fn wave15e_benevolent_unicorn_subtracts_spell_damage_only() {
    static SPELL_SOURCE: Filter = Filter::InZone(ZoneRef::Stack);
    static UNICORN_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReplaceDamage {
            sources: &SPELL_SOURCE,
            recipients: DamageRecipients::Any,
            combat: None,
            replacement: DamageAmountReplacement::Subtract(1),
        }
    )];
    static BOLT_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(3),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static PING_SPELL_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget))
    )];
    static PING_PERMANENT_AB: &[AbilityDef] = &[free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget,
        }],
        Some(TargetReq::one(TargetSpec::AnyTarget)),
    )];

    let defs = || {
        vec![
            card(
                W0,
                creature_face("Benevolent Unicorn Probe", "{0}", 1, 2),
                KeywordSet::EMPTY,
                UNICORN_AB,
            ),
            card(
                W1,
                face("Bolt Probe", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                BOLT_AB,
            ),
            card(
                W2,
                face("Spark Probe", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                PING_SPELL_AB,
            ),
            card(
                W3,
                creature_face("Tim Pinger", "{0}", 1, 1),
                KeywordSet::EMPTY,
                PING_PERMANENT_AB,
            ),
            card(
                W4,
                creature_face("Fragile Sprite", "{0}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
        ]
    };

    let mut engine = bench(
        15_530,
        defs(),
        [
            Seat::with(&[W0, W3]).holding(&[W1, W2]).at(20),
            Seat::with(&[W4]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let bolt = the(&engine, ZoneLocation::Hand(me()), W1);
    let spark = the(&engine, ZoneLocation::Hand(me()), W2);
    let pinger = the(&engine, ZoneLocation::Battlefield, W3);
    let sprite = the(&engine, ZoneLocation::Battlefield, W4);

    // 1) 3-damage spell to opponent deals 3 - 1 = 2 damage.
    cast_spell(&mut engine, me(), bolt);
    aim_player(&mut engine, me(), them());
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), 18);

    // 2) 1-damage spell to 1/1 Fragile Sprite deals 1 - 1 = 0 damage (survives!).
    cast_spell(&mut engine, me(), spark);
    aim_object(&mut engine, me(), sprite);
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, W4),
        vec![sprite],
        "1-damage spell reduced to 0 leaves 1/1 Sprite alive"
    );

    // 3) Negative check: 1-damage activated ability from battlefield permanent is NOT a spell,
    //    so it deals full 1 damage and kills Fragile Sprite!
    activate(&mut engine, me(), pinger, 0);
    aim_object(&mut engine, me(), sprite);
    settle(&mut engine, me());
    assert!(
        objects(&engine, ZoneLocation::Battlefield, W4).is_empty(),
        "permanent ability damage is not reduced by Benevolent Unicorn"
    );
}

#[test]
fn wave15e_crumbling_sanctuary_and_bloatfly_swarm_riders() {
    const SANCTUARY_RIDER: DamageRider = DamageRider::new(&[Effect::ExileTop {
        who: PlayerRel::DamagedPlayer,
        count: Amount::ThisWay(ThisWay::DamagePrevented),
    }]);
    const RAD_RIDER: DamageRider = DamageRider::new(&[Effect::Rad {
        who: PlayerRel::EachPlayer,
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
    }]);
    const MILL_RIDER: DamageRider = DamageRider::new(&[Effect::Mill {
        amount: Amount::ThisWay(ThisWay::DamagePrevented),
        target: PlayerRel::DamagedPlayer,
    }]);

    static SANCTUARY_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReplaceOrPreventDamageWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Players(PlayerRel::EachPlayer),
            combat: None,
            is_replacement: true,
            then: SANCTUARY_RIDER,
        }
    )];
    static MILL_SANCTUARY_AB: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::ReplaceOrPreventDamageWithThen {
            sources: &Filter::Any,
            recipients: DamageRecipients::Players(PlayerRel::EachOpponent),
            combat: None,
            is_replacement: true,
            then: MILL_RIDER,
        }
    )];
    static BLOATFLY_ENTERS: &[EnterModifier] = &[EnterModifier::WithCounters {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(5),
    }];
    static BLOATFLY_AB: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::PreventDamageRemovingCounters {
            kind: CounterKind::P1P1,
            then: Some(RAD_RIDER),
        }
    )];
    static FLAME_RIFT_AB: &[AbilityDef] = &[spell!(&[Effect::DealDamage {
        amount: Amount::Fixed(3),
        target: TargetSpec::Player(PlayerRel::EachPlayer),
    }])];
    static SLASH_TWO_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static BLAST_TEN_AB: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Fixed(10),
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let defs = || {
        let bloatfly_face = FaceDef {
            enter_modifiers: BLOATFLY_ENTERS,
            ..creature_face("Bloatfly Swarm Probe", "{0}", 0, 0)
        };
        vec![
            card(
                W0,
                face("Crumbling Sanctuary Probe", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                SANCTUARY_AB,
            ),
            card(W1, bloatfly_face, KeywordSet::FLYING, BLOATFLY_AB),
            card(
                W2,
                face("Flame Rift Probe", "{0}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                FLAME_RIFT_AB,
            ),
            card(
                W3,
                face("Slash Two", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                SLASH_TWO_AB,
            ),
            card(
                W4,
                face("Blast Ten", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                BLAST_TEN_AB,
            ),
            card(
                W5,
                face("Mill Sanctuary Probe", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                MILL_SANCTUARY_AB,
            ),
        ]
    };

    // Part A: Crumbling Sanctuary replaces simultaneous damage to BOTH players with
    //         each player exiling that many cards from the top of their own library.
    {
        let mut engine = bench(
            15_540,
            defs(),
            [
                Seat::with(&[W0]).holding(&[W2]).at(20),
                Seat::default().at(20),
            ],
        );
        to_main(&mut engine, me());
        let rift = the(&engine, ZoneLocation::Hand(me()), W2);
        cast_spell(&mut engine, me(), rift);
        settle(&mut engine, me());

        assert_eq!(life(&engine, me()), 20);
        assert_eq!(life(&engine, them()), 20);
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Exile(me())).len(),
            3,
            "me() exiled 3 top cards instead of taking 3 damage"
        );
        assert_eq!(
            engine.state().zones.list(ZoneLocation::Exile(them())).len(),
            3,
            "them() exiled 3 top cards instead of taking 3 damage"
        );
    }

    // Part B: Bloatfly Swarm enters with 5 +1/+1 counters, prevents 2 damage -> removes 2
    //         counters + gives each player 2 rad; then prevents 10 damage -> removes all 3
    //         remaining counters + gives each player 3 rad (5 total, not 12!).
    {
        let mut engine = bench(
            15_541,
            defs(),
            [
                Seat::default().holding(&[W1, W3, W4]).at(20),
                Seat::default().at(20),
            ],
        );
        to_main(&mut engine, me());
        let bloatfly = the(&engine, ZoneLocation::Hand(me()), W1);
        let slash = the(&engine, ZoneLocation::Hand(me()), W3);
        let blast = the(&engine, ZoneLocation::Hand(me()), W4);

        cast_spell(&mut engine, me(), bloatfly);
        settle(&mut engine, me());
        assert_eq!(
            engine
                .state()
                .object(bloatfly)
                .expect("on board")
                .counters
                .get(CounterKind::P1P1),
            5
        );

        cast_spell(&mut engine, me(), slash);
        aim_object(&mut engine, me(), bloatfly);
        settle(&mut engine, me());
        assert_eq!(
            engine
                .state()
                .object(bloatfly)
                .expect("still 3/3")
                .counters
                .get(CounterKind::P1P1),
            3
        );
        assert_eq!(engine.state().players[0].rad, 2);
        assert_eq!(engine.state().players[1].rad, 2);

        cast_spell(&mut engine, me(), blast);
        aim_object(&mut engine, me(), bloatfly);
        settle(&mut engine, me());
        assert!(
            objects(&engine, ZoneLocation::Battlefield, W1).is_empty(),
            "0/0 Bloatfly Swarm dies to SBA once all 3 remaining +1/+1 counters come off"
        );
        assert_eq!(
            engine.state().players[0].rad,
            5,
            "rad counters equal +1/+1 counters actually removed (2 + 3 = 5), not 12"
        );
        assert_eq!(engine.state().players[1].rad, 5);
    }

    // Part C: Damage-to-mill replacement (`Mill` `DamageRider` for `DamagedPlayer`).
    {
        let mut engine = bench(
            15_542,
            defs(),
            [
                Seat::with(&[W5]).holding(&[W2]).at(20),
                Seat::default().at(20),
            ],
        );
        to_main(&mut engine, me());
        let rift = the(&engine, ZoneLocation::Hand(me()), W2);
        cast_spell(&mut engine, me(), rift);
        settle(&mut engine, me());

        // `me()` is not an opponent, so `me()` takes 3 damage; `them()` mills 3 instead!
        assert_eq!(life(&engine, me()), 17);
        assert_eq!(life(&engine, them()), 20);
        assert_eq!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Graveyard(them()))
                .len(),
            3,
            "them() milled 3 cards instead of taking 3 damage"
        );
    }
}

// ============================================================================
// 5. TriggerMultiplier verification (Panharmonicon & Delney LKI shapes)
// ============================================================================

#[test]
fn wave15e_trigger_multiplier_panharmonicon_and_delney_lki_dies() {
    static SMALL_YOUR_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByYou,
        Filter::PowerAtMost(2),
    ]);
    static DELNEY_AB: &[AbilityDef] = &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplier {
            source_filter: &SMALL_YOUR_CREATURE,
            event: TriggerEventKind::Any,
        },
    )];
    static DIES_GAIN_AB: &[AbilityDef] = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::gain_life(2)]
    )];
    static ANTHEM_AB: &[AbilityDef] =
        &[static_ability!(Filter::CREATURE, Modifier::ModifyPT(3, 3))];
    static DESTROY_AB: &[AbilityDef] = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let defs = || {
        vec![
            card(
                W0,
                creature_face("Delney Probe", "{0}", 2, 2),
                KeywordSet::EMPTY,
                DELNEY_AB,
            ),
            card(
                W1,
                creature_face("Small Martyr", "{0}", 2, 2),
                KeywordSet::EMPTY,
                DIES_GAIN_AB,
            ),
            card(
                W2,
                face("Big Anthem", "{0}", TypeSet::ENCHANTMENT),
                KeywordSet::EMPTY,
                ANTHEM_AB,
            ),
            card(
                W3,
                face("Doom Probe", "{0}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                DESTROY_AB,
            ),
        ]
    };

    // Part A: 2/2 Small Martyr dies under Delney -> triggers twice (+4 life).
    {
        let mut engine = bench(
            15_550,
            defs(),
            [
                Seat::with(&[W0, W1]).holding(&[W3]).at(10),
                Seat::default().at(10),
            ],
        );
        to_main(&mut engine, me());
        let martyr = the(&engine, ZoneLocation::Battlefield, W1);
        let doom = the(&engine, ZoneLocation::Hand(me()), W3);
        cast_spell(&mut engine, me(), doom);
        aim_object(&mut engine, me(), martyr);
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), 14);
    }

    // Part B: Negative LKI check — with +3/+3 Anthem on the battlefield, Small Martyr was 5/5
    //         when it died (even though its printed power in the graveyard is 2!), so Delney
    //         must NOT double its dies trigger (+2 life only).
    {
        let mut engine = bench(
            15_551,
            defs(),
            [
                Seat::with(&[W0, W1, W2]).holding(&[W3]).at(10),
                Seat::default().at(10),
            ],
        );
        to_main(&mut engine, me());
        let martyr = the(&engine, ZoneLocation::Battlefield, W1);
        let doom = the(&engine, ZoneLocation::Hand(me()), W3);
        cast_spell(&mut engine, me(), doom);
        aim_object(&mut engine, me(), martyr);
        settle(&mut engine, me());
        assert_eq!(
            life(&engine, me()),
            12,
            "LKI battlefield power was 5, so PowerAtMost(2) did not double the dies trigger"
        );
    }
}
