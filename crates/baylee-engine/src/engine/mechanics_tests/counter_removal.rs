//! Removing targeted counters and spending the resulting next-spell reduction.

use super::*;
use baylee_cards_dsl::{
    AltCondition, AlternativeCost, Amount, CounterKind, Filter, TargetSpec, ThisWay,
};

const SOURCE: u32 = 99_680;
const OPPONENT_SOURCE: u32 = 99_681;
const TARGET: u32 = 99_682;
const OTHER: u32 = 99_683;
const SPELL: u32 = 99_684;
const ALTERNATIVE: u32 = 99_685;
const TARGETED_SPELL: u32 = 99_686;
const KINDS: [CounterKind; 3] = [CounterKind::P1P1, CounterKind::Charge, CounterKind::Lore];

static CLEANSE: &[AbilityDef] = &[free(
    &[
        Effect::RemoveAllCountersTarget,
        Effect::NextSpellThisTurnCostsLess(Amount::ThisWay(ThisWay::CountersRemovedFromTargets)),
    ],
    Some(TargetReq::up_to_one(TargetSpec::OpponentOrObject(
        &Filter::Any,
    ))),
)];
static BOUNDED: &[AbilityDef] = &[free(
    &[Effect::RemoveUpToCountersTarget { maximum: 3 }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
)];
static GRANT: &[AbilityDef] = &[free(
    &[Effect::NextSpellThisTurnCostsLess(Amount::Fixed(3))],
    None,
)];
static NOTHING: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[],
    targets: None,
    second_targets: None,
    condition: None,
}];
static TARGETED: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[],
    targets: Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
    second_targets: None,
    condition: None,
}];
static ONE_GREEN: &[AlternativeCost] = &[AlternativeCost {
    cost: Cost {
        mana: ManaCost::parse("{1}{G}"),
        parts: &[],
    },
    condition: AltCondition::Always,
}];

fn game(
    seed: u64,
    ours: &'static [AbilityDef],
    theirs: &'static [AbilityDef],
    hand: &[u32],
) -> Bench {
    let source = card(
        SOURCE,
        creature_face("Counter reader", "{0}", 1, 1),
        KeywordSet::EMPTY,
        ours,
    );
    let opposing_source = card(
        OPPONENT_SOURCE,
        creature_face("Opposing reader", "{0}", 1, 1),
        KeywordSet::EMPTY,
        theirs,
    );
    let target = card(
        TARGET,
        creature_face("Counter bearer", "{0}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let other = card(
        OTHER,
        creature_face("Other bearer", "{0}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let spell = card(
        SPELL,
        face("Three generic and green", "{3}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        NOTHING,
    );
    let alternative = card(
        ALTERNATIVE,
        FaceDef {
            alternative_costs: ONE_GREEN,
            ..face("Alternative green", "{5}{G}", TypeSet::INSTANT)
        },
        KeywordSet::EMPTY,
        NOTHING,
    );
    let targeted = card(
        TARGETED_SPELL,
        face("Targeted green", "{3}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        TARGETED,
    );
    let mut engine = bench(
        seed,
        vec![
            source,
            opposing_source,
            target,
            other,
            spell,
            alternative,
            targeted,
        ],
        [
            Seat::with(&[SOURCE, SOURCE, TARGET, forest(), forest()]).holding(hand),
            Seat::with(&[OPPONENT_SOURCE, OTHER, forest()]),
        ],
    );
    to_main(&mut engine, me());
    engine
}

fn counters(engine: &Bench, object: ObjectId, kind: CounterKind) -> u16 {
    engine.state.object(object).unwrap().counters.get(kind)
}

fn total_counters(engine: &Bench, object: ObjectId) -> u32 {
    engine
        .state
        .object(object)
        .unwrap()
        .counters
        .iter()
        .map(|(_, count)| u32::from(count))
        .sum()
}

fn player_counter(engine: &Bench, player: PlayerId, kind: CounterKind) -> u16 {
    engine.state.players[usize::from(player.get())]
        .counters()
        .into_iter()
        .find_map(|(actual, count)| (actual == kind).then_some(count))
        .unwrap_or(0)
}

fn grant(engine: &mut Bench, amount: u32) {
    assert_eq!(
        engine.state.per_turn.next_spell_cost_reduction[0], amount,
        "the reduction granted to this seat"
    );
}

#[track_caller]
fn activate_source(engine: &mut Bench) {
    let source = objects(engine, ZoneLocation::Battlefield, SOURCE)[0];
    activate(engine, me(), source, 0);
}

#[test]
fn bounded_removal_distributes_the_total_and_honours_zero_and_early_stopping() {
    for answers in [
        vec![(1, Some(true)), (1, Some(true)), (1, None)],
        vec![(1, Some(false))],
        vec![(0, Some(false))],
        vec![(0, Some(true)), (1, Some(false))],
    ] {
        let mut engine = game(99_680, BOUNDED, &[], &[SPELL]);
        let target = the(&engine, ZoneLocation::Battlefield, TARGET);
        let other = the(&engine, ZoneLocation::Battlefield, OTHER);
        for (kind, count) in [
            (CounterKind::P1P1, 2),
            (CounterKind::Charge, 2),
            (CounterKind::Lore, 1),
        ] {
            crate::replacement::put_counters(&mut engine.state, target, kind, count);
        }
        crate::replacement::put_counters(&mut engine.state, other, CounterKind::Charge, 3);
        activate_source(&mut engine);
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![target],
                    players: vec![],
                },
            )
            .unwrap();
        walk_until(&mut engine, |engine| {
            matches!(engine.pending(), Pending::ChooseNumber { .. })
        });
        let mut asked = Vec::new();
        let mut expected = 0;
        for (number, continuation) in answers.clone() {
            let Pending::ChooseNumber {
                min,
                max,
                reason:
                    crate::choice::NumberPrompt::Counters {
                        target: asked_target,
                        kind,
                    },
                ..
            } = engine.pending().clone()
            else {
                panic!("a present counter kind is offered: {:?}", engine.pending())
            };
            assert_eq!(asked_target, target);
            assert_eq!(min, 0);
            assert!(max >= 1);
            assert!(counters(&engine, target, kind) > 0);
            asked.push((kind, u16::try_from(number).unwrap()));
            expected += number;
            engine
                .apply(me(), PlayerAction::ChooseNumber(number))
                .unwrap();
            if let Some(yes) = continuation {
                let Pending::YesNo {
                    prompt:
                        crate::choice::YesNoPrompt::ContinueRemovingCounters {
                            target: asked_target,
                            remaining,
                        },
                    ..
                } = engine.pending().clone()
                else {
                    panic!("continue or stop is explicit: {:?}", engine.pending())
                };
                assert_eq!(asked_target, target);
                assert_eq!(remaining, 3 - u16::try_from(expected).unwrap());
                engine.apply(me(), PlayerAction::YesNo(yes)).unwrap();
            }
        }
        settle(&mut engine, me());

        assert_eq!(
            total_counters(&engine, target),
            5 - expected,
            "answers={answers:?}"
        );
        for kind in KINDS {
            let removed: u16 = asked
                .iter()
                .filter_map(|(actual, count)| (*actual == kind).then_some(count))
                .sum();
            assert_eq!(
                counters(&engine, target, kind),
                (if kind == CounterKind::Lore { 1_u16 } else { 2 }).saturating_sub(removed)
            );
        }
        if answers.len() == 3 {
            let mut kinds = asked.iter().map(|(kind, _)| *kind).collect::<Vec<_>>();
            kinds.sort();
            kinds.dedup();
            assert_eq!(kinds.len(), 3, "three distinct kinds are distributed");
        }
        assert_eq!(counters(&engine, other, CounterKind::Charge), 3);
    }
}

#[test]
fn complete_removal_counts_actual_reductions_on_a_permanent_or_an_opponent() {
    for choice in 0..3 {
        let mut engine = game(99_681 + choice, CLEANSE, &[], &[SPELL]);
        let target = the(&engine, ZoneLocation::Battlefield, TARGET);
        let other = the(&engine, ZoneLocation::Battlefield, OTHER);
        for (kind, count) in [
            (CounterKind::P1P1, 2),
            (CounterKind::Charge, 2),
            (CounterKind::Lore, 1),
        ] {
            crate::replacement::put_counters(&mut engine.state, target, kind, count);
        }
        crate::replacement::put_counters(&mut engine.state, other, CounterKind::Charge, 3);
        for (player, kind, count) in [
            (them(), CounterKind::Poison, 3),
            (them(), CounterKind::Energy, 2),
            (them(), CounterKind::Rad, 1),
            (me(), CounterKind::Poison, 4),
        ] {
            engine.state.change_player_counters(player, kind, count);
        }
        activate_source(&mut engine);
        let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
            panic!("an optional permanent or opponent is offered")
        };
        assert_eq!(
            player_options,
            vec![them()],
            "the controller is not an opponent"
        );
        let (objects_answer, players_answer, expected) = match choice {
            0 => (vec![target], vec![], 5),
            1 => (vec![], vec![them()], 6),
            _ => (vec![], vec![], 0),
        };
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: objects_answer,
                    players: players_answer,
                },
            )
            .unwrap();
        settle(&mut engine, me());

        assert_eq!(
            total_counters(&engine, target),
            if choice == 0 { 0 } else { 5 }
        );
        assert_eq!(counters(&engine, other, CounterKind::Charge), 3);
        for kind in [CounterKind::Poison, CounterKind::Energy, CounterKind::Rad] {
            assert_eq!(
                player_counter(&engine, them(), kind),
                if choice == 1 {
                    0
                } else {
                    match kind {
                        CounterKind::Poison => 3,
                        CounterKind::Energy => 2,
                        CounterKind::Rad => 1,
                        _ => unreachable!(),
                    }
                }
            );
        }
        assert_eq!(player_counter(&engine, me(), CounterKind::Poison), 4);
        grant(&mut engine, expected);
        let (_, legal) = offer(&engine);
        let card = the(&engine, ZoneLocation::Hand(me()), SPELL);
        assert_eq!(
            legal.spell_reduction(card, crate::choice::CastModeKind::Normal),
            expected
        );
    }
}

#[test]
fn a_next_spell_reduction_is_published_paid_once_and_private_to_its_controller() {
    let mut engine = game(99_684, GRANT, &[], &[SPELL, SPELL]);
    for source in objects(&engine, ZoneLocation::Battlefield, SOURCE) {
        activate(&mut engine, me(), source, 0);
        settle(&mut engine, me());
    }
    grant(&mut engine, 6);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let (player, legal) = offer(&engine);
    assert_eq!(player, them());
    let theirs = objects(&engine, ZoneLocation::Hand(me()), SPELL)[0];
    assert_eq!(
        legal.spell_reduction(theirs, crate::choice::CastModeKind::Normal),
        0
    );
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    float_all(&mut engine, me());
    let (_, legal) = offer(&engine);
    let first = objects(&engine, ZoneLocation::Hand(me()), SPELL)[0];
    assert_eq!(
        legal.spell_reduction(first, crate::choice::CastModeKind::Normal),
        6
    );
    assert!(
        legal.castable.contains(&first),
        "two green mana and six generic credit pay {{3}}{{G}}"
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: first })
        .unwrap();
    assert_eq!(engine.state.object(first).unwrap().zone, Zone::Stack);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        1,
        "only the green symbol is paid"
    );
    grant(&mut engine, 0);
    settle(&mut engine, me());
    let (_, legal) = offer(&engine);
    let second = objects(&engine, ZoneLocation::Hand(me()), SPELL)[0];
    assert_eq!(
        legal.spell_reduction(second, crate::choice::CastModeKind::Normal),
        0
    );
    assert!(
        !legal.castable.contains(&second),
        "the following spell costs its full price"
    );
}

#[test]
fn an_alternative_cost_uses_the_same_reduction_and_a_cancel_does_not_spend_it() {
    let mut engine = game(99_685, GRANT, &[], &[ALTERNATIVE, TARGETED_SPELL]);
    activate_source(&mut engine);
    settle(&mut engine, me());
    float_all(&mut engine, me());
    let alternative = the(&engine, ZoneLocation::Hand(me()), ALTERNATIVE);
    let (_, legal) = offer(&engine);
    assert!(legal.castable.contains(&alternative));
    assert_eq!(
        legal.spell_reduction(alternative, crate::choice::CastModeKind::Alternative(0)),
        3
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: alternative })
        .unwrap();
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let option = options
            .iter()
            .position(|option| option.kind == crate::choice::CastModeKind::Alternative(0))
            .unwrap();
        engine
            .apply(me(), PlayerAction::ChooseMode(option))
            .unwrap();
    }
    assert!(engine.state.object(alternative).unwrap().alt_cast);
    assert_eq!(engine.state.object(alternative).unwrap().zone, Zone::Stack);
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        1,
        "three generic credit leaves one of the two green mana"
    );
    grant(&mut engine, 0);
    settle(&mut engine, me());

    activate_source(&mut engine);
    settle(&mut engine, me());
    float_all(&mut engine, me());
    let targeted = the(&engine, ZoneLocation::Hand(me()), TARGETED_SPELL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: targeted })
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    engine.apply(me(), PlayerAction::CancelCast).unwrap();
    grant(&mut engine, 3);
    let (_, legal) = offer(&engine);
    assert_eq!(
        legal.spell_reduction(targeted, crate::choice::CastModeKind::Normal),
        3
    );
    engine
        .apply(me(), PlayerAction::CastSpell { card: targeted })
        .unwrap();
    let target = the(&engine, ZoneLocation::Battlefield, OTHER);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(engine.state.object(targeted).unwrap().zone, Zone::Stack);
    grant(&mut engine, 0);
    settle(&mut engine, me());

    activate_source(&mut engine);
    settle(&mut engine, me());
    to_main(&mut engine, them());
    grant(&mut engine, 0);
}
