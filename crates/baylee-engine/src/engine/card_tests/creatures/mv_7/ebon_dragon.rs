//! `cards/creatures/mv_7/ebon_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ebon_dragon() -> CardIndex {
    card_index("12e4d2bd-83e9-4120-a2e8-0645c0ed2387")
}

/// Seat 0 with the Dragon in hand and seven Swamps, seat 1 holding one
/// Island, the Dragon cast and its enters trigger aimed at seat 1: the game is
/// stopped at the "you may" the trigger puts to the controller.
#[track_caller]
fn at_the_may() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lots(swamp(), 7))
        .hand(0, &[ebon_dragon()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pay_exactly_and_cast(&mut engine, ebon_dragon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let (options, players) = target_menu(&engine);
    assert!(options.is_empty(), "target opponent: no objects");
    assert_eq!(players, vec![p1], "the one opponent is the only target");
    point_at(&mut engine, p0, &[], &[p1]);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// Ebon Dragon — a 5/4 flier for {5}{B}{B}. "When this creature enters, you
/// may have target opponent discard a card": answered yes, the opponent
/// chooses the card to discard (their only one, the Island) and it goes to
/// their graveyard.
#[test]
fn ebon_dragon_makes_the_opponent_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_the_may();
    engine.apply(p0, PlayerAction::YesNo(true)).expect("yes");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass stopped here")
    };
    assert_eq!(player, p1, "the discarding player chooses");
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: options })
        .expect("discarding their only card");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, island()).is_some());
    let dragon = on_battlefield(&engine, p0, ebon_dragon()).expect("resolved");
    assert_body(&engine, dragon, (5, 4), KeywordSet::FLYING);
}

/// The "may": answered no, nothing is discarded.
#[test]
fn ebon_dragon_may_decline_to_make_them_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_the_may();
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declined");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p1, island()).is_some(), "still in hand");
    assert!(in_graveyard(&engine, p1, island()).is_none());
}
