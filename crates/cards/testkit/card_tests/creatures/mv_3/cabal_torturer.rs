//! `cards/creatures/mv_3/cabal_torturer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn cabal_torturer_shrinks_by_one_and_refuses_threshold_ability_below_seven_cards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("fbd295ee-545a-4eab-b000-c0b228dd412d");
    let mut board = vec![card];
    board.extend_from_slice(&[swamp(); 11]);
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let torturer = on_battlefield(&game, p0, card).unwrap();
    let bear = on_battlefield(&game, p1, grizzly_bears()).unwrap();
    assert_eq!(pt(&game, torturer), (1, 1));
    assert_eq!(pt(&game, bear), (2, 2));

    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..6]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(legal.abilities.contains(&(torturer, 0)));
    assert!(
        !legal.abilities.contains(&(torturer, 1)),
        "threshold ability (index 1) must not be offered with 6 cards in graveyard"
    );

    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert_eq!(pt(&game, bear), (1, 1));
}

#[test]
fn cabal_torturer_unlocks_minus_two_ability_at_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("fbd295ee-545a-4eab-b000-c0b228dd412d");
    let mut board = vec![card];
    board.extend_from_slice(&[swamp(); 12]);
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &board)
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let torturer = on_battlefield(&game, p0, card).unwrap();
    let giant = on_battlefield(&game, p1, index::HILL_GIANT).unwrap();
    let swamps = all_on_battlefield(&game, p0, swamp());
    bury(&mut game, &swamps[..7]);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        7
    );

    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        legal.abilities.contains(&(torturer, 1)),
        "threshold ability (index 1) is offered once 7 cards are in graveyard"
    );

    activate(&mut game, p0, card, 1);
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
    assert_eq!(pt(&game, giant), (1, 1));
}
