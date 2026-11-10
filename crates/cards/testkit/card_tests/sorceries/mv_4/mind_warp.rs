//! `cards/sorceries/mv_4/mind_warp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_warp() -> CardIndex {
    card_index("5109aa76-ca40-44d1-abb4-5e7a23a2c274")
}

/// Casts Mind Warp with exactly its required mana and stops at its private
/// hand-inspection and discard choice.
fn resolving_mind_warp(
    x: u32,
    target_hand: &[CardIndex],
) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let lands = vec![swamp(); 4 + x as usize];
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &lands)
        .hand(0, &[mind_warp()])
        .hand(1, target_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let hand: Vec<_> = target_hand
        .iter()
        .map(|card| in_hand(&engine, P1, *card).unwrap())
        .collect();

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, mind_warp());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(x)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LookAtHand,
                ..
            }
        )
    });
    (engine, hand)
}

/// Mind Warp (5109aa76-ca40-44d1-abb4-5e7a23a2c274) — {X}{3}{B} Sorcery:
/// "Look at target player's hand and choose X cards from it. That player discards those cards."
#[test]
fn mind_warp_discards_exactly_x_cards_chosen_from_the_looked_at_hand() {
    let (mut engine, hand) = resolving_mind_warp(2, &[forest(), island(), mountain()]);
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
    assert_eq!(player, P0, "Mind Warp's controller chooses");
    assert_eq!(options, hand, "the entire targeted hand is inspected");
    assert_eq!((min, max), (2, 2), "X requires exactly two choices");

    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![hand[0]],
                },
            )
            .is_err(),
        "fewer than X choices cannot resolve the instruction"
    );
    assert!(in_hand(&engine, P1, forest()).is_some());
    assert!(in_hand(&engine, P1, island()).is_some());

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![hand[0], hand[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert!(in_graveyard(&engine, P1, island()).is_some());
    assert!(in_hand(&engine, P1, mountain()).is_some());
    assert!(in_graveyard(&engine, P0, mind_warp()).is_some());
}

#[test]
fn mind_warp_with_zero_x_still_looks_but_discards_nothing() {
    let (mut engine, hand) = resolving_mind_warp(0, &[forest(), island()]);
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options, hand);
    assert_eq!((min, max), (0, 0));

    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P1, forest()).is_some());
    assert!(in_hand(&engine, P1, island()).is_some());
    assert!(in_graveyard(&engine, P1, forest()).is_none());
    assert!(in_graveyard(&engine, P1, island()).is_none());
}

#[test]
fn mind_warp_discards_a_smaller_whole_hand_when_x_exceeds_its_size() {
    let (mut engine, hand) = resolving_mind_warp(3, &[forest(), island()]);
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options, hand);
    assert_eq!((min, max), (2, 2), "the hand size limits X");

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: hand.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, forest()).is_some());
    assert!(in_graveyard(&engine, P1, island()).is_some());
    assert!(in_hand(&engine, P1, forest()).is_none());
    assert!(in_hand(&engine, P1, island()).is_none());
}
