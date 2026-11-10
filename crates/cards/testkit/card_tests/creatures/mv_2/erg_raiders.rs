//! `cards/creatures/mv_2/erg_raiders.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Erg Raiders — `{1}{B}` 2/3: "At the beginning of your end step, if this
/// creature didn't attack this turn, it deals 2 damage to you unless it came
/// under your control this turn." A seated Raiders that stayed home is the
/// plain case, and the damage lands on its controller.
#[test]
fn erg_raiders_deals_two_to_its_controller_when_it_stayed_home() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[erg_raiders()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, erg_raiders()).is_some(),
        "seated, and under its controller's control since the game began"
    );

    pass_until(&mut engine, |e| e.state().players[0].life != 20);

    assert_eq!(
        engine.state().players[0].life,
        18,
        "\"it deals 2 damage to you\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and none of it across the table"
    );
}

/// The same sentence's first clause: the Raiders attacked, so the end step
/// passes with no damage to its controller.
#[test]
fn erg_raiders_deals_nothing_when_it_attacked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[erg_raiders()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let raiders = on_battlefield(&engine, p0, erg_raiders()).expect("seated");

    attack_and_collect_blocks(&mut engine, raiders, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("nothing on the other side can block");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    assert_eq!(
        engine.state().players[0].life,
        20,
        "it attacked this turn, so no 2 damage to its controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "while the unblocked 2/3 dealt its printed 2"
    );
}

/// The exception: "unless it came under your control this turn." The
/// 2018-03-16 ruling says that is entering under your control *or* gaining
/// control of it on the battlefield, so Control Magic takes p1's seated
/// Raiders in p0's first main and p0's own end step charges nothing.
#[test]
fn erg_raiders_deals_nothing_when_control_of_it_was_gained_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[control_magic()])
        .battlefield(1, &[erg_raiders()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let raiders = on_battlefield(&engine, p1, erg_raiders()).expect("their Raiders");

    cast_from_hand(&mut engine, p0, control_magic());
    aim_at(&mut engine, p0, raiders);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(raiders).map(|o| o.controller),
        Some(p0),
        "it came under p0's control this turn"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::End
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "gained control this turn, so the end step spared its new controller"
    );
}
