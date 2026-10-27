//! `cards/sorceries/mv_7/kamahl_s_sledge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn kamahl_s_sledge_deals_four_to_creature_without_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("defa9b92-4b54-4b00-852e-6c5ec4015643");
    let mut board = vec![];
    board.extend_from_slice(&[mountain(); 13]);
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &board)
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let giant = on_battlefield(&game, p1, index::HILL_GIANT).unwrap();
    let mountains = all_on_battlefield(&game, p0, mountain());
    bury(&mut game, &mountains[..6]);
    game.refresh_offer();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![giant],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(on_battlefield(&game, p1, index::HILL_GIANT).is_none());
    assert_eq!(
        game.state().players[1].life,
        20,
        "without threshold, controller of target creature takes no damage"
    );
}

#[test]
fn kamahl_s_sledge_also_deals_four_to_target_creatures_controller_at_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("defa9b92-4b54-4b00-852e-6c5ec4015643");
    let mut board = vec![];
    board.extend_from_slice(&[mountain(); 14]);
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &board)
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let giant = on_battlefield(&game, p1, index::HILL_GIANT).unwrap();
    let mountains = all_on_battlefield(&game, p0, mountain());
    bury(&mut game, &mountains[..7]);
    game.refresh_offer();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![giant],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(on_battlefield(&game, p1, index::HILL_GIANT).is_none());
    assert_eq!(
        game.state().players[1].life,
        16,
        "with threshold, controller of target creature also takes 4 damage"
    );
}
