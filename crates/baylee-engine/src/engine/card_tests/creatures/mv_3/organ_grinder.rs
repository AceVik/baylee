//! `cards/creatures/mv_3/organ_grinder.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn organ_grinder_exiles_three_graveyard_cards_to_make_target_player_lose_three_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("a9f279a5-c35f-4703-a583-050f2dffb28e");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[card, swamp(), plains(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let grinder = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, grinder), (3, 1));
    let c1 = on_battlefield(&game, p0, swamp()).unwrap();
    let c2 = on_battlefield(&game, p0, plains()).unwrap();
    let c3 = on_battlefield(&game, p0, index::GRIZZLY_BEARS).unwrap();

    // Only two cards in graveyard: ability is not offered.
    bury(&mut game, &[c1, c2]);
    game.refresh_offer();
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(grinder, 0)),
        "requires three cards in graveyard to pay the exile cost"
    );

    bury(&mut game, &[c3]);
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

    for card_to_exile in [c1, c2, c3] {
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
    assert!(is_tapped(&game, grinder));
    for exiled in [c1, c2, c3] {
        assert_eq!(
            game.state().object(exiled).map(|o| o.zone),
            Some(Zone::Exile)
        );
    }
    assert_eq!(game.state().players[1].life, 17);
}
