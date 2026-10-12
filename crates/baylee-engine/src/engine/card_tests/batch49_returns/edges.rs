//! Additional real-action incarnation, ownership and independent-delay checks.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn glaciers_cleanup_does_not_return_bounced_then_replayed_land() {
    let mut e = prepared(&[index::BOOMERANG], &[index::THAWING_GLACIERS], &[]);
    let land = on_battlefield(&e, P0, index::THAWING_GLACIERS).unwrap();
    e.state.set_tapped(land, false); // preset entry-tap setup
    activate(&mut e, P0, index::THAWING_GLACIERS, 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let old = e.state.object(land).unwrap().version;
    cast_target(&mut e, index::BOOMERANG, land);
    play_land(&mut e, P0, index::THAWING_GLACIERS);
    pass_until(&mut e, stack_is_empty);
    assert_ne!(e.state.object(land).unwrap().version, old);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
}

#[test]
fn maze_returns_stolen_attacker_under_owner_control_and_tapped() {
    let mut e = prepared(
        &[index::CONTROL_MAGIC],
        &[],
        &[index::MYSTIFYING_MAZE, index::GRIZZLY_BEARS],
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_target(&mut e, index::CONTROL_MAGIC, bear);
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn); // controlled continuously since turn began
    attack(&mut e, P0, bear, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P1, .. })
    });
    e.state.players[1].mana_pool.add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, P1, index::MYSTIFYING_MAZE, 1);
    targets(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state.object(bear).unwrap().owner, P1);
    assert_eq!(e.state.object(bear).unwrap().controller, P1);
    assert!(is_tapped(&e, bear));
}
