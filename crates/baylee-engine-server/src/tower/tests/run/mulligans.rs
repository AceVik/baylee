#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn mulligan_redraws_do_not_feed_run_emblems() {
    let player = PlayerId::new(0);
    let lens = item(Family::TideglassLens, Rarity::Common);
    let coil = item(Family::LifeCoil, Rarity::MythicRare);
    let mut game =
        Game::with_detection(&setup(&[lens, coil], &[], 19), Registry::default()).unwrap();
    let starter = game.state().turn.active;
    let opponent_life = game.state().players[1].life;
    game.apply(player, PlayerAction::MulliganTake).unwrap();
    game.apply(player, PlayerAction::MulliganKeep).unwrap();
    game.apply(PlayerId::new(1), PlayerAction::MulliganKeep)
        .unwrap();

    let lens = source(&game, player, lens);
    main(&mut game, 1, starter);
    assert_eq!(charges(&game, lens), 0);
    assert_eq!(game.state().players[1].life, opponent_life);

    let own_draw_turn = if starter == player { 3 } else { 2 };
    main(&mut game, own_draw_turn, player);
    assert_eq!(charges(&game, lens), 1);
    assert_eq!(game.state().players[1].life, opponent_life - 1);
}
