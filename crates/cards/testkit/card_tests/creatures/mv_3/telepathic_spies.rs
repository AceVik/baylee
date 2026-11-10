//! `cards/creatures/mv_3/telepathic_spies.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn telepathic_spies() -> CardIndex {
    card_index("8162730e-2573-4bc0-827f-5b7b3dbf3bff")
}

/// Telepathic Spies (8162730e-2573-4bc0-827f-5b7b3dbf3bff) — {2}{U} 2/2
/// Creature — Human Wizard:
/// "When this creature enters, look at target opponent's hand."
#[test]
fn telepathic_spies_etb_targets_only_an_opponent_and_looks_at_their_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[telepathic_spies()])
        .hand(1, &[grizzly_bears(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let expected_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();

    cast_from_hand(&mut engine, p0, telepathic_spies());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        player_options,
        vec![p1],
        "only an opponent may be targeted, not the controller"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .is_err(),
        "targeting yourself is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LookAtHand,
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
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(options, expected_hand);
    assert_eq!((min, max), (0, 0));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert_eq!(
        *engine.state().zones.list(ZoneLocation::Hand(p1)),
        expected_hand,
        "looking at the opponent's hand leaves every card in their hand"
    );
}
