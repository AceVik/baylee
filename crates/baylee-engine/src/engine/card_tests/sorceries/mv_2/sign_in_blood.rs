// Oracle checklist: Target player draws two cards and loses 2 life.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("c6207f6a-a624-4754-88f5-dbe700c841ff")
}

// Either player may be targeted; the same recipient draws two and loses two life.
#[test]
fn opponent_draws_two_and_loses_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .life(1, 19)
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
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
    assert_eq!(game.state().players[0].life, 17);
    assert_eq!(game.state().players[1].life, 17);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1 + 2
    );
}

#[test]
fn caster_can_target_self_for_two_cards_at_two_life() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 9)
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut game, p0, subject());
    game.apply(p0, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 7);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert_eq!(game.state().players[1].life, 20);
}
