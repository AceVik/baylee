//! `cards/creatures/mv_1/putrid_imp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn putrid_imp_discards_for_flying_and_gains_stats_and_cant_block_at_threshold() {
    let p0 = PlayerId::new(0);
    let card = card_index("7192ff07-9a84-49ad-a40c-ce255ce21544");
    let mut board = vec![card];
    board.extend_from_slice(&[swamp(); 6]);
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let imp = on_battlefield(&game, p0, card).unwrap();
    let hand_card = in_hand(&game, p0, forest()).unwrap();
    assert_eq!(pt(&game, imp), (1, 1));
    assert!(!keywords(&game, imp).contains(KeywordSet::FLYING));
    assert!(!keywords(&game, imp).contains(KeywordSet::CANT_BLOCK));

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps);
    game.refresh_offer();
    assert_eq!(pt(&game, imp), (1, 1));

    // Discard the 7th card into the graveyard to gain flying AND turn on threshold.
    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![hand_card],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert_eq!(pt(&game, imp), (2, 2));
    assert!(keywords(&game, imp).contains(KeywordSet::FLYING));
    assert!(keywords(&game, imp).contains(KeywordSet::CANT_BLOCK));
}
