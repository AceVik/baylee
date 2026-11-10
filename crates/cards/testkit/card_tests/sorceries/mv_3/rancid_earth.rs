//! `cards/sorceries/mv_3/rancid_earth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn rancid_earth_destroys_target_land_below_threshold_without_sweeping_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("c1972a20-5308-4ee4-b234-751b805d30e4");
    let mut board = vec![swamp(); 9];
    board.push(index::SAVANNAH_LIONS);
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .battlefield(1, &[forest(), index::SAVANNAH_LIONS])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..6]);
    game.refresh_offer();
    let target_land = on_battlefield(&game, p1, forest()).unwrap();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![target_land],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(in_graveyard(&game, p1, forest()).is_some());
    assert!(on_battlefield(&game, p0, index::SAVANNAH_LIONS).is_some());
    assert!(on_battlefield(&game, p1, index::SAVANNAH_LIONS).is_some());
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 20);
}

#[test]
fn rancid_earth_at_threshold_destroys_target_land_and_deals_one_damage_to_each_creature_and_player()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("c1972a20-5308-4ee4-b234-751b805d30e4");
    let mut board = vec![swamp(); 10];
    board.push(index::SAVANNAH_LIONS);
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .battlefield(1, &[forest(), index::GRIZZLY_BEARS])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..7]);
    game.refresh_offer();
    let target_land = on_battlefield(&game, p1, forest()).unwrap();
    let bear = on_battlefield(&game, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![target_land],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(in_graveyard(&game, p1, forest()).is_some());
    assert!(
        in_graveyard(&game, p0, index::SAVANNAH_LIONS).is_some(),
        "1 damage kills 2/1 Savannah Lions"
    );
    assert_eq!(
        game.state().object(bear).unwrap().damage,
        1,
        "2/2 Grizzly Bears takes 1 damage and survives"
    );
    assert_eq!(game.state().players[0].life, 19);
    assert_eq!(game.state().players[1].life, 19);
}
