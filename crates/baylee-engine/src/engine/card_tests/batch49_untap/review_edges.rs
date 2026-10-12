//! Additional real-card phase and event-incarnation boundaries.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn cascade_skipped_untap_does_not_consume_suppression() {
    let mut e = prepared(
        &[index::SKYLINE_CASCADE, index::STASIS],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    play_land(&mut e, P0, index::SKYLINE_CASCADE);
    targets(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    e.state.set_tapped(bear, true); // later tap fixture, same incarnation
    let stasis = in_hand(&e, P0, index::STASIS).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: stasis })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(is_tapped(&e, bear));
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(stasis).unwrap().zone, Zone::Graveyard);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(
        is_tapped(&e, bear),
        "first actual untap is still suppressed"
    );
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn telekinesis_duration_can_expire_while_creature_phased_out() {
    let mut e = prepared(&[index::TELEKINESIS], &[], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_target(&mut e, index::TELEKINESIS, bear);
    let version = e.state.object(bear).unwrap().version;
    // Explicit phasing fixture, not a claim about an unfinished card.
    // Indirect phase-out prevents automatic phase-in at the next untap;
    // this isolates expiration while absent for two actual recipient steps.
    e.state
        .object_mut(bear)
        .unwrap()
        .status
        .insert(crate::object::Status::PHASED_OUT);
    e.state
        .object_mut(bear)
        .unwrap()
        .status
        .insert(crate::object::Status::PHASED_OUT_INDIRECTLY);
    e.state.invalidate_projections();
    e.sync_static_effects();
    for _ in 0..2 {
        let turn = e.state.turn.number;
        main_after(&mut e, P1, turn);
        assert!(
            e.state
                .object(bear)
                .unwrap()
                .status
                .contains(crate::object::Status::PHASED_OUT)
        );
    }
    e.state
        .object_mut(bear)
        .unwrap()
        .status
        .remove(crate::object::Status::PHASED_OUT);
    e.state
        .object_mut(bear)
        .unwrap()
        .status
        .remove(crate::object::Status::PHASED_OUT_INDIRECTLY);
    e.state.invalidate_projections();
    e.sync_static_effects();
    assert_eq!(
        e.state.object(bear).unwrap().version,
        version,
        "phasing never changes incarnation"
    );
    assert!(is_tapped(&e, bear));
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(
        !is_tapped(&e, bear),
        "both suppression steps expired while phased out"
    );
}

#[test]
fn bastion_trigger_does_not_tap_or_suppress_victims_reentered_incarnation() {
    let mut e = prepared(
        &[index::UNSUMMON],
        &[index::FROSTWALK_BASTION],
        &[index::GIANT_TURTLE],
    );
    let bastion = on_battlefield(&e, P0, index::FROSTWALK_BASTION).unwrap();
    let turtle = on_battlefield(&e, P1, index::GIANT_TURTLE).unwrap();
    e.state.players[0]
        .mana_pool
        .add_snow(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, P0, index::FROSTWALK_BASTION, 1);
    pass_until(&mut e, stack_is_empty);
    attack(&mut e, P0, bastion, P1);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(turtle, bastion)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatDamage
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    let old = e.state.object(turtle).unwrap().version;
    // Actual response bounce, with explicit direct instant-reentry fixture
    // since no registered immediate land/creature blink spell is required.
    let spell = in_hand(&e, P0, index::UNSUMMON).unwrap();
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    targets(&mut e, vec![turtle]);
    pass_until(&mut e, |e| {
        e.state.object(turtle).is_some_and(|o| o.zone == Zone::Hand)
    });
    e.state
        .move_object(
            turtle,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    assert_ne!(e.state.object(turtle).unwrap().version, old);
    pass_until(&mut e, stack_is_empty);
    assert!(
        !is_tapped(&e, turtle),
        "old trigger cannot tap new incarnation"
    );
    e.state.set_tapped(turtle, true);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(
        !is_tapped(&e, turtle),
        "old trigger cannot suppress new incarnation"
    );
}
