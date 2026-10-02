//! Cyclopean Tomb: its activation, counter duration and recurring cleanup.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters::MIRE;
use baylee_core::generated::{index, subtypes};

fn p0() -> PlayerId {
    PlayerId::new(0)
}
fn p1() -> PlayerId {
    PlayerId::new(1)
}
fn tomb() -> CardIndex {
    index::CYCLOPEAN_TOMB
}
fn upkeep(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(e, |e| {
        e.state().turn.active == p
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p)
    });
}
fn mana(e: &mut Engine<RegistryLookup>, n: u16) {
    e.dev_state_mut(p0()).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, n);
    e.refresh_offer();
}
fn choose(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.apply(p0(), PlayerAction::ChooseObjects { objects: vec![id] })
        .unwrap();
}
fn mark(e: &mut Engine<RegistryLookup>, source: ObjectId, land: ObjectId) {
    mana(e, 2);
    assert!(priority_offer(e).abilities.contains(&(source, 0)));
    e.apply(
        p0(),
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    )
    .unwrap();
    choose(e, land);
}
fn is_swamp(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    e.state()
        .object(id)
        .unwrap()
        .characteristics()
        .subtypes
        .contains(subtypes::land::SWAMP)
}
fn kill(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.dev_state_mut(p0())
        .unwrap()
        .move_object(
            id,
            ZoneLocation::Graveyard(p0()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
}
fn setup(lands: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(1090, forest())
        .battlefield(0, &[tomb()])
        .battlefield(1, lands)
        .start();
    keep_mulligans(&mut e);
    upkeep(&mut e, p0());
    let source = on_battlefield(&e, p0(), tomb()).unwrap();
    (e, source)
}

#[test]
fn cyclopean_tomb_requires_own_upkeep_two_mana_tap_and_non_swamp_land() {
    let (mut e, source) = setup(&[forest(), swamp()]);
    mana(&mut e, 1);
    assert!(!priority_offer(&e).abilities.contains(&(source, 0)));
    mana(&mut e, 1);
    e.apply(
        p0(),
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    )
    .unwrap();
    let land = on_battlefield(&e, p1(), forest()).unwrap();
    let swamp = on_battlefield(&e, p1(), swamp()).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target selection")
    };
    assert!(options.contains(&land));
    assert!(!options.contains(&swamp));
    assert!(!options.contains(&source));
    choose(&mut e, land);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(
        e.state()
            .object(source)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().counters.get(MIRE), 1);
    assert!(is_swamp(&e, land));
    assert!(!is_swamp(&e, source));
    reach_main_phase(&mut e, p0());
    e.dev_state_mut(p0())
        .unwrap()
        .object_mut(source)
        .unwrap()
        .status
        .remove(crate::object::Status::TAPPED);
    mana(&mut e, 2);
    assert!(!priority_offer(&e).abilities.contains(&(source, 0)));
    upkeep(&mut e, p1());
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority {player, ..} if *player == p0()),
    );
    mana(&mut e, 2);
    assert!(!priority_offer(&e).abilities.contains(&(source, 0)));
}

#[test]
fn cyclopean_tomb_duration_survives_source_exile_but_ends_permanently_at_zero() {
    let (mut e, source) = setup(&[index::SNOW_COVERED_FOREST]);
    let land = on_battlefield(&e, p1(), index::SNOW_COVERED_FOREST).unwrap();
    mark(&mut e, source, land);
    pass_until(&mut e, stack_is_empty);
    let c = e.state().object(land).unwrap().characteristics();
    assert!(
        c.supertypes
            .contains(baylee_core::types::SupertypeSet::SNOW)
    );
    assert!(!c.subtypes.contains(subtypes::land::FOREST));
    e.dev_state_mut(p0())
        .unwrap()
        .move_object(
            source,
            ZoneLocation::Exile(p0()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    assert!(is_swamp(&e, land));
    assert!(e.state().delayed.is_empty());
    let state = e.dev_state_mut(p0()).unwrap();
    crate::replacement::put_counters(state, land, MIRE, 1);
    crate::replacement::remove_counters(state, land, MIRE, 1);
    state.refresh_characteristics();
    assert!(
        state
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::SWAMP)
    );
    crate::replacement::remove_counters(state, land, MIRE, 1);
    // No intervening engine step: the effect cannot restart.
    crate::replacement::put_counters(state, land, MIRE, 1);
    e.dev_state_mut(p0()).unwrap().refresh_characteristics();
    e.refresh_offer();
    assert!(!is_swamp(&e, land));
}

#[test]
fn cyclopean_tomb_cleanup_repeats_and_removes_all_counters_from_one_land() {
    let (mut e, source) = setup(&[forest(), island()]);
    let forest = on_battlefield(&e, p1(), forest()).unwrap();
    let island = on_battlefield(&e, p1(), island()).unwrap();
    mark(&mut e, source, forest);
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, p1());
    upkeep(&mut e, p0());
    mark(&mut e, source, island);
    pass_until(&mut e, stack_is_empty);
    crate::replacement::put_counters(e.dev_state_mut(p0()).unwrap(), forest, MIRE, 3);
    kill(&mut e, source);
    pass_until(&mut e, stack_is_empty);
    assert!(is_swamp(&e, forest));
    reach_their_main_phase(&mut e, p1());
    assert_eq!(e.state().object(forest).unwrap().counters.get(MIRE), 4);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!((*min, *max), (1, 1));
    assert_eq!(options.len(), 2);
    choose(&mut e, forest);
    assert_eq!(e.state().object(forest).unwrap().counters.get(MIRE), 0);
    assert!(!is_swamp(&e, forest));
    assert!(is_swamp(&e, island));
    // This instance already cleaned Forest, even if someone adds mire again.
    crate::replacement::put_counters(e.dev_state_mut(p0()).unwrap(), forest, MIRE, 1);
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![island]);
    choose(&mut e, island);
    assert!(!is_swamp(&e, island));
    assert_eq!(e.state().object(forest).unwrap().counters.get(MIRE), 1);
    assert_eq!(
        e.state().delayed.len(),
        1,
        "the recurring trigger lasts for the rest of the game"
    );
}

#[test]
fn cyclopean_tomb_activation_resolves_after_source_dies_and_joins_its_cleanup() {
    let (mut e, source) = setup(&[forest()]);
    let land = on_battlefield(&e, p1(), forest()).unwrap();
    mark(&mut e, source, land);
    kill(&mut e, source);
    pass_until(&mut e, stack_is_empty);
    assert!(is_swamp(&e, land));
    assert_eq!(e.state().delayed.len(), 1);
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    choose(&mut e, land);
    assert!(!is_swamp(&e, land));
}

#[test]
fn cyclopean_tomb_blinked_source_does_not_inherit_the_old_instances_lands() {
    let (mut e, source) = setup(&[forest(), island()]);
    let first = on_battlefield(&e, p1(), forest()).unwrap();
    let second = on_battlefield(&e, p1(), island()).unwrap();
    mark(&mut e, source, first);
    pass_until(&mut e, stack_is_empty);
    let state = e.dev_state_mut(p0()).unwrap();
    for zone in [ZoneLocation::Exile(p0()), ZoneLocation::Battlefield] {
        state
            .move_object(source, zone, ZonePosition::Top, crate::event::Cause::Effect)
            .unwrap();
    }
    e.refresh_offer();
    mark(&mut e, source, second);
    pass_until(&mut e, stack_is_empty);
    kill(&mut e, source);
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![second]);
    choose(&mut e, second);
    assert!(is_swamp(&e, first));
    assert!(!is_swamp(&e, second));
}

#[test]
fn cyclopean_tomb_dead_source_can_return_before_its_death_trigger_resolves() {
    let (mut e, source) = setup(&[forest(), island()]);
    let first = on_battlefield(&e, p1(), forest()).unwrap();
    let second = on_battlefield(&e, p1(), island()).unwrap();
    mark(&mut e, source, first);
    pass_until(&mut e, stack_is_empty);
    kill(&mut e, source);
    // Let the death event be collected, but do not resolve its trigger.
    let Pending::Priority { player, .. } = e.pending() else {
        unreachable!()
    };
    e.apply(*player, PlayerAction::PassPriority).unwrap();
    assert!(!stack_is_empty(&e));
    e.dev_state_mut(p0())
        .unwrap()
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority {player, ..} if *player == p0()),
    );
    mark(&mut e, source, second);
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![first]);
    choose(&mut e, first);
    assert!(!is_swamp(&e, first));
    assert!(is_swamp(&e, second));
}

#[test]
fn cyclopean_tomb_blinked_land_does_not_keep_its_type_or_cleanup_eligibility() {
    let (mut e, source) = setup(&[forest()]);
    let land = on_battlefield(&e, p1(), forest()).unwrap();
    mark(&mut e, source, land);
    pass_until(&mut e, stack_is_empty);
    let state = e.dev_state_mut(p0()).unwrap();
    for zone in [ZoneLocation::Exile(p1()), ZoneLocation::Battlefield] {
        state
            .move_object(land, zone, ZonePosition::Top, crate::event::Cause::Effect)
            .unwrap();
    }
    crate::replacement::put_counters(state, land, MIRE, 2);
    state.refresh_characteristics();
    assert!(!is_swamp(&e, land));
    kill(&mut e, source);
    reach_their_main_phase(&mut e, p1());
    upkeep(&mut e, p0());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().counters.get(MIRE), 2);
}

#[test]
fn cyclopean_tomb_cleanup_can_be_countered_without_canceling_future_upkeeps() {
    let mut e = Duel::new(1091, forest())
        .battlefield(0, &[tomb()])
        .battlefield(1, &[forest()])
        .hand(0, &[index::DISENCHANT, index::TISHANA_S_TIDEBINDER])
        .start();
    keep_mulligans(&mut e);
    upkeep(&mut e, p0());
    let source = on_battlefield(&e, p0(), tomb()).unwrap();
    let land = on_battlefield(&e, p1(), forest()).unwrap();
    mark(&mut e, source, land);
    pass_until(&mut e, stack_is_empty);
    let state = e.dev_state_mut(p0()).unwrap();
    state.players[0].mana_pool.add(ManaColor::White, 1);
    state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0(), index::DISENCHANT);
    choose(&mut e, source);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().delayed.len(), 1);
    reach_their_main_phase(&mut e, p1());
    upkeep(&mut e, p0());
    let trigger = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
    e.dev_state_mut(p0()).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 1);
    mana(&mut e, 2);
    cast_with_floating(&mut e, p0(), index::TISHANA_S_TIDEBINDER);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    choose(&mut e, trigger);
    pass_until(&mut e, stack_is_empty);
    assert!(is_swamp(&e, land));
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    choose(&mut e, land);
    assert!(!is_swamp(&e, land));
}

#[test]
fn cyclopean_tomb_history_is_part_of_snapshot_and_loop_hashes() {
    let (mut e, source) = setup(&[forest()]);
    let land = on_battlefield(&e, p1(), forest()).unwrap();
    mark(&mut e, source, land);
    pass_until(&mut e, stack_is_empty);
    let before = (e.state().snapshot_hash(), e.state().loop_signature());
    e.dev_state_mut(p0()).unwrap().counter_links.clear();
    assert_ne!(before.0, e.state().snapshot_hash());
    assert_ne!(before.1, e.state().loop_signature());
    let state = e.dev_state_mut(p0()).unwrap();
    crate::replacement::remove_counters(state, land, MIRE, u16::MAX);
    state.refresh_characteristics();
    assert!(
        !is_swamp(&e, land),
        "duration expiry does not depend on the cleanup ledger"
    );
}

#[test]
fn cyclopean_tomb_two_instances_recheck_non_swamp_legality_on_resolution() {
    let mut e = Duel::new(1092, forest())
        .battlefield(0, &[tomb(), tomb()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut e);
    upkeep(&mut e, p0());
    let sources = all_on_battlefield(&e, p0(), tomb());
    let land = on_battlefield(&e, p1(), forest()).unwrap();
    mark(&mut e, sources[0], land);
    mark(&mut e, sources[1], land);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(land).unwrap().counters.get(MIRE),
        1,
        "the lower ability lost its only legal target"
    );
    kill(&mut e, sources[0]);
    reach_their_main_phase(&mut e, p1());
    upkeep(&mut e, p0());
    pass_until(&mut e, stack_is_empty);
    assert!(
        is_swamp(&e, land),
        "the first Tomb put no counter on this land"
    );
    kill(&mut e, sources[1]);
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    choose(&mut e, land);
    pass_until(&mut e, stack_is_empty);
    assert!(!is_swamp(&e, land));
}

#[test]
fn cyclopean_tomb_replacement_doubles_counters_and_swamp_produces_black() {
    let mut e = Duel::new(1093, forest())
        .battlefield(0, &[tomb(), forest(), index::DOUBLING_SEASON])
        .start();
    keep_mulligans(&mut e);
    upkeep(&mut e, p0());
    let source = on_battlefield(&e, p0(), tomb()).unwrap();
    let land = on_battlefield(&e, p0(), forest()).unwrap();
    mark(&mut e, source, land);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().counters.get(MIRE), 2);
    e.apply(p0(), PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Black),
        1
    );
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Green),
        0
    );
}

#[test]
fn cyclopean_tomb_same_resolution_return_keeps_the_departing_identity() {
    let (mut e, source) = setup(&[forest(), island()]);
    let first = on_battlefield(&e, p1(), forest()).unwrap();
    let second = on_battlefield(&e, p1(), island()).unwrap();
    mark(&mut e, source, first);
    pass_until(&mut e, stack_is_empty);
    let state = e.dev_state_mut(p0()).unwrap();
    // Two moves in one mutation batch, before any trigger collection.
    for zone in [ZoneLocation::Graveyard(p0()), ZoneLocation::Battlefield] {
        state
            .move_object(source, zone, ZonePosition::Top, crate::event::Cause::Effect)
            .unwrap();
    }
    e.apply(p0(), PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    mark(&mut e, source, second);
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, p1());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![first]);
    choose(&mut e, first);
    assert!(is_swamp(&e, second));
}
