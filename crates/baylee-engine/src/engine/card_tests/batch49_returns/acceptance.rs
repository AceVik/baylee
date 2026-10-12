//! Countering, skipped steps and exact incarnations of real delayed returns.
#[allow(clippy::wildcard_imports)]
use super::*;

fn priority(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player),
    );
}
fn refill(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    for color in ManaColor::ALL {
        e.state.players[usize::from(player.get())]
            .mana_pool
            .add(color, 30);
    }
    e.refresh_offer();
}
fn counter_delayed(e: &mut Engine<RegistryLookup>) {
    let trigger = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    priority(e, P0);
    refill(e, P0);
    cast_with_floating(e, P0, index::TISHANA_S_TIDEBINDER);
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
    targets(e, vec![trigger]);
    pass_until(e, stack_is_empty);
    assert!(on_battlefield(e, P0, index::TISHANA_S_TIDEBINDER).is_some());
}
fn search_fail(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(e, stack_is_empty);
}
fn maze_exile(e: &mut Engine<RegistryLookup>, attacker: ObjectId) {
    let turn = e.state.turn.number;
    main_after(e, P1, turn);
    attack(e, P1, attacker, P0);
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    priority(e, P0);
    refill(e, P0);
    activate(e, P0, index::MYSTIFYING_MAZE, 1);
    targets(e, vec![attacker]);
    pass_until(e, stack_is_empty);
}
fn next_end_trigger(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| {
        e.state.turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
}

#[test]
fn batch49_paradise_skipped_untap_waits_for_next_actual_untap() {
    let mut e = prepared(
        &[index::BOOMERANG],
        &[index::UNDISCOVERED_PARADISE],
        &[index::STASIS],
    );
    let land = on_battlefield(&e, P0, index::UNDISCOVERED_PARADISE).unwrap();
    let stasis = on_battlefield(&e, P1, index::STASIS).unwrap();
    activate(&mut e, P0, index::UNDISCOVERED_PARADISE, 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    // Preserve Stasis through its controller's upkeep using the real payment.
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert_eq!(e.state.turn.active, P1);
    refill(&mut e, P1);
    e.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(stasis).unwrap().zone, Zone::Battlefield);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    assert!(is_tapped(&e, land));
    cast_target(&mut e, index::BOOMERANG, stasis);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
}

#[test]
fn batch49_paradise_control_change_does_not_change_whose_next_untap_returns_it() {
    let mut e = prepared(&[], &[index::UNDISCOVERED_PARADISE], &[]);
    let land = on_battlefield(&e, P0, index::UNDISCOVERED_PARADISE).unwrap();
    activate(&mut e, P0, index::UNDISCOVERED_PARADISE, 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    // Transparent control-transfer fixture; no Implemented land exchange exists.
    e.state.object_mut(land).unwrap().set_controller(P1);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(land).unwrap().controller, P1);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
    assert!(e.state.zones.list(ZoneLocation::Hand(P0)).contains(&land));
}

#[test]
fn batch49_glaciers_countered_cleanup_return_does_not_repeat_in_additional_cleanup_or_later_turn() {
    let mut e = prepared(
        &[index::TISHANA_S_TIDEBINDER],
        &[index::THAWING_GLACIERS],
        &[],
    );
    let land = on_battlefield(&e, P0, index::THAWING_GLACIERS).unwrap();
    e.state.set_tapped(land, false);
    activate(&mut e, P0, index::THAWING_GLACIERS, 0);
    search_fail(&mut e);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::Cleanup && !stack_is_empty(e)
    });
    counter_delayed(&mut e);
    assert_eq!(e.state.turn.step, crate::turn::Step::Cleanup);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
}

#[test]
fn batch49_maze_countered_end_trigger_does_not_retry_next_end_step() {
    let mut e = prepared(
        &[index::TISHANA_S_TIDEBINDER],
        &[index::MYSTIFYING_MAZE],
        &[index::GRIZZLY_BEARS],
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    maze_exile(&mut e, bear);
    next_end_trigger(&mut e);
    counter_delayed(&mut e);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
}

#[test]
fn batch49_maze_return_on_stack_does_not_follow_a_new_exile_incarnation() {
    let mut e = prepared(&[], &[index::MYSTIFYING_MAZE], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    maze_exile(&mut e, bear);
    next_end_trigger(&mut e);
    let old = e.state.object(bear).unwrap().version;
    // Transparent intervening-zone fixture, while the actual delayed trigger
    // remains on the stack. Exile->graveyard->exile is a different object.
    e.state
        .move_object(
            bear,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            bear,
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_ne!(e.state.object(bear).unwrap().version, old);
    e.refresh_offer();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
}

#[test]
fn batch49_maze_exiled_attacking_token_cannot_return() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(1, &[index::RAISE_THE_ALARM])
        .battlefield(0, &[index::MYSTIFYING_MAZE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    priority(&mut e, P1);
    refill(&mut e, P1);
    cast_with_floating(&mut e, P1, index::RAISE_THE_ALARM);
    pass_until(&mut e, stack_is_empty);
    let soldier = e
        .state
        .battlefield_seen()
        .find(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.controller == P1 && o.token.is_some())
        })
        .unwrap();
    maze_exile(&mut e, soldier);
    assert!(!e.state.battlefield_seen().any(|id| id == soldier));
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert!(!e.state.battlefield_seen().any(|id| id == soldier));
    let count = e
        .state
        .battlefield_seen()
        .filter(|id| {
            e.state
                .object(*id)
                .is_some_and(|o| o.controller == P1 && o.token.is_some())
        })
        .count();
    assert_eq!(count, 1);
}

#[test]
fn batch49_glaciers_activated_on_opponents_turn_returns_in_that_turns_cleanup() {
    let mut e = prepared(&[], &[index::THAWING_GLACIERS], &[]);
    let land = on_battlefield(&e, P0, index::THAWING_GLACIERS).unwrap();
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    priority(&mut e, P0);
    refill(&mut e, P0);
    e.state.set_tapped(land, false);
    e.refresh_offer();
    activate(&mut e, P0, index::THAWING_GLACIERS, 0);
    search_fail(&mut e);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::Cleanup && !stack_is_empty(e)
    });
    assert_eq!(e.state.turn.active, P1);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Hand);
}
