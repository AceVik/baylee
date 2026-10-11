//! Exact counter moves use the same all-or-nothing path across printed cards.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)] // shared duel helpers
use super::*;
use baylee_core::{generated::index, ids::DamageSourceRef};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn select_target(e: &mut Engine<RegistryLookup>, object: ObjectId) {
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![object],
        },
    )
    .unwrap();
}

fn start_move(e: &mut Engine<RegistryLookup>, from: ObjectId, to: ObjectId) {
    activate(e, P0, index::NESTING_GROUNDS, 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("first target")
    };
    assert!(options.contains(&from));
    assert!(
        options
            .iter()
            .all(|id| e.state().object(*id).unwrap().controller == P0)
    );
    select_target(e, from);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("second target")
    };
    assert!(options.contains(&to));
    select_target(e, to);
}

#[test]
fn counter_movement_nesting_grounds_chooses_kind_after_targets_and_doubles_only_destination() {
    let mut e = prepared(
        &[],
        &[
            index::NESTING_GROUNDS,
            index::GRIZZLY_BEARS,
            index::FOREST,
            index::DOUBLING_SEASON,
        ],
        &[index::SERRA_ANGEL],
    );
    let from = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let to = on_battlefield(&e, P0, index::FOREST).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    s.object_mut(from)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    s.object_mut(from)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 3);
    s.invalidate_projections();
    e.refresh_offer();
    start_move(&mut e, from, to);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    assert_eq!(counters_on(&e, from, CounterKind::P1P1), 2);
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        unreachable!()
    };
    let option = options
        .iter()
        .find(|o| matches!(o.kind, CastModeKind::Counter(CounterKind::P1P1)))
        .unwrap()
        .index;
    e.apply(
        P0,
        PlayerAction::ChooseMode(usize::try_from(option).unwrap()),
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, from, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&e, from, CounterKind::Charge), 3);
    assert_eq!(counters_on(&e, to, CounterKind::P1P1), 2);
}

#[test]
fn counter_movement_offers_all_counter_kinds_beyond_the_old_u8_option_limit() {
    let mut e = prepared(
        &[],
        &[index::NESTING_GROUNDS, index::FOREST],
        &[index::FOREST],
    );
    let from = on_battlefield(&e, P0, index::FOREST).unwrap();
    let to = on_battlefield(&e, P1, index::FOREST).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    for kind in 0..300 {
        s.object_mut(from)
            .unwrap()
            .counters
            .set(CounterKind::Custom(kind), 1);
    }
    s.invalidate_projections();
    e.refresh_offer();
    start_move(&mut e, from, to);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options.len(), 300);
    assert_eq!(options[299].index, 299);
    e.apply(P0, PlayerAction::ChooseMode(299)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, from, CounterKind::Custom(299)), 0);
    assert_eq!(counters_on(&e, to, CounterKind::Custom(299)), 1);
    assert_eq!(counters_on(&e, from, CounterKind::Custom(298)), 1);
}

#[test]
fn counter_movement_cannot_remove_when_rasputin_cannot_receive_another_dream() {
    let mut e = prepared(
        &[],
        &[
            index::NESTING_GROUNDS,
            index::FOREST,
            index::RASPUTIN_DREAMWEAVER,
        ],
        &[],
    );
    let from = on_battlefield(&e, P0, index::FOREST).unwrap();
    let to = on_battlefield(&e, P0, index::RASPUTIN_DREAMWEAVER).unwrap();
    let kind = baylee_cards_dsl::counters::DREAM;
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(from)
        .unwrap()
        .counters
        .set(kind, 1);
    assert_eq!(counters_on(&e, to, kind), 7);
    e.refresh_offer();
    start_move(&mut e, from, to);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, from, kind), 1);
    assert_eq!(counters_on(&e, to, kind), 7);
}

#[test]
fn counter_movement_identical_or_stale_endpoints_preserve_both_totals() {
    let mut e = prepared(&[], &[index::FOREST], &[index::FOREST]);
    let from = on_battlefield(&e, P0, index::FOREST).unwrap();
    let to = on_battlefield(&e, P1, index::FOREST).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    s.object_mut(from)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 1);
    let source = DamageSourceRef {
        object: from,
        version: s.object(from).unwrap().version,
    };
    let target = DamageSourceRef {
        object: to,
        version: s.object(to).unwrap().version,
    };
    crate::replacement::move_counter(s, P0, source, source, CounterKind::Charge);
    assert_eq!(s.object(from).unwrap().counters.get(CounterKind::Charge), 1);
    s.move_object(to, ZoneLocation::Hand(P1), ZonePosition::Top, Cause::Effect)
        .unwrap();
    s.move_object(
        to,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    )
    .unwrap();
    crate::replacement::move_counter(s, P0, source, target, CounterKind::Charge);
    assert_eq!(s.object(from).unwrap().counters.get(CounterKind::Charge), 1);
    assert_eq!(s.object(to).unwrap().counters.get(CounterKind::Charge), 0);
}
