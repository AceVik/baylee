//! `cards/creatures/mv_4/nantuko_calmer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn nantuko_calmer_gets_plus_one_at_threshold_and_sacrifices_to_destroy_target_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("d4409b15-b9f8-4bdb-9857-5cd70a0d3193");
    let mut board = vec![card];
    board.extend_from_slice(&[forest(); 8]);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::GLORIOUS_ANTHEM])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let calmer = on_battlefield(&game, p0, card).unwrap();
    let anthem = on_battlefield(&game, p1, index::GLORIOUS_ANTHEM).unwrap();
    assert_eq!(pt(&game, calmer), (2, 3));

    let lands = all_on_battlefield(&game, p0, forest());
    bury(&mut game, &lands[..6]);
    game.refresh_offer();
    assert_eq!(pt(&game, calmer), (2, 3));

    bury(&mut game, &[lands[6]]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    assert_eq!(pt(&game, calmer), (3, 4));

    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![anthem],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(in_graveyard(&game, p0, card).is_some());
    assert!(in_graveyard(&game, p1, index::GLORIOUS_ANTHEM).is_some());
}
