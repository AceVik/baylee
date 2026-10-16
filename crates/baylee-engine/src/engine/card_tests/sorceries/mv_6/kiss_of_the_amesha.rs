//! `cards/sorceries/mv_6/kiss_of_the_amesha.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn subject() -> CardIndex {
    card_index("779d3e58-de06-44be-98f8-f05f7711ecb2")
}

// Both players are legal targets; the named player receives both seven life and two draws.
#[test]
fn gives_both_effects_to_the_target_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 13)
        .life(1, 17)
        .battlefield(
            0,
            &[plains(), island(), plains(), island(), plains(), island()],
        )
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let hand0 = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    let hand1 = game.state().zones.list(ZoneLocation::Hand(p1)).len();
    cast_from_hand(&mut game, p0, subject());
    let Pending::ChoosePlayer {
        options: player_options,
        ..
    } = game.pending().clone()
    else {
        panic!("player target required")
    };
    assert!(player_options.contains(&p0));
    assert!(player_options.contains(&p1));
    game.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 13);
    assert_eq!(game.state().players[1].life, 24);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand0 - 1
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1 + 2
    );
}

#[test]
fn caster_can_receive_both_life_and_cards() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 8)
        .battlefield(
            0,
            &[plains(), island(), plains(), island(), plains(), island()],
        )
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut game, p0, subject());
    game.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 15);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert_eq!(game.state().players[1].life, 20);
}
