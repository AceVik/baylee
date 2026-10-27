//! `cards/creatures/mv_5/cephalid_aristocrat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn cephalid_aristocrat_mills_two_cards_when_it_becomes_the_target_of_a_spell() {
    let p0 = PlayerId::new(0);
    let card = card_index("7a0514d0-db8f-402d-abf1-cddda87405cf");
    let mut game = Duel::new(SEED, island())
        .battlefield(0, &[card, forest()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let aristocrat = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, aristocrat), (3, 3));
    let gy_before = game.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let lib_before = game.state().zones.list(ZoneLocation::Library(p0)).len();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, giant_growth());
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![aristocrat],
            players: vec![],
        },
    )
    .unwrap();

    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert_eq!(pt(&game, aristocrat), (6, 6));
    assert_eq!(
        game.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before - 2
    );
    // 2 milled cards + 1 resolved Giant Growth = +3 cards in graveyard.
    assert_eq!(
        game.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 3
    );
}
