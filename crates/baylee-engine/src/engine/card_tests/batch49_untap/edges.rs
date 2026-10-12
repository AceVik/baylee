//! Step consumption follows the affected creature, not the effect's source.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn telekinesis_tracks_current_controller_each_untap() {
    let mut e = prepared(
        &[index::TELEKINESIS, index::CONTROL_MAGIC],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_target(&mut e, index::TELEKINESIS, bear);
    cast_target(&mut e, index::CONTROL_MAGIC, bear);
    assert_eq!(e.state.object(bear).unwrap().controller, P0);
    for _ in 0..2 {
        let turn = e.state.turn.number;
        main_after(&mut e, P0, turn);
        assert!(is_tapped(&e, bear), "intervening P1 steps do not count");
    }
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn cascade_consumes_its_step_even_if_target_already_untapped() {
    let mut e = prepared(&[index::SKYLINE_CASCADE], &[], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    play_land(&mut e, P0, index::SKYLINE_CASCADE);
    targets(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
    attack(&mut e, P1, bear, P0);
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert!(is_tapped(&e, bear));
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn telekinesis_does_not_prevent_noncombat_damage() {
    let mut e = prepared(
        &[index::TELEKINESIS, index::TWIDDLE],
        &[index::PRODIGAL_SORCERER],
        &[],
    );
    let sorcerer = on_battlefield(&e, P0, index::PRODIGAL_SORCERER).unwrap();
    cast_target(&mut e, index::TELEKINESIS, sorcerer);
    cast_target(&mut e, index::TWIDDLE, sorcerer);
    assert!(!is_tapped(&e, sorcerer));
    let life = e.state.players[1].life;
    activate(&mut e, P0, index::PRODIGAL_SORCERER, 0);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[1].life, life - 1);
}

#[test]
fn cascade_suppression_survives_source_bounce() {
    let mut e = prepared(
        &[index::SKYLINE_CASCADE, index::BOOMERANG],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let cascade = play_land(&mut e, P0, index::SKYLINE_CASCADE);
    targets(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    cast_target(&mut e, index::BOOMERANG, cascade);
    e.state.set_tapped(bear, true); // later-tap fixture; same recipient incarnation
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(is_tapped(&e, bear));
    assert_eq!(e.state.object(cascade).unwrap().zone, Zone::Hand);
}
