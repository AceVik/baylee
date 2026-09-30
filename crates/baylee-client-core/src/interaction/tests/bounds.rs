//! Whether an answer may be given at all, how many things it may hold, and when it may be sent: a question addressed to another seat, `min`, `max`, `SelectionOutcome::Full`, `can_confirm`, and the empty answer that stands in for a cancel the wire does not carry. A discard belongs here because its bounds are a bare count with no offered list behind them. What a pick *is* - an object or a seat, one member of a stack, taken back one at a time - is in `picks`, and combat counts its own declarations rather than this list.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_choice_addressed_to_another_seat_is_not_actionable() {
    let mut i = interaction(Pending::ChooseTargets {
        player: PlayerId::new(1),
        options: vec![obj(1)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert!(!i.is_mine());
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Rejected);
    assert!(i.confirm().is_none());
    assert!(matches!(i.prompt(), Prompt::Waiting { on: Some(_) }));
}

#[test]
fn the_maximum_is_enforced_and_toggling_off_frees_a_slot() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2), obj(3)],
        min: 1,
        max: 2,
        prompt: ChoicePrompt::Generic,
        total: None,
    });
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Full);
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Removed);
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Added);
}

#[test]
fn a_minimum_blocks_confirmation_until_it_is_met() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2)],
        min: 2,
        max: 2,
        prompt: ChoicePrompt::Generic,
        total: None,
    });
    assert!(!i.can_confirm());
    i.toggle(obj(1));
    assert!(!i.can_confirm());
    i.toggle(obj(2));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1), obj(2)]
        })
    );
}

#[test]
fn an_up_to_choice_can_be_confirmed_with_nothing_selected() {
    let i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 0,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] })
    );
}

#[test]
fn discarding_operates_on_the_hand_which_the_engine_leaves_implicit() {
    let mut i = interaction(Pending::DiscardChoice {
        player: me(),
        count: 2,
    });
    // No enumerated options, so any card in hand is fair game.
    assert!(i.selectable().is_empty());
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
    assert_eq!(i.toggle(obj(3)), SelectionOutcome::Full);
    assert!(i.can_confirm());
}

/// Cancel is an *empty* answer, and only a question that will take one
/// has it to offer.
///
/// The zone browser's footer rests entirely on this: there is no cancel
/// action on the wire, so the way out of a "you may search" is to send
/// the empty set, and a question with a minimum above zero has no way out
/// at all. Both halves are asserted, and so is the order — [`cancel`]
/// first and [`confirm`] after, because a player who ticked a card and
/// then changed their mind must not have that card sent under the word
/// "Cancel".
///
/// [`cancel`]: Interaction::cancel
/// [`confirm`]: Interaction::confirm
#[test]
fn a_question_that_takes_nothing_is_answered_with_nothing() {
    let search = |min: u8| {
        Interaction::new(
            Pending::ChooseCards {
                player: me(),
                options: vec![obj(1), obj(2)],
                min,
                max: 2,
                prompt: ChoicePrompt::SearchLibrary,
                total: None,
            },
            me(),
        )
    };

    let may = search(0);
    assert_eq!(may.bounds(), Some((0, 2)));
    assert_eq!(
        may.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] }),
        "an empty answer to a may-search is not an answer at all"
    );

    let must = search(1);
    assert_eq!(must.bounds(), Some((1, 2)));
    assert!(
        must.confirm().is_none(),
        "a question with a minimum has a way out it cannot deliver"
    );

    // And the order: a pick taken back before the send.
    let mut mind_changed = search(0);
    mind_changed.toggle(obj(1));
    assert_eq!(
        mind_changed.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        })
    );
    mind_changed.cancel();
    assert_eq!(
        mind_changed.confirm(),
        Some(PlayerAction::ChooseObjects { objects: vec![] }),
        "Cancel sent the card the player had just decided against"
    );
}

/// A crew short of the total power the question states (CR 702.122a) is
/// complete by its count and still not sent: the question names the fault,
/// Confirm stays unlit, and two Elves' worth of power lights it.
#[test]
fn a_crew_short_of_its_stated_total_is_held_back() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2), obj(3)],
        min: 1,
        max: 3,
        prompt: ChoicePrompt::CostCrew { power: 2 },
        total: Some(baylee_engine::choice::CardTotal {
            of: baylee_engine::choice::Measure::Power,
            weights: vec![1, 1, 2],
            at_least: Some(2),
            at_most: None,
        }),
    });
    i.toggle(obj(1));
    assert_eq!(
        i.answer_fault(),
        Some(baylee_engine::choice::AnswerFault::TotalTooLow)
    );
    assert!(!i.can_confirm(), "one power-1 creature does not crew 2");
    assert_eq!(i.confirm(), None);
    i.toggle(obj(2));
    assert_eq!(i.answer_fault(), None);
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1), obj(2)]
        })
    );
}

/// One creature blocking an attacker the question bounds to two or more
/// (menace, CR 702.111b) is not sent; a second one makes it an answer.
#[test]
fn a_lone_blocker_on_a_bounded_attacker_is_held_back() {
    let mut pending = block_choice(vec![
        BlockOption {
            blocker: obj(1),
            attackers: vec![obj(9)],
        },
        BlockOption {
            blocker: obj(2),
            attackers: vec![obj(9)],
        },
    ]);
    if let Pending::ChooseBlockers { bounds, .. } = &mut pending {
        bounds.push(baylee_engine::choice::AttackerBound {
            attacker: obj(9),
            min_blockers: 2,
            max_blockers: u32::MAX,
        });
    }
    let mut i = interaction(pending);
    assert!(i.can_confirm(), "blocking nothing is an answer");
    assert!(i.declare_blocker(obj(1), obj(9)));
    assert_eq!(
        i.answer_fault(),
        Some(baylee_engine::choice::AnswerFault::TooFewBlockers)
    );
    assert!(!i.can_confirm());
    assert_eq!(i.confirm(), None);
    assert!(i.declare_blocker(obj(2), obj(9)));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareBlockers {
            blockers: vec![(obj(1), obj(9)), (obj(2), obj(9))]
        })
    );
}

/// Once a hand would open with zero cards the question takes no further
/// mulligan (CR 103.5), and the interaction does not build one.
#[test]
fn no_mulligan_is_built_past_the_last_the_question_allows() {
    let i = interaction(Pending::Mulligan {
        player: me(),
        taken: 8,
        next_is_free: false,
        can_take: false,
    });
    assert_eq!(i.answer_mulligan(false), None);
    assert_eq!(i.answer_mulligan(true), Some(PlayerAction::MulliganKeep));
}
