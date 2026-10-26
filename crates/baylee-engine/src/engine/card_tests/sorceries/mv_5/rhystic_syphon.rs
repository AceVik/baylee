//! `cards/sorceries/mv_5/rhystic_syphon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rhystic_syphon() -> CardIndex {
    card_index("b1eea34a-74ef-4d7c-bf70-c64b3487f296")
}

/// Rhystic Syphon ({3}{B}{B} Sorcery):
/// "Unless target player pays {3}, that player loses 5 life and you gain 5 life."
#[test]
fn rhystic_syphon_drains_five_life_when_target_player_declines_to_pay_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = rhystic_syphon();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[subject])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, subject);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 25);
    assert_eq!(engine.state().players[1].life, 15);
}

/// When the targeted player pays {3}, Rhystic Syphon neither causes life loss nor grants life.
#[test]
fn rhystic_syphon_does_nothing_when_target_player_pays_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = rhystic_syphon();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(0, &[subject])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, subject);
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
