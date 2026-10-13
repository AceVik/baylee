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
