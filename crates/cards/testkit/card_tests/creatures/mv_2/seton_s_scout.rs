//! `cards/creatures/mv_2/seton_s_scout.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn setons_scout_has_reach_and_gets_plus_two_plus_two_at_threshold() {
    let p0 = PlayerId::new(0);
    let card = card_index("7b2a32ca-ef5b-40a5-acf7-aefac1d09e5b");
    let mut board = vec![card];
    board.extend_from_slice(&[forest(); 8]);
    let mut game = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let scout = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, scout), (2, 1));
    assert!(keywords(&game, scout).contains(KeywordSet::REACH));

    let lands = all_on_battlefield(&game, p0, forest());
    bury(&mut game, &lands[..6]);
    game.refresh_offer();
    assert_eq!(pt(&game, scout), (2, 1));

    bury(&mut game, &[lands[6]]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    assert_eq!(pt(&game, scout), (4, 3));
}
