//! `cards/enchantments/mv_3/mortiphobia.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mortiphobia_exiles_cards_from_graveyards_by_discarding_or_sacrificing_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("4fb8312c-058a-415c-93ab-59d37ebad270");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                card,
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[plains()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let bear = on_battlefield(&game, p0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&game, p1, index::HILL_GIANT).unwrap();
    let fodder = in_hand(&game, p0, plains()).unwrap();
    bury(&mut game, &[bear, giant]);
    game.refresh_offer();

    // Activate ability 0 ({1}{B}, Discard a card) targeting opponent's Hill Giant in graveyard.
    tap_all_mana(&mut game, p0);
    activate(&mut game, p0, card, 0);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![giant],
            players: vec![],
        },
    )
    .unwrap();
    let Pending::ChooseCards { options, .. } = game.pending().clone() else {
        panic!("expected ChooseCards for Discard cost");
    };
    assert!(options.contains(&fodder));
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![fodder],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert_eq!(
        game.state().object(giant).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert!(on_battlefield(&game, p0, card).is_some());

    // Activate ability 1 ({1}{B}, Sacrifice Mortiphobia) targeting p0's Grizzly Bears in graveyard.
    activate(&mut game, p0, card, 1);
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
    assert_eq!(game.state().object(bear).map(|o| o.zone), Some(Zone::Exile));
    assert!(in_graveyard(&game, p0, card).is_some());
}
