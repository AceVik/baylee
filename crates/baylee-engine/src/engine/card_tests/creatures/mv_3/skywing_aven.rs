//! `cards/creatures/mv_3/skywing_aven.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn skywing_aven_has_flying_and_discards_a_card_to_return_itself_to_hand() {
    let p0 = PlayerId::new(0);
    let card = card_index("afc3a65e-7a7a-4bd9-bca4-aa762b9bbbcb");
    let mut game = Duel::new(SEED, island())
        .battlefield(0, &[card])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let aven = on_battlefield(&game, p0, card).unwrap();
    let discard_card = in_hand(&game, p0, forest()).unwrap();
    assert_eq!(pt(&game, aven), (2, 1));
    assert!(keywords(&game, aven).contains(KeywordSet::FLYING));

    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard_card],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(on_battlefield(&game, p0, card).is_none());
    assert!(in_hand(&game, p0, card).is_some());
    assert!(in_graveyard(&game, p0, forest()).is_some());
}
