//! `cards/enchantments/mv_3/pyromania.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn pyromania_deals_one_damage_by_discarding_at_random_or_sacrificing_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("2c89cf19-8dc6-422e-bbf8-4dd5a399c630");
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let lions = on_battlefield(&game, p1, index::SAVANNAH_LIONS).unwrap();

    // Activate ability 0 ({1}{R}, Discard a card at random) targeting p1.
    tap_all_mana(&mut game, p0);
    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert_eq!(game.state().players[1].life, 19);
    assert!(in_graveyard(&game, p0, forest()).is_some());

    // Activate ability 1 ({1}{R}, Sacrifice Pyromania) targeting 2/1 Savannah Lions.
    activate(&mut game, p0, card, 1);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![lions],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert!(on_battlefield(&game, p0, card).is_none());
    assert!(in_graveyard(&game, p0, card).is_some());
    assert!(in_graveyard(&game, p1, index::SAVANNAH_LIONS).is_some());
}
