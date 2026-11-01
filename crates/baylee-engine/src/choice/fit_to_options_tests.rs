//! Fitting a question to its options: what `Pending::fit_to_options` shrinks, and what it leaves.

use super::*;

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn card(n: u32) -> ObjectId {
    ObjectId::new(n, 0)
}

/// A choice of cards is held to the cards it offers (CR 609.3), and a
/// target choice is not: shrinking it would cast a spell with fewer
/// targets than it requires (CR 601.2c).
#[test]
fn a_choice_of_cards_is_fitted_and_a_short_target_choice_is_not() {
    let mut cards = Pending::ChooseCards {
        player: me(),
        options: vec![card(1)],
        min: 2,
        max: 2,
        prompt: ChoicePrompt::Generic,
        total: None,
    };
    assert!(cards.fit_to_options());
    let Pending::ChooseCards { min, max, .. } = cards else {
        unreachable!()
    };
    assert_eq!((min, max), (1, 1));

    let mut targets = Pending::ChooseTargets {
        player: me(),
        options: vec![card(1)],
        player_options: vec![],
        min: 2,
        max: 2,
        reason: TargetPrompt::Targets,
    };
    assert!(!targets.fit_to_options());
    let Pending::ChooseTargets { min, .. } = targets else {
        unreachable!()
    };
    assert_eq!(min, 2, "a target choice keeps what it requires");

    let mut nobody = Pending::ChoosePlayer {
        player: me(),
        options: vec![],
    };
    assert!(!nobody.fit_to_options(), "an empty menu has no answer");
}

fn crew(weights: Vec<i32>, at_least: i32) -> Pending {
    let options = (1..=u32::try_from(weights.len()).unwrap())
        .map(card)
        .collect::<Vec<_>>();
    Pending::ChooseCards {
        player: me(),
        max: u8::try_from(options.len()).unwrap(),
        options,
        min: 1,
        prompt: ChoicePrompt::CostCrew { power: 2 },
        total: Some(CardTotal {
            of: Measure::Power,
            weights,
            at_least: Some(at_least),
            at_most: None,
        }),
    }
}

fn choose(ids: &[u32]) -> PlayerAction {
    PlayerAction::ChooseObjects {
        objects: ids.iter().copied().map(card).collect(),
    }
}

/// A stated total is a promise that some answer keeps it: a question
/// whose weights cannot reach it, or whose weights do not line up with
/// its options, is one nothing can answer.
#[test]
fn a_total_no_choice_can_reach_is_unanswerable() {
    assert!(crew(vec![1, 1], 2).fit_to_options());
    assert!(
        !crew(vec![1, 0], 2).fit_to_options(),
        "power 1 cannot crew 2"
    );
    let mut misaligned = crew(vec![1, 1], 2);
    if let Pending::ChooseCards { options, .. } = &mut misaligned {
        options.push(card(9));
    }
    assert!(
        !misaligned.fit_to_options(),
        "a weight for every option, in order"
    );
}

/// The total is summed as the engine sums it, a negative power below
/// zero (CR 107.1b), and the question's other bounds come first.
#[test]
fn a_total_counts_negative_weights_and_names_its_own_fault() {
    let question = crew(vec![2, -1, 1], 2);
    assert_eq!(question.answer_fault(&choose(&[1])), None);
    assert_eq!(
        question.answer_fault(&choose(&[1, 2])),
        Some(AnswerFault::TotalTooLow),
        "2 + (-1) is 1, short of 2"
    );
    assert_eq!(question.answer_fault(&choose(&[1, 2, 3])), None);
    assert_eq!(
        question.answer_fault(&choose(&[3, 3])),
        Some(AnswerFault::Repeated)
    );
    assert_eq!(
        question.answer_fault(&choose(&[4])),
        Some(AnswerFault::NotOffered)
    );
    assert_eq!(
        question.answer_fault(&choose(&[])),
        Some(AnswerFault::TooFew)
    );
    assert_eq!(
        question.answer_fault(&PlayerAction::YesNo(true)),
        Some(AnswerFault::WrongKind)
    );
    let mut capped = crew(vec![2, 1, 1], 0);
    if let Pending::ChooseCards {
        total: Some(total), ..
    } = &mut capped
    {
        total.at_least = None;
        total.at_most = Some(2);
    }
    assert_eq!(
        capped.answer_fault(&choose(&[1, 2])),
        Some(AnswerFault::TotalTooHigh)
    );
    assert_eq!(capped.answer_fault(&choose(&[2, 3])), None);
}

/// A blocker blocks one attacker (CR 509.1a), and an attacker's bound
/// counts the blockers named against it: none, or `min..=max`.
#[test]
fn a_blockers_answer_is_held_to_its_pairings_and_bounds() {
    let (runner, other) = (card(1), card(2));
    let question = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: me(),
        attacker: PlayerId::new(1),
        blockers: vec![
            BlockOption {
                blocker: card(10),
                attackers: vec![runner, other],
            },
            BlockOption {
                blocker: card(11),
                attackers: vec![runner],
            },
        ],
        bounds: vec![AttackerBound {
            attacker: runner,
            min_blockers: 2,
            max_blockers: u32::MAX,
        }],
        capacity: Vec::new(),
        obeying: Vec::new(),
    };
    let declare = |pairs: &[(u32, ObjectId)]| PlayerAction::DeclareBlockers {
        blockers: pairs.iter().map(|(b, a)| (card(*b), *a)).collect(),
    };
    assert_eq!(question.answer_fault(&declare(&[])), None);
    assert_eq!(question.answer_fault(&declare(&[(10, other)])), None);
    assert_eq!(
        question.answer_fault(&declare(&[(10, runner)])),
        Some(AnswerFault::TooFewBlockers)
    );
    assert_eq!(
        question.answer_fault(&declare(&[(10, runner), (11, runner)])),
        None
    );
    assert_eq!(
        question.answer_fault(&declare(&[(11, other)])),
        Some(AnswerFault::NotOffered)
    );
    assert_eq!(
        question.answer_fault(&declare(&[(10, runner), (10, other)])),
        Some(AnswerFault::OverCapacity),
        "one creature blocks one attacker"
    );
    assert_eq!(
        question.answer_fault(&declare(&[(10, runner), (10, runner)])),
        Some(AnswerFault::Repeated),
        "one pair named twice"
    );
}

/// An attack is held to what its question says of it: a creature in
/// `required` attacks (CR 508.1d), and one in `limits` attacks only
/// what its limit lists (CR 508.1c). The engine refused both and this
/// passed both, so a client lit Confirm for a declaration the engine
/// then refused.
#[test]
fn an_attack_is_held_to_its_requirements_and_limits() {
    use baylee_core::ids::Defender;
    let (jugg, elf) = (card(1), card(2));
    let them = Defender::Player(PlayerId::new(1));
    let walker = Defender::Planeswalker(card(9));
    let question = Pending::ChooseAttackers {
        player: me(),
        attackers: vec![jugg, elf],
        defenders: vec![them, walker],
        required: vec![jugg],
        alone: Vec::new(),
        minimum_required: None,
        limits: vec![AttackLimit {
            creature: elf,
            defenders: vec![them],
        }],
    };
    let declare = |pairs: &[(ObjectId, Defender)]| PlayerAction::DeclareAttackers {
        attackers: pairs.to_vec(),
    };
    assert_eq!(
        question.answer_fault(&declare(&[])),
        Some(AnswerFault::MustAttack),
        "the required attacker is missing"
    );
    assert_eq!(
        question.answer_fault(&declare(&[(elf, them)])),
        Some(AnswerFault::MustAttack)
    );
    assert_eq!(question.answer_fault(&declare(&[(jugg, walker)])), None);
    assert_eq!(
        question.answer_fault(&declare(&[(jugg, them), (elf, walker)])),
        Some(AnswerFault::NotOffered),
        "the Elf may attack only the player"
    );
    assert_eq!(
        question.answer_fault(&declare(&[(jugg, them), (elf, them)])),
        None
    );
}

/// A block is held to as many requirements as `obeying` obeys
/// (CR 509.1c), each pair weighed by `demands`: with two lured
/// attackers and one blocker, blocking either obeys one, as `obeying`
/// does, and blocking the third, or nothing, obeys none.
#[test]
fn a_block_is_held_to_as_many_requirements_as_obeying() {
    let (lured, also, plain, blocker) = (card(1), card(2), card(3), card(10));
    let question = Pending::ChooseBlockers {
        player: me(),
        attacker: PlayerId::new(1),
        blockers: vec![BlockOption {
            blocker,
            attackers: vec![lured, also, plain],
        }],
        capacity: Vec::new(),
        obeying: vec![(blocker, lured)],
        demands: [lured, also]
            .into_iter()
            .map(|attacker| BlockDemand {
                blocker,
                attacker,
                count: 1,
            })
            .collect(),
        bounds: Vec::new(),
    };
    let declare = |pairs: &[(ObjectId, ObjectId)]| PlayerAction::DeclareBlockers {
        blockers: pairs.to_vec(),
    };
    for (pairs, fault) in [
        (vec![], Some(AnswerFault::MustBlock)),
        (vec![(blocker, plain)], Some(AnswerFault::MustBlock)),
        (vec![(blocker, also)], None),
        (vec![(blocker, lured)], None),
    ] {
        assert_eq!(question.answer_fault(&declare(&pairs)), fault, "{pairs:?}");
    }
}
