//! The house does not retry an action the engine took back (owner,
//! 10.11.2026).
//!
//! A cast or activation the engine cannot finish (the X announced leaves
//! no legal target, a cost cannot be paid) is taken back: the game is where
//! it was before the action. A house asked again from there gives the same
//! answer, being deterministic, and the game goes round until a cap ends
//! it. So an agent remembers the last priority action it gave, the
//! position it gave it from and the question it answered (the view's
//! `seq`). Asked a new question from the same position, it would give the
//! same action again only because the engine took it back, and it passes
//! instead. The same question asked again (a host that asks twice, a
//! question resent after a reconnect) is answered as before.
//!
//! It is a loop guard, not a strength change. A position the house has
//! acted from does not come back once the action stands (the action moves a
//! card, taps a permanent or spends mana), so the guard acts only where the
//! game would otherwise never end, and at every level, the frozen reference
//! included (`docs/house-ai.md` §"The frozen reference").

use crate::HeuristicAgent;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, PoisonError};

/// The last priority action an agent gave, the position it gave it from
/// and the question it answered, shared by the agent's clones. A seat that
/// builds an agent for each answer keeps one of these for the game
/// (`baylee_seat::HouseMind`).
#[derive(Clone, Debug, Default)]
pub struct StallMemory(Arc<Mutex<Option<Given>>>);

/// One priority action, as [`StallMemory`] keeps it.
#[derive(Debug)]
struct Given {
    /// [`position`] of the view it was given from.
    position: u64,
    /// The view's question number.
    seq: u64,
    action: PlayerAction,
}

/// The position `view` shows, as one number: everything a cast or an
/// activation changes when it stands, and nothing that changes between two
/// questions from the same position (the question's sequence number, the
/// clocks, what standing policies answered).
pub(crate) fn position(view: &PlayerView) -> u64 {
    let seats: Vec<_> = view
        .seats
        .iter()
        .map(|seat| {
            (
                seat.player,
                seat.life,
                seat.poison,
                seat.energy,
                seat.hand_count,
                seat.library_count,
                seat.graveyard_count,
                &seat.mana_pool,
            )
        })
        .collect();
    let bytes = baylee_core::wire::encode(&(
        (view.turn, view.phase, view.step, view.active, view.seat),
        (view.priority_held, view.casting, view.announced_cast),
        &seats,
        &view.hand,
        &view.battlefield,
        &view.stack,
        &view.graveyards,
        &view.exile,
        &view.command,
        &view.combat,
        &view.owed,
    ));
    let mut hasher = std::hash::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

impl HeuristicAgent {
    /// `action`, the answer to `pending`, unless it is a priority action
    /// this agent gave to an earlier question from the same position: then
    /// the engine took that one back, and the agent passes rather than give
    /// it again.
    pub(crate) fn unstalled(
        &self,
        view: &PlayerView,
        pending: &Pending,
        action: PlayerAction,
    ) -> PlayerAction {
        if !matches!(pending, Pending::Priority { .. })
            || matches!(action, PlayerAction::PassPriority)
        {
            return action;
        }
        let here = position(view);
        let mut last = self.stall.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(given) = last.as_ref()
            && given.position == here
            && given.action == action
        {
            if given.seq == view.seq {
                // The same question again: the same answer.
                return action;
            }
            *last = None;
            return PlayerAction::PassPriority;
        }
        *last = Some(Given {
            position: here,
            seq: view.seq,
            action: action.clone(),
        });
        action
    }

    /// This agent remembering its last priority action in `memory`, shared
    /// with whoever else holds it.
    #[must_use]
    pub fn with_stall_memory(mut self, memory: StallMemory) -> Self {
        self.stall = memory;
        self
    }
}
