//! `cards/creatures/mv_1/grim_lavamancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn grim_lavamancer_exiles_two_graveyard_cards_to_deal_two_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("37445e06-88a1-4e2e-a432-383736c9b977");
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &[card, mountain(), grizzly_bears(), quiet_artifact()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let lavamancer = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, lavamancer), (1, 1));
    let bear = on_battlefield(&game, p0, grizzly_bears()).unwrap();
    let artifact = on_battlefield(&game, p0, quiet_artifact()).unwrap();

    // Only one card in graveyard: ability is not offered.
    bury(&mut game, &[bear]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(lavamancer, 0)),
        "requires two cards in graveyard to pay the exile cost"
    );

    bury(&mut game, &[artifact]);
    game.refresh_offer();
    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();

    for card_to_exile in [bear, artifact] {
        let Pending::ChooseCards {
            options,
            min,
            max,
            prompt,
            ..
        } = game.pending().clone()
        else {
            panic!(
                "expected ChooseCards for CostExile, got {:?}",
                game.pending()
            );
        };
        assert_eq!(prompt, crate::choice::ChoicePrompt::CostExile);
        assert_eq!((min, max), (1, 1));
        assert!(options.contains(&card_to_exile));
        game.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![card_to_exile],
            },
        )
        .unwrap();
    }

    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(is_tapped(&game, lavamancer));
    assert_eq!(game.state().object(bear).map(|o| o.zone), Some(Zone::Exile));
    assert_eq!(
        game.state().object(artifact).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert_eq!(game.state().players[1].life, 18);
}
