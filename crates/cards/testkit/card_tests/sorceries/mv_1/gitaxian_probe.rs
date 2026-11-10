//! `cards/sorceries/mv_1/gitaxian_probe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The caster privately inspects the target's hand before drawing a card.
#[test]
fn gitaxian_probe_asks_for_any_player_and_replaces_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[gitaxian_probe()])
        .hand(1, &[forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, gitaxian_probe());

    // The phyrexian symbol may be asked about before the target is; two
    // Islands are sitting there to pay either way, so the first option the
    // engine lists is taken.
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::ChoosePlayer { .. } => break,
            other => panic!("expected the spell's target question, got {other:?}"),
        }
    }

    // A spell whose whole target requirement is a player is asked as
    // `ChoosePlayer` and not as `ChooseTargets`: the wizard branches on
    // `TargetSpec::AnyPlayer` before the object half of targeting runs, so
    // there is no empty object list to inspect — the enumeration of seats
    // *is* the question.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("the probe never asked what to look at")
    };
    assert_eq!(player, p0, "the caster chooses the target");
    assert_eq!(
        options,
        vec![p0, p1],
        "\"target player\" is every seat at the table, the caster included"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent is a player the probe may target");
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
    assert_eq!(options, *engine.state().zones.list(ZoneLocation::Hand(p1)));
    assert_eq!((min, max), (0, 0));
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the draw follows the look"
    );
    assert!(
        engine
            .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
            .is_err()
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, gitaxian_probe()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card spent and one drawn: the Probe is a cantrip"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the card drawn came off the top of the caster's library"
    );

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand,
        "inspection moves no cards"
    );
    assert_eq!(library_size(&engine, p1), their_library);
}
