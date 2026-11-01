//! Platinum Angel prevents both the controller's loss and the opponent's win.

use super::*;

fn platinum_angel() -> CardIndex {
    card_index("b148578c-c0bf-4785-b97c-4b6f83028008")
}

#[test]
fn platinum_angel_keeps_game_going_at_zero_life_and_has_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[platinum_angel(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[lightning_bolt()])
        .life(0, 3)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p0],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 0);
    assert!(!e.state().players[0].has_lost());
    assert!(
        !matches!(e.pending(), Pending::GameOver(_)),
        "the opponent has not won either"
    );

    let angel = on_battlefield(&e, p0, platinum_angel()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    let options = attack_and_collect_blocks(&mut e, angel, p1);
    assert!(
        !options
            .iter()
            .any(|option| { option.blocker == ogre && option.attackers.contains(&angel) })
    );
}

/// "Your opponents can't win the game": the opponent's Test of Endurance
/// resolves at their upkeep with 50 life and the game goes on. With the
/// Angel destroyed, the same board wins at the opponent's next upkeep —
/// the control that shows the trigger was armed all along.
#[test]
fn platinum_angel_stops_an_opponent_s_effect_win() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[platinum_angel()])
        .battlefield(1, &[baylee_core::generated::index::TEST_OF_ENDURANCE])
        .life(1, 50)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(e.state().players[1].life, 50);
    assert_eq!(e.state().game_end, None, "p1 could not win");

    let angel = on_battlefield(&e, p0, platinum_angel()).unwrap();
    kill(&mut e, angel);
    assert!(on_battlefield(&e, p0, platinum_angel()).is_none());
    pass_until(&mut e, |e| e.state().game_end.is_some());
    assert_eq!(
        e.state().game_end,
        Some(crate::win::GameResult {
            winner: Some(crate::win::Victor::Player(p1)),
            reason: crate::win::EndReason::EffectWin,
        })
    );
}

/// "You can't lose the game" ends with the Angel: its controller sitting at
/// 0 life loses as soon as it is gone.
#[test]
fn platinum_angel_leaving_lets_its_zero_life_controller_lose() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[platinum_angel(), mountain()])
        .hand(0, &[lightning_bolt()])
        .life(0, 3)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p0],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 0);
    assert!(!e.state().players[0].has_lost());

    let angel = on_battlefield(&e, p0, platinum_angel()).unwrap();
    kill(&mut e, angel);
    assert!(e.state().players[0].has_lost());
}
