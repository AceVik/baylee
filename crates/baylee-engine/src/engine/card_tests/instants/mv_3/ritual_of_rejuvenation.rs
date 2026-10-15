// Oracle checklist: resolve on opponent turn; caster alone draws exactly 1; life 4 for caster only; exact library/hand deltas; spell graveyard.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("7c9d8065-9ab0-4468-a11c-383bd036029f")
}

#[test]
fn instant_draws_for_caster_on_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    reach_their_main_phase(&mut game, p1);
    let hand = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library = game.state().zones.list(ZoneLocation::Library(p0)).len();
    let their_hand = game.state().zones.list(ZoneLocation::Hand(p1)).len();
    let life = [game.state().players[0].life, game.state().players[1].life];
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand - 1 + 1
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Library(p0)).len(),
        library - 1
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand
    );
    assert_eq!(game.state().players[0].life, life[0] + 4);
    assert_eq!(game.state().players[1].life, life[1]);
    assert!(in_graveyard(&game, p0, subject()).is_some());
}
