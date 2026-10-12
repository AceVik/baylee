#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch49_telekinesis_taps_and_skips_exactly_two_controller_untaps() {
    let mut e = prepared(&[index::TELEKINESIS], &[], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_target(&mut e, index::TELEKINESIS, bear);
    assert!(is_tapped(&e, bear));
    for _ in 0..2 {
        let turn = e.state.turn.number;
        main_after(&mut e, P1, turn);
        assert!(is_tapped(&e, bear));
    }
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn batch49_telekinesis_does_not_suppress_a_new_incarnation() {
    let mut e = prepared(&[index::TELEKINESIS], &[], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_target(&mut e, index::TELEKINESIS, bear);
    blink(&mut e, bear, P1);
    e.state.set_tapped(bear, true);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn batch49_cascade_does_not_tap_target_but_suppresses_next_untap_even_if_tapped_later() {
    let mut e = prepared(&[index::SKYLINE_CASCADE], &[], &[index::GRIZZLY_BEARS]);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let land = play_land(&mut e, P0, index::SKYLINE_CASCADE);
    targets(&mut e, vec![bear]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, land));
    assert!(!is_tapped(&e, bear));
    e.state.set_tapped(bear, true);
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(is_tapped(&e, bear));
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn batch49_two_telekinesis_effects_overlap_instead_of_adding_four_skipped_untaps() {
    let mut e = prepared(
        &[index::TELEKINESIS, index::TELEKINESIS],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_target(&mut e, index::TELEKINESIS, bear);
    cast_target(&mut e, index::TELEKINESIS, bear);
    for _ in 0..2 {
        let turn = e.state.turn.number;
        main_after(&mut e, P1, turn);
        assert!(is_tapped(&e, bear));
    }
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, bear));
}

#[test]
fn batch49_telekinesis_prevents_attackers_combat_damage_without_removing_it_from_combat() {
    let mut e = prepared(&[index::TELEKINESIS], &[index::SHIVAN_DRAGON], &[]);
    let dragon = on_battlefield(&e, P0, index::SHIVAN_DRAGON).unwrap();
    let life = e.state.players[1].life;
    attack(&mut e, P0, dragon, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    e.state.players[0].mana_pool.add(ManaColor::Blue, 2);
    cast_target(&mut e, index::TELEKINESIS, dragon);
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert_eq!(e.state.players[1].life, life);
    assert_eq!(e.state.object(dragon).unwrap().zone, Zone::Battlefield);
}

#[test]
fn batch49_bastion_combat_damage_taps_and_suppresses_damaged_creatures_next_untap() {
    let mut e = prepared(&[], &[index::FROSTWALK_BASTION], &[index::GIANT_TURTLE]);
    let land = on_battlefield(&e, P0, index::FROSTWALK_BASTION).unwrap();
    let turtle = on_battlefield(&e, P1, index::GIANT_TURTLE).unwrap();
    e.state.players[0]
        .mana_pool
        .add_snow(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, P0, index::FROSTWALK_BASTION, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(power_of(&e, land), Some(2));
    attack(&mut e, P0, land, P1);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(turtle, land)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
    assert!(is_tapped(&e, turtle));
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(is_tapped(&e, turtle));
    let turn = e.state.turn.number;
    main_after(&mut e, P1, turn);
    assert!(!is_tapped(&e, turtle));
}
