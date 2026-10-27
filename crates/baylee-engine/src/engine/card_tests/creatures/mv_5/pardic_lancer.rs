//! `cards/creatures/mv_5/pardic_lancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn pardic_lancer_discards_at_random_for_plus_one_power_and_first_strike() {
    let p0 = PlayerId::new(0);
    let card = card_index("1bb2b090-19a4-40d9-823a-671f6eaca9f9");
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &[card])
        .hand(0, &[mountain(), forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let lancer = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, lancer), (3, 2));
    assert!(!keywords(&game, lancer).contains(KeywordSet::FIRST_STRIKE));
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut game, p0, card, 0);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );
    assert_eq!(pt(&game, lancer), (4, 2));
    assert!(keywords(&game, lancer).contains(KeywordSet::FIRST_STRIKE));
}
