//! "Keep all remaining" on a copy's new-targets questions (report 10).
//!
//! A copy of a spell with ten targets asks, target by target, whether to
//! change each (CR 707.10c), and moving one onto an object another target
//! already holds makes that one a question that must be answered with a new
//! target. Correct, and ten questions long. This is the shortcut a player
//! approves on one of them: keep this target and every later one of the same
//! run, answering each with the empty answer the engine's own prompt offers
//! (`min` 0 keeps the target). A question that may not be kept (`min` above
//! 0) is never answered here: it waits for the player, and the run goes on
//! keeping after it. Nothing here decides a rule; it answers only what the
//! `Pending` says may be answered so.

use baylee_engine::choice::{Pending, PlayerAction, TargetPrompt};
use baylee_view::PlayerView;

/// One approved run of new-target questions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeepTargets {
    turn: u32,
    of: u32,
    next: u32,
    sent_at: Option<u64>,
}

/// The run a `Pending` is part of: `(index, of, min)` of a new-target
/// question this seat answers.
fn retarget(view: &PlayerView, pending: &Pending) -> Option<(u32, u32, u32)> {
    match pending {
        Pending::ChooseTargets {
            player,
            min,
            reason: TargetPrompt::Retarget { index, of, .. },
            ..
        } if *player == view.seat => Some((*index, *of, *min)),
        _ => None,
    }
}

impl KeepTargets {
    /// Whether "keep all remaining" may be offered on this question: one of
    /// this seat's new-target questions that may be kept, with more after it.
    #[must_use]
    pub fn offered(view: &PlayerView, pending: &Pending) -> Option<u32> {
        let (index, of, min) = retarget(view, pending)?;
        (min == 0 && index + 1 < of).then(|| of - index)
    }

    /// Approves this question and the rest of its run.
    #[must_use]
    pub fn begin(view: &PlayerView, pending: &Pending) -> Option<Self> {
        Self::offered(view, pending)?;
        let (index, of, _) = retarget(view, pending)?;
        Some(Self {
            turn: view.turn,
            of,
            next: index,
            sent_at: None,
        })
    }

    /// Whether this question still belongs to the approved run: the same
    /// turn and size, at or after the target it was approved on. A question
    /// of the run that may not be kept belongs to it too; it is only not
    /// answered here.
    #[must_use]
    pub fn accepts(&self, view: &PlayerView, pending: &Pending) -> bool {
        retarget(view, pending).is_some_and(|(index, of, _)| {
            view.turn == self.turn
                && of == self.of
                && (index >= self.next
                    || (self.sent_at == Some(view.seq) && index.saturating_add(1) == self.next))
        })
    }

    /// The empty answer that keeps this target, once per question, and
    /// nothing for a question the player has to answer.
    pub fn answer(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        if self.sent_at == Some(view.seq) || !self.accepts(view, pending) {
            return None;
        }
        let (index, _, min) = retarget(view, pending)?;
        if min > 0 {
            return None;
        }
        self.sent_at = Some(view.seq);
        self.next = index.saturating_add(1);
        Some(PlayerAction::ChooseTargets {
            objects: Vec::new(),
            players: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;
    use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId, TargetRef};

    fn current() -> TargetRef {
        TargetRef::Object(DamageSourceRef {
            object: ObjectId::new(9, 0),
            version: 1,
        })
    }

    /// A new-target question for `player`: target `index` of `of`.
    fn question(player: u8, min: u32, index: u32, of: u32) -> Pending {
        Pending::ChooseTargets {
            player: PlayerId::new(player),
            options: vec![ObjectId::new(9, 0)],
            player_options: Vec::new(),
            min,
            max: 1,
            reason: TargetPrompt::Retarget {
                current: current(),
                index,
                of,
            },
        }
    }

    fn keep() -> PlayerAction {
        PlayerAction::ChooseTargets {
            objects: Vec::new(),
            players: Vec::new(),
        }
    }

    /// The shortcut is for a question that may be kept with more to come:
    /// offering it on the last target, on a must-change question, on another
    /// seat's question or on a plain target prompt would promise a run that
    /// does not exist or answer for someone else.
    #[test]
    fn keep_all_is_offered_only_where_the_question_may_be_kept_and_more_follow() {
        let view = ViewBuilder::new(2).build();
        assert_eq!(
            KeepTargets::offered(&view, &question(0, 0, 0, 3)),
            Some(3),
            "the first of three may be kept and two follow: three left"
        );
        assert_eq!(
            KeepTargets::offered(&view, &question(0, 0, 1, 3)),
            Some(2),
            "the middle of three: this one and the last"
        );
        assert_eq!(
            KeepTargets::offered(&view, &question(0, 0, 2, 3)),
            None,
            "the last target has nothing after it to keep"
        );
        assert_eq!(
            KeepTargets::offered(&view, &question(0, 0, 0, 1)),
            None,
            "a copy with one target has no run"
        );
        assert_eq!(
            KeepTargets::offered(&view, &question(0, 1, 0, 3)),
            None,
            "a question that must be changed cannot be kept"
        );
        assert_eq!(
            KeepTargets::offered(&view, &question(1, 0, 0, 3)),
            None,
            "another seat's question is not this seat's to shortcut"
        );
        let plain = Pending::ChooseTargets {
            player: view.seat,
            options: vec![ObjectId::new(9, 0)],
            player_options: Vec::new(),
            min: 0,
            max: 1,
            reason: TargetPrompt::Targets,
        };
        assert_eq!(
            KeepTargets::offered(&view, &plain),
            None,
            "an ordinary target choice is not a copy's retargeting"
        );
        assert!(
            KeepTargets::begin(&view, &question(0, 0, 2, 3)).is_none(),
            "begin approves nothing where nothing is offered"
        );
    }

    /// The run keeps 0 and 1, leaves the must-change question at 2 to the
    /// player without ending, and keeps 3 once the player has answered.
    #[test]
    fn a_run_keeps_what_may_be_kept_and_waits_at_what_must_change() {
        let mut view = ViewBuilder::new(2).build();
        let mut run = KeepTargets::begin(&view, &question(0, 0, 0, 4)).expect("offered at 0 of 4");
        assert_eq!(run.answer(&view, &question(0, 0, 0, 4)), Some(keep()));
        view.seq += 1;
        assert_eq!(run.answer(&view, &question(0, 0, 1, 4)), Some(keep()));
        view.seq += 1;
        let must = question(0, 1, 2, 4);
        assert!(
            run.accepts(&view, &must),
            "a must-change question belongs to the run"
        );
        assert_eq!(
            run.answer(&view, &must),
            None,
            "a question that may not be kept is never answered here"
        );
        assert!(
            run.accepts(&view, &must),
            "and the run stays approved while the player answers it"
        );
        view.seq += 1;
        assert_eq!(
            run.answer(&view, &question(0, 0, 3, 4)),
            Some(keep()),
            "the question after the player's own answer is kept again"
        );
    }

    /// One answer per question (`view.seq`): the driver runs every frame and
    /// the same question comes back until the engine has heard the answer.
    #[test]
    fn a_question_is_answered_once_however_often_it_is_asked() {
        let mut view = ViewBuilder::new(2).build();
        let mut run = KeepTargets::begin(&view, &question(0, 0, 0, 3)).unwrap();
        let q = question(0, 0, 0, 3);
        assert_eq!(run.answer(&view, &q), Some(keep()));
        assert_eq!(run.answer(&view, &q), None, "no duplicate send for one seq");
        assert!(
            run.accepts(&view, &q),
            "the question just answered still belongs to the run"
        );
        view.seq += 1;
        assert_eq!(
            run.answer(&view, &question(0, 0, 1, 3)),
            Some(keep()),
            "the next question has its own seq and is answered"
        );
        assert_eq!(
            run.answer(&view, &question(0, 0, 1, 3)),
            None,
            "and only once"
        );
    }

    /// A run belongs to one copy on one turn; a standing instruction that
    /// outlived it would answer a different spell's questions unasked.
    #[test]
    fn a_new_turn_or_another_copy_does_not_continue_the_run() {
        let mut view = ViewBuilder::new(2).build();
        let run = KeepTargets::begin(&view, &question(0, 0, 1, 3)).unwrap();
        assert!(run.accepts(&view, &question(0, 0, 1, 3)));
        assert!(
            !run.accepts(&view, &question(0, 0, 1, 5)),
            "a copy with another number of targets is another run"
        );
        assert!(
            !run.accepts(&view, &question(0, 0, 0, 3)),
            "an earlier target is not part of the run (it never starts over)"
        );
        assert!(
            !run.accepts(&view, &question(1, 0, 1, 3)),
            "another seat's question is not accepted"
        );
        view.turn += 1;
        assert!(
            !run.accepts(&view, &question(0, 0, 1, 3)),
            "a new turn ends the run"
        );
        let mut run = run;
        assert_eq!(
            run.answer(&view, &question(0, 0, 1, 3)),
            None,
            "and answers nothing"
        );
    }
}
