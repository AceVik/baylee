use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::{
    effects::{ContinuousEffect, EffectFilter, EffectOrigin},
    object::ObjectKind,
    zone::ZoneLocation,
};
use baylee_cards_dsl::{Duration, Filter, Modifier, ReplacementRule};
use baylee_core::ids::{EffectId, ObjectId};
use std::collections::BTreeSet;

fn results(mut plan: CounterPlacementPlan) -> BTreeSet<u16> {
    if plan.question().is_none() {
        return BTreeSet::from([plan.counters[0].1]);
    }
    let mut yes = plan.clone();
    yes.answer(true);
    plan.answer(false);
    results(yes).union(&results(plan)).copied().collect()
}

fn reference(n: u16, additions: Vec<u16>, doubles: u16, effect: bool) -> BTreeSet<u16> {
    if additions.is_empty() && (doubles == 0 || !effect) {
        return BTreeSet::from([n]);
    }
    let mut out = BTreeSet::new();
    for index in 0..additions.len() {
        let mut rest = additions.clone();
        let extra = rest.remove(index);
        out.extend(reference(n.saturating_add(extra), rest, doubles, true));
    }
    if effect && doubles > 0 {
        out.extend(reference(n.saturating_mul(2), additions, doubles - 1, true));
    }
    out
}

#[test]
fn entry_additions_enable_counter_replacements_only_after_the_first_counter() {
    let plan = |entries, ordinary, doubles| {
        let mut plan = CounterPlacementPlan::new(
            DamageSourceRef {
                object: ObjectId::new(0, 0),
                version: 0,
            },
            PlayerId::new(0),
            CounterKind::P1P1,
            0,
            ordinary,
            doubles,
            true,
        );
        plan.additions.extend((0..entries).map(|_| CounterAddition {
            kind: Some(CounterKind::P1P1),
            n: 1,
        }));
        plan
    };
    assert_eq!(results(plan(1, vec![], 1)), BTreeSet::from([2]));
    assert_eq!(results(plan(2, vec![], 1)), BTreeSet::from([3, 4]));
    assert_eq!(results(plan(1, vec![1], 1)), BTreeSet::from([3, 4]));
    assert_eq!(results(plan(1, vec![1], 0)), BTreeSet::from([2]));
}

#[test]
fn an_additional_counter_cannot_create_a_kind_that_has_not_been_added_yet() {
    let mut plan = CounterPlacementPlan::new(
        DamageSourceRef {
            object: ObjectId::new(0, 0),
            version: 0,
        },
        PlayerId::new(0),
        CounterKind::P1P1,
        0,
        vec![1],
        0,
        true,
    );
    plan.counters.push((CounterKind::Charge, 1));
    plan.additions.push(CounterAddition {
        kind: Some(CounterKind::P1P1),
        n: 1,
    });
    assert_eq!(results(plan), BTreeSet::from([1, 2]));
}

#[test]
fn decision_tree_matches_every_legal_replacement_permutation() {
    for n in 1..=4 {
        for additions in [vec![], vec![1], vec![1, 1], vec![1, 2]] {
            for doubles in 0..=3 {
                for effect in [false, true] {
                    let plan = CounterPlacementPlan::new(
                        DamageSourceRef {
                            object: ObjectId::new(0, 0),
                            version: 0,
                        },
                        PlayerId::new(0),
                        CounterKind::Charge,
                        n,
                        additions.clone(),
                        doubles,
                        effect,
                    );
                    assert_eq!(
                        results(plan),
                        reference(n, additions.clone(), doubles, effect)
                    );
                }
            }
        }
    }
}

fn setup() -> (GameState, ObjectId) {
    let mut state =
        GameState::from_preset(&preset(571, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let player = PlayerId::new(0);
    let name = state.names.intern("counter target");
    let target = state.create_bare(
        player,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let modifier = Modifier::AdditionalCounterPlacement { n: 1 };
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: player,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::Dsl(&Filter::ControlledByYou),
        modifier,
    });
    state
        .replacement_rules
        .push(crate::state::ReplacementEntry {
            source: target,
            controller: player,
            rule: ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::ControlledByYou,
            },
        });
    (state, target)
}

#[test]
fn placement_waits_for_recipient_choice_and_preserves_placing_player() {
    for (add_first, expected) in [(true, 4), (false, 3)] {
        let (mut state, target) = setup();
        let before = state.journal.last_seq();
        super::super::put_counters_by(&mut state, PlayerId::new(1), target, CounterKind::Charge, 1);
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            0
        );
        assert_eq!(
            state.journal.last_seq(),
            before,
            "no speculative placement event"
        );
        assert!(
            matches!(pending(&mut state),Some(Pending::YesNo {player,..}) if player==PlayerId::new(0))
        );
        answer(&mut state, add_first);
        assert!(pending(&mut state).is_none());
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Charge),
            expected
        );
        assert!(
            matches!(state.journal.entries().last().unwrap().event,crate::event::GameEvent::CounterChanged {by:Some(player),new,..} if player==PlayerId::new(1) && new==expected)
        );
    }
}

#[test]
fn cost_counters_enable_doubling_only_after_an_additive_replacement() {
    let (mut state, target) = setup();
    super::super::record_counters_by(
        &mut state,
        PlayerId::new(0),
        target,
        CounterKind::Loyalty,
        1,
    );
    assert!(
        !has_pending(&state),
        "single additive must precede the effect-only doubler"
    );
    assert_eq!(
        state
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        4
    );
    let (mut state, target) = setup();
    state.effects.remove_where(|_| true);
    super::super::record_counters_by(
        &mut state,
        PlayerId::new(0),
        target,
        CounterKind::Loyalty,
        1,
    );
    assert_eq!(
        state
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        1
    );
}

#[test]
fn simultaneous_kinds_share_one_order_and_zero_does_not_create_counters() {
    let (mut state, target) = setup();
    super::super::put_counters_by(&mut state, PlayerId::new(0), target, CounterKind::Charge, 0);
    assert!(!has_pending(&state));
    super::super::counters::put_counter_group_by(
        &mut state,
        PlayerId::new(0),
        target,
        vec![(CounterKind::Charge, 1), (CounterKind::Shield, 2)],
    );
    assert!(pending(&mut state).is_some());
    answer(&mut state, true);
    assert!(pending(&mut state).is_none());
    let object = state.object(target).unwrap();
    assert_eq!(object.counters.get(CounterKind::Charge), 4);
    assert_eq!(object.counters.get(CounterKind::Shield), 6);
}
#[test]
fn simultaneous_batch_waits_for_all_apnap_choices_and_player_counters() {
    let (mut state, first) = setup();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    state.turn.active = p1;
    let name = state.names.intern("second counter target");
    let second = state.create_bare(p1, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let forced = state.create_bare(p0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let make = |id, additions, doubles| {
        CounterPlacementPlan::new(
            DamageSourceRef {
                object: id,
                version: state.object(id).unwrap().version,
            },
            p0,
            CounterKind::Charge,
            1,
            additions,
            doubles,
            true,
        )
    };
    let plans = vec![
        make(first, vec![1], 1),
        make(forced, vec![1], 0),
        make(second, vec![1], 1),
    ];
    let before = state.journal.last_seq();
    enqueue_batch(&mut state, plans, vec![(p0, CounterKind::Energy, 1)]);
    assert!(matches!(pending(&mut state), Some(Pending::YesNo {player,..}) if player == p1));
    answer(&mut state, true);
    assert!(matches!(pending(&mut state), Some(Pending::YesNo {player,..}) if player == p0));
    assert_eq!(state.journal.last_seq(), before);
    assert_eq!(state.players[0].energy, 0);
    for id in [first, forced, second] {
        assert_eq!(
            state.object(id).unwrap().counters.get(CounterKind::Charge),
            0
        );
    }
    answer(&mut state, false);
    assert!(pending(&mut state).is_none());
    for (id, expected) in [(first, 3), (forced, 2), (second, 4)] {
        assert_eq!(
            state.object(id).unwrap().counters.get(CounterKind::Charge),
            expected
        );
    }
    assert_eq!(state.players[0].energy, 1);
}
#[test]
fn forced_arithmetic_matches_repeated_operations_at_saturation() {
    for by_effect in [false, true] {
        for additions in [vec![], vec![1, 2, u16::MAX], vec![1; 70_000]] {
            let mut actual = CounterPlacementPlan::new(
                DamageSourceRef {
                    object: ObjectId::new(0, 0),
                    version: 0,
                },
                PlayerId::new(0),
                CounterKind::Charge,
                1,
                additions,
                0,
                by_effect,
            );
            actual
                .counters
                .extend([(CounterKind::Shield, 0), (CounterKind::P1P1, 32_768)]);
            let mut expected = actual.clone();
            // Independent linear reference performs every individual addition.
            for extra in std::mem::take(&mut expected.additions) {
                for (kind, amount) in &mut expected.counters {
                    if extra.applies_to(*kind, *amount) {
                        *amount = amount.saturating_add(extra.n);
                    }
                }
                expected.by_effect = true;
                expected.next = 0;
            }
            assert_eq!(actual.question(), None);
            assert_eq!(actual, expected);
        }
        for doubles in [0, 1, 2, 15, 16, 17, u16::MAX] {
            let mut actual = CounterPlacementPlan::new(
                DamageSourceRef {
                    object: ObjectId::new(0, 0),
                    version: 0,
                },
                PlayerId::new(0),
                CounterKind::Charge,
                1,
                Vec::new(),
                doubles,
                by_effect,
            );
            actual
                .counters
                .extend([(CounterKind::Shield, 0), (CounterKind::P1P1, 32_768)]);
            let mut expected = actual.clone();
            if by_effect {
                while expected.doubles > 0 {
                    expected.double();
                }
            }
            assert_eq!(actual.question(), None);
            assert_eq!(actual, expected);
        }
    }
}
