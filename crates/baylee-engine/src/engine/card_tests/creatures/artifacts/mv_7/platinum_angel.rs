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
