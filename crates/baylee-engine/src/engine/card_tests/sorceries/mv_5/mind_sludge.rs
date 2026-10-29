//! `cards/sorceries/mv_5/mind_sludge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn mind_sludge_makes_target_player_discard_for_each_swamp_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("86e64a29-6ed1-451b-a114-f74341322eeb");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), plains(), plains()])
        .battlefield(1, &[swamp()])
        .hand(0, &[card])
        .hand(1, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let gy_before = game.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    // p0 controls 3 Swamps -> target player discards 3 of their 4 cards (keeps 1).
    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    game.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut game, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Keep,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = game.pending().clone()
    else {
        panic!("expected ChooseCards(Keep)");
    };
    assert_eq!(player, p1);
    assert_eq!(min, 1);
    assert_eq!(max, 1);
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![options[0]],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p1)),
        &[options[0]]
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        gy_before + 3
    );
}
