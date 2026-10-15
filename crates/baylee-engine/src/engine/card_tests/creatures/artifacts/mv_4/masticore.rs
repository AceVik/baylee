//! `cards/creatures/artifacts/mv_4/masticore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

fn masticore() -> CardIndex {
    card_index("621476a4-44bf-4f55-adb1-880cdd213f38")
}

/// Passes priority until something that is not priority is asked, and
/// returns it.
#[track_caller]
fn next_question(engine: &mut Engine<RegistryLookup>) -> Pending {
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => return other,
        }
    }
    panic!("no question was ever asked");
}

/// Answers the upkeep question by discarding the card on offer.
#[track_caller]
fn discard_the_offered_card(engine: &mut Engine<RegistryLookup>) {
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the upkeep question stands: {:?}", engine.pending());
    };
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card on offer is a legal discard");
}

/// Answers the upkeep question by discarding nothing.
#[track_caller]
fn discard_nothing(engine: &mut Engine<RegistryLookup>) {
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseObjects { objects: vec![] },
        )
        .expect("discarding nothing is a legal answer: the cost is optional");
}

/// Seat 0's Masticore in its first upkeep, with `hand` in hand: the question
/// "discard a card, or sacrifice" stands unanswered.
#[track_caller]
fn at_the_upkeep_question(hand: &[CardIndex], board: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut seated = vec![masticore()];
    seated.extend_from_slice(board);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &seated)
        .battlefield(1, &[grizzly_bears()])
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    let question = next_question(&mut engine);
    assert!(
        matches!(
            question,
            Pending::ChooseCards {
                prompt: ChoicePrompt::CostDiscard,
                ..
            }
        ),
        "the upkeep asks whether to discard: {question:?}"
    );
    engine
}

/// Masticore — {4} 4/4: "At the beginning of your upkeep, sacrifice this
/// creature unless you discard a card." Paying with a Grizzly Bears from
/// hand keeps it and puts the Bears in the graveyard.
#[test]
fn masticore_stays_when_a_card_is_discarded_at_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = at_the_upkeep_question(&[grizzly_bears()], &[]);
    discard_the_offered_card(&mut engine);
    answer_until_settled(&mut engine);
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, masticore()).is_some());
    assert!(in_graveyard(&engine, p0, masticore()).is_none());
}

/// Declining the discard sacrifices it, and the Bears stay in hand.
#[test]
fn masticore_is_sacrificed_when_the_discard_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = at_the_upkeep_question(&[grizzly_bears()], &[]);
    discard_nothing(&mut engine);
    answer_until_settled(&mut engine);
    assert!(in_graveyard(&engine, p0, masticore()).is_some());
    assert!(on_battlefield(&engine, p0, masticore()).is_none());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some(), "kept");
}

/// "{2}: This creature deals 1 damage to target creature." Twice at a
/// Grizzly Bears: one damage marked, then dead.
#[test]
fn masticore_pings_a_creature_for_two_each() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_the_upkeep_question(&[grizzly_bears()], &[forest(); 4]);
    discard_the_offered_card(&mut engine);
    answer_until_settled(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
    });
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    tap_all_mana(&mut engine, p0);
    for expected in [1, 2] {
        activate(&mut engine, p0, masticore(), 2);
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("the ability targets: {:?}", engine.pending());
        };
        assert!(options.contains(&theirs));
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![theirs],
                    players: vec![],
                },
            )
            .expect("the Bears are a legal target");
        pass_until(&mut engine, stack_is_empty);
        if expected == 1 {
            assert_eq!(engine.state().object(theirs).expect("bears").damage, 1);
        }
    }
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "2 damage"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{2}} twice"
    );
}

/// "{2}: Regenerate this creature." Bought, a destroy leaves it on the
/// battlefield, tapped.
#[test]
fn masticore_regenerates() {
    let p0 = PlayerId::new(0);
    let mut engine = at_the_upkeep_question(&[grizzly_bears()], &[forest(), forest()]);
    discard_the_offered_card(&mut engine);
    answer_until_settled(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
    });
    let beast = on_battlefield(&engine, p0, masticore()).expect("seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, beast, 1);
    kill(&mut engine, beast);
    assert!(on_battlefield(&engine, p0, masticore()).is_some());
    assert!(is_tapped(&engine, beast));
}
