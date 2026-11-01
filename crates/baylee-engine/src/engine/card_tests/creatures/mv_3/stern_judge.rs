//! `cards/creatures/mv_3/stern_judge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn stern_judge_taps_to_make_each_player_lose_life_for_each_swamp_they_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("b4fbf91c-8eed-49d5-91fa-9ae1c03966b7");
    let mut game = Duel::new(SEED, plains())
        .battlefield(0, &[card, swamp(), plains(), plains()])
        .battlefield(1, &[swamp(), swamp(), swamp(), forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    activate(&mut game, p0, card, 0);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert_eq!(
        game.state().players[0].life,
        19,
        "p0 controls 1 Swamp and loses 1 life"
    );
    assert_eq!(
        game.state().players[1].life,
        17,
        "p1 controls 3 Swamps and loses 3 life"
    );
    let _ = p1;
}
