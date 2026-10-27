//! `cards/enchantments/mv_4/mortal_combat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mortal_combat_does_not_trigger_with_nineteen_creatures_and_wins_game_with_twenty() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("dc9d1d0c-5310-460a-a3cd-1ff1f4f83c8f");
    let mut board = vec![card];
    board.extend_from_slice(&[index::GRIZZLY_BEARS; 20]);
    board.extend_from_slice(&[swamp(); 5]);
    let mut game = Duel::new(SEED, swamp()).battlefield(0, &board).start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let bears = all_on_battlefield(&game, p0, index::GRIZZLY_BEARS);
    let swamps = all_on_battlefield(&game, p0, swamp());
    // Put 19 creature cards and 5 noncreature cards into p0's graveyard (24 cards total, 19 creatures).
    bury(&mut game, &bears[..19]);
    bury(&mut game, &swamps);
    game.refresh_offer();

    reach_their_main_phase(&mut game, p1);
    reach_their_main_phase(&mut game, p0);
    assert!(
        !matches!(game.pending(), Pending::GameOver(_)),
        "19 creature cards in graveyard (even with 24 total cards) does not trigger Mortal Combat"
    );

    // Put the 20th creature card into p0's graveyard and advance to p0's next upkeep.
    bury(&mut game, &[bears[19]]);
    game.refresh_offer();
    reach_their_main_phase(&mut game, p1);
    pass_until(&mut game, |e| matches!(e.pending(), Pending::GameOver(_)));

    let Pending::GameOver(result) = game.pending().clone() else {
        panic!("expected GameOver");
    };
    assert_eq!(result.winner, Some(crate::win::Victor::Player(p0)));
}
