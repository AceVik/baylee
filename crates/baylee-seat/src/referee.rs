//! The referee: a mind's answer is held to the question before it is sent.
//!
//! The engine refuses an answer only for a reason the question states in
//! its own fields ([`Pending::answer_fault`]), so an answer that passes here
//! is one the table takes. Checking first turns a wrong answer into a second
//! chance for the mind, told in words why, instead of a refusal from the
//! table and a question asked again over the network.
//!
//! What the engine takes outside the question is taken here the same way,
//! and nothing more:
//!
//! - **a concession**, at any time (CR 104.3a), which the engine applies
//!   before looking at the question;
//! - **a batch of targets** for a waiting series of one trigger, which is the
//!   same choice as the one target question it answers, repeated.
//!
//! A draw offer and the seat's standing orders (priority holds, per-ability
//! yields and answers) are refused. The engine takes them, but none of them
//! answers the question, so the mind would be asked again; the orders are
//! the bridge's (the standing orders in [`crate::wake`]), and a draw offer
//! is not something this stage lets a mind make.

use baylee_engine::choice::{AnswerFault, Pending, PlayerAction};

/// Why the referee would not send an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foul {
    /// The answer breaks a constraint the question states.
    Fault(AnswerFault),
    /// The answer is an action that answers no question.
    NotAnAnswer(&'static str),
}

impl Foul {
    /// The foul in words, for the mind and the transcript.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Fault(fault) => fault.reason(),
            Self::NotAnAnswer(why) => why,
        }
    }
}

/// Holds `answer` to `pending`: `Ok` when the table would take it.
///
/// # Errors
/// The [`Foul`] that keeps it from being sent.
pub fn check(pending: &Pending, answer: &PlayerAction) -> Result<(), Foul> {
    if matches!(pending, Pending::GameOver(_)) {
        return Err(Foul::NotAnAnswer("the game is over"));
    }
    match answer {
        PlayerAction::Concede => Ok(()),
        PlayerAction::OfferDraw => Err(Foul::NotAnAnswer(
            "a draw offer answers no question, and this bridge makes none",
        )),
        PlayerAction::SetPriorityHold(_)
        | PlayerAction::SetAbilityYield { .. }
        | PlayerAction::SetAbilityPolicy { .. } => Err(Foul::NotAnAnswer(
            "the bridge keeps this seat's standing orders; a mind answers the question",
        )),
        PlayerAction::ChooseTargetBatch {
            objects,
            players,
            count,
        } => {
            if *count == 0 {
                return Err(Foul::NotAnAnswer("a batch of targets for no trigger"));
            }
            let one = PlayerAction::ChooseTargets {
                objects: objects.clone(),
                players: players.clone(),
            };
            pending
                .answer_fault(&one)
                .map_or(Ok(()), |fault| Err(Foul::Fault(fault)))
        }
        _ => pending
            .answer_fault(answer)
            .map_or(Ok(()), |fault| Err(Foul::Fault(fault))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{AbilityRef, CardIndex, Defender, ObjectId, PlayerId, SubtypeId};
    use baylee_core::mana::{ManaColor, ManaCost};
    use baylee_engine::choice::{
        ArrangePile, ArrangePlace, ArrangePrompt, BlockOption, CastModeDesc, CastModeKind,
        ChoicePrompt, LegalActions, NumberPrompt, PriorityHold, TargetPrompt, YesNoPrompt,
    };
    use baylee_engine::win::{EndReason, GameResult};

    const ME: PlayerId = PlayerId::new(0);
    const THEM: PlayerId = PlayerId::new(1);

    const fn o(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    /// One question of every kind, each with an answer the table takes.
    fn questions() -> Vec<(Pending, PlayerAction)> {
        let mut questions = turn_questions();
        questions.extend(pick_questions());
        questions.extend(choice_questions());
        questions
    }

    /// The questions of a turn's frame: the opening hand, priority, combat.
    fn turn_questions() -> Vec<(Pending, PlayerAction)> {
        let legal = LegalActions {
            can_pass: true,
            lands: vec![o(1)],
            castable: vec![o(2)],
            ..LegalActions::default()
        };
        vec![
            (
                Pending::Mulligan {
                    player: ME,
                    taken: 0,
                    next_is_free: true,
                    can_take: true,
                },
                PlayerAction::MulliganTake,
            ),
            (
                Pending::MulliganBottom {
                    player: ME,
                    count: 2,
                },
                PlayerAction::ChooseObjects {
                    objects: vec![o(1), o(2)],
                },
            ),
            (
                Pending::Priority {
                    player: ME,
                    legal: Box::new(legal),
                },
                PlayerAction::CastSpell { card: o(2) },
            ),
            (
                Pending::ChooseAttackers {
                    player: ME,
                    attackers: vec![o(5)],
                    defenders: vec![Defender::Player(THEM)],
                },
                PlayerAction::DeclareAttackers {
                    attackers: vec![(o(5), Defender::Player(THEM))],
                },
            ),
            (
                Pending::ChooseBlockers {
                    player: ME,
                    attacker: THEM,
                    blockers: vec![BlockOption {
                        blocker: o(6),
                        attackers: vec![o(9)],
                    }],
                    bounds: Vec::new(),
                },
                PlayerAction::DeclareBlockers {
                    blockers: vec![(o(6), o(9))],
                },
            ),
        ]
    }

    /// The questions that pick objects, players or names.
    fn pick_questions() -> Vec<(Pending, PlayerAction)> {
        vec![
            (
                Pending::DiscardChoice {
                    player: ME,
                    count: 1,
                },
                PlayerAction::ChooseObjects {
                    objects: vec![o(3)],
                },
            ),
            (
                Pending::LegendChoice {
                    player: ME,
                    options: vec![o(7), o(8)],
                },
                PlayerAction::ChooseObjects {
                    objects: vec![o(8)],
                },
            ),
            (
                Pending::ChooseCards {
                    player: ME,
                    options: vec![o(1), o(2), o(3)],
                    min: 0,
                    max: 2,
                    prompt: ChoicePrompt::SearchLibrary,
                    total: None,
                },
                PlayerAction::ChooseObjects {
                    objects: vec![o(1), o(3)],
                },
            ),
            (
                Pending::ChooseTargets {
                    player: ME,
                    options: vec![o(4)],
                    player_options: vec![THEM],
                    min: 1,
                    max: 1,
                    reason: TargetPrompt::Targets,
                },
                PlayerAction::ChooseTargets {
                    objects: Vec::new(),
                    players: vec![THEM],
                },
            ),
            (
                Pending::ChooseSubtype {
                    player: ME,
                    options: vec![SubtypeId::new(3)],
                },
                PlayerAction::ChooseSubtype(SubtypeId::new(3)),
            ),
            (
                Pending::ChooseCardName { player: ME },
                PlayerAction::ChooseCardName {
                    card: CardIndex::new(1),
                    face: 0,
                },
            ),
        ]
    }

    /// The questions an effect or a cost asks on the way.
    fn choice_questions() -> Vec<(Pending, PlayerAction)> {
        let mode = |index, kind| CastModeDesc {
            index,
            kind,
            cost: ManaCost::default(),
        };
        vec![
            (
                Pending::ChooseColor {
                    player: ME,
                    options: vec![ManaColor::Red, ManaColor::Green],
                },
                PlayerAction::ChooseColor(ManaColor::Green),
            ),
            (
                Pending::YesNo {
                    player: ME,
                    prompt: YesNoPrompt::Kicker,
                    source: None,
                },
                PlayerAction::YesNo(true),
            ),
            (
                Pending::ChooseCastMode {
                    player: ME,
                    object: o(2),
                    options: vec![mode(0, CastModeKind::Normal), mode(1, CastModeKind::Kicked)],
                },
                PlayerAction::ChooseMode(1),
            ),
            (
                Pending::ChooseNumber {
                    player: ME,
                    min: 0,
                    max: 5,
                    reason: NumberPrompt::X,
                },
                PlayerAction::ChooseNumber(5),
            ),
            (
                Pending::ChoosePlayer {
                    player: ME,
                    options: vec![ME, THEM],
                },
                PlayerAction::ChoosePlayer(THEM),
            ),
            (
                Pending::Arrange {
                    player: ME,
                    cards: vec![o(1), o(2)],
                    piles: vec![
                        ArrangePile::up_to(ArrangePlace::LibraryTop, 2),
                        ArrangePile::up_to(ArrangePlace::LibraryBottom, 2),
                    ],
                    prompt: ArrangePrompt::Scry,
                },
                PlayerAction::Arrange {
                    piles: vec![vec![o(2)], vec![o(1)]],
                },
            ),
            (
                Pending::ChoosePile {
                    player: ME,
                    piles: vec![vec![o(1)], vec![o(2), o(3)]],
                },
                PlayerAction::ChooseMode(1),
            ),
        ]
    }

    const fn over() -> Pending {
        Pending::GameOver(GameResult {
            winner: None,
            reason: EndReason::Draw,
        })
    }

    /// Which kind of question it is, by an exhaustive match: a new kind of
    /// question fails to compile here until these tests hold it too.
    const fn slot(pending: &Pending) -> usize {
        match pending {
            Pending::Mulligan { .. } => 0,
            Pending::MulliganBottom { .. } => 1,
            Pending::Priority { .. } => 2,
            Pending::ChooseAttackers { .. } => 3,
            Pending::ChooseBlockers { .. } => 4,
            Pending::DiscardChoice { .. } => 5,
            Pending::LegendChoice { .. } => 6,
            Pending::ChooseCards { .. } => 7,
            Pending::ChooseTargets { .. } => 8,
            Pending::ChooseSubtype { .. } => 9,
            Pending::ChooseCardName { .. } => 10,
            Pending::ChooseColor { .. } => 11,
            Pending::YesNo { .. } => 12,
            Pending::ChooseCastMode { .. } => 13,
            Pending::ChooseNumber { .. } => 14,
            Pending::ChoosePlayer { .. } => 15,
            Pending::Arrange { .. } => 16,
            Pending::ChoosePile { .. } => 17,
            Pending::GameOver(_) => 18,
        }
    }

    fn question(kind: usize) -> Pending {
        questions()
            .into_iter()
            .find(|(q, _)| slot(q) == kind)
            .map_or_else(over, |(q, _)| q)
    }

    #[test]
    fn the_questions_here_are_one_of_every_kind() {
        let mut slots: Vec<usize> = questions().iter().map(|(q, _)| slot(q)).collect();
        slots.push(slot(&over()));
        slots.sort_unstable();
        assert_eq!(slots, (0..=18).collect::<Vec<_>>());
    }

    /// Every question takes its right answer, and a concession, which the
    /// engine takes whatever is asked (CR 104.3a).
    #[test]
    fn every_question_takes_its_answer_and_a_concession() {
        for (question, answer) in questions() {
            assert_eq!(
                check(&question, &answer),
                Ok(()),
                "{question:?} / {answer:?}"
            );
            assert_eq!(
                check(&question, &PlayerAction::Concede),
                Ok(()),
                "{question:?}"
            );
        }
    }

    /// An answer of another kind is refused before it is sent, for every
    /// kind of question.
    #[test]
    fn every_question_refuses_an_answer_of_another_kind() {
        for (question, _) in questions() {
            let wrong = if matches!(question, Pending::YesNo { .. }) {
                PlayerAction::ChooseNumber(0)
            } else {
                PlayerAction::YesNo(true)
            };
            assert!(
                matches!(check(&question, &wrong), Err(Foul::Fault(_))),
                "{question:?} took {wrong:?}"
            );
        }
    }

    /// A draw offer and the seat's standing orders are not answers, for any
    /// question: the engine would take them and ask again.
    #[test]
    fn a_draw_offer_and_a_setting_are_not_answers() {
        let ability = AbilityRef::new(CardIndex::new(1), 0);
        let settings = [
            PlayerAction::OfferDraw,
            PlayerAction::SetPriorityHold(PriorityHold::default()),
            PlayerAction::SetAbilityYield {
                ability,
                enabled: true,
            },
            PlayerAction::SetAbilityPolicy {
                ability,
                pass: true,
                answer: None,
            },
        ];
        for (question, _) in questions() {
            for setting in &settings {
                assert!(
                    matches!(check(&question, setting), Err(Foul::NotAnAnswer(_))),
                    "{question:?} took {setting:?}"
                );
            }
        }
    }

    /// Nothing answers a game that is over, not even a concession.
    #[test]
    fn nothing_answers_a_game_that_is_over() {
        for answer in [
            PlayerAction::Concede,
            PlayerAction::PassPriority,
            PlayerAction::MulliganKeep,
        ] {
            assert!(matches!(check(&over(), &answer), Err(Foul::NotAnAnswer(_))));
        }
    }

    /// The question's own bounds hold: what was not offered, too many, too
    /// few, out of range, repeated.
    #[test]
    fn an_answer_outside_the_questions_bounds_is_refused() {
        let fouls = [
            (
                1,
                PlayerAction::ChooseObjects {
                    objects: vec![o(1)],
                },
            ),
            (2, PlayerAction::CastSpell { card: o(9) }),
            (
                3,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(o(6), Defender::Player(THEM))],
                },
            ),
            (
                4,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(o(6), o(10))],
                },
            ),
            (
                6,
                PlayerAction::ChooseObjects {
                    objects: vec![o(7), o(8)],
                },
            ),
            (
                7,
                PlayerAction::ChooseObjects {
                    objects: vec![o(1), o(1)],
                },
            ),
            (
                8,
                PlayerAction::ChooseTargets {
                    objects: vec![o(5)],
                    players: Vec::new(),
                },
            ),
            (9, PlayerAction::ChooseSubtype(SubtypeId::new(4))),
            (11, PlayerAction::ChooseColor(ManaColor::Blue)),
            (13, PlayerAction::ChooseMode(2)),
            (14, PlayerAction::ChooseNumber(6)),
            (15, PlayerAction::ChoosePlayer(PlayerId::new(2))),
            (
                16,
                PlayerAction::Arrange {
                    piles: vec![vec![o(2)], Vec::new()],
                },
            ),
            (17, PlayerAction::ChooseMode(2)),
        ];
        for (kind, answer) in fouls {
            let question = question(kind);
            assert!(
                matches!(check(&question, &answer), Err(Foul::Fault(_))),
                "{question:?} took {answer:?}"
            );
        }
    }

    /// A batch of targets for a waiting series of one trigger is the target
    /// question's answer, repeated: held to that question, and a batch for
    /// no trigger is refused.
    #[test]
    fn a_batch_of_targets_is_held_to_the_target_question() {
        let question = question(8);
        let batch = |players: Vec<PlayerId>, count| PlayerAction::ChooseTargetBatch {
            objects: Vec::new(),
            players,
            count,
        };
        assert_eq!(check(&question, &batch(vec![THEM], 3)), Ok(()));
        assert!(matches!(
            check(&question, &batch(vec![ME], 3)),
            Err(Foul::Fault(_))
        ));
        assert!(matches!(
            check(&question, &batch(vec![THEM], 0)),
            Err(Foul::NotAnAnswer(_))
        ));
    }
}
