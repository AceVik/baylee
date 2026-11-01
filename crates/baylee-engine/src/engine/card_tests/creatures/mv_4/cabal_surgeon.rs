//! `cards/creatures/mv_4/cabal_surgeon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn cabal_surgeon_exiles_two_graveyard_cards_to_return_target_creature_card_to_hand() {
    let p0 = PlayerId::new(0);
    let card = card_index("5528d0f1-4774-419f-ba1e-b2abdbce8735");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                card,
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                grizzly_bears(),
                quiet_artifact(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let surgeon = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, surgeon), (2, 1));
    let bear = on_battlefield(&game, p0, grizzly_bears()).unwrap();
    let artifact = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    let extra = on_battlefield(&game, p0, plains()).unwrap();

    // Only one card in graveyard: cannot pay "Exile two cards from your graveyard".
    bury(&mut game, &[bear]);
    game.refresh_offer();
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(surgeon, 0)),
        "requires at least two cards in your graveyard to pay the exile cost"
    );

    // Put two more cards into p0's graveyard so two non-target cards can pay the cost.
    bury(&mut game, &[artifact, extra]);
    game.refresh_offer();

    tap_all_mana(&mut game, p0);
    activate(&mut game, p0, card, 0);

    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", game.pending());
    };
    assert_eq!(
        options,
        vec![bear],
        "only creature cards in your graveyard are legal targets"
    );
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();

    for card_to_exile in [artifact, extra] {
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
    assert!(is_tapped(&game, surgeon));
    assert_eq!(
        game.state().object(artifact).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert_eq!(
        game.state().object(extra).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert!(in_hand(&game, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&game, p0, grizzly_bears()).is_none());
}
