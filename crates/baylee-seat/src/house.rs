//! The house as a mind: `HeuristicAgent::act`, on the view the house is
//! allowed to see.
//!
//! Two jobs. It is a mind in its own right (`--mind house`), which is how the
//! bridge is measured and tried before any model is attached; and it is the
//! bridge's fallback, the answer given when a mind is too slow, refused
//! twice, or down.
//!
//! Either way it is the house, and the house's rules hold: it never reads
//! the log, and it never reads anything a clock made. A socket view carries
//! both (a player is told the log and their clock), so the view is stripped
//! here exactly as `Session::agent_view` builds one for a house chair: no
//! clock remainder, no policy history, no teammate's hand. The request's
//! [`Request::log`](crate::Request::log) is not read at all.

use crate::mind::{Answer, Disclosure, GameContext, Mind, Request, Thinking};
use baylee_ai::{AIProfile, HeuristicAgent, policy_seed};
use baylee_core::ids::SeatSet;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use std::time::Instant;

/// The house heuristic at one level.
#[derive(Clone, Debug)]
pub struct HouseMind {
    profile: AIProfile,
}

impl Default for HouseMind {
    fn default() -> Self {
        Self::new(AIProfile::default())
    }
}

impl HouseMind {
    /// The house at this profile: its level.
    #[must_use]
    pub const fn new(profile: AIProfile) -> Self {
        Self { profile }
    }

    /// The house at a level the lobby names (`steady`, `sharp`, …); `None`
    /// for a name it does not know.
    #[must_use]
    pub fn named(level: &str) -> Option<Self> {
        AIProfile::named(level).map(Self::new)
    }

    /// The house's answer, synchronously.
    ///
    /// Seeded as the host seeds a house chair at a named table
    /// (`policy_seed(game, seat)`), and told the sides, so a partner is
    /// never taken for an enemy.
    #[must_use]
    pub fn answer(
        &self,
        context: &GameContext,
        view: &PlayerView,
        pending: &Pending,
    ) -> PlayerAction {
        let agent = HeuristicAgent::new(self.profile)
            .with_teams(context.teams.clone())
            .with_seed(policy_seed(&context.game_id, context.seat.get()));
        agent.act(&house_view(view.clone()), pending)
    }
}

impl Mind for HouseMind {
    fn decide(&self, request: Request) -> Thinking<'_> {
        let house = self.clone();
        Box::pin(async move {
            // Off the async threads: a sharp profile searches, and a search
            // on a socket's thread is a socket that stops reading.
            let answered = tokio::task::spawn_blocking(move || {
                let started = Instant::now();
                let action = house.answer(&request.context, &request.view, &request.pending);
                Answer::new(action).took(started.elapsed())
            })
            .await;
            answered.map_err(|e| crate::MindError::Unavailable(format!("the house failed: {e}")))
        })
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::House
    }
}

/// The view the house answers from, made from a seat's socket view.
///
/// The fields `Session::agent_view` leaves out of a house chair's view,
/// left out again: the clock's remainder (wall time is not an input to a
/// house decision, #87), what the seat's standing policies answered (told to
/// the player, not to a decision), and every hand a teammate shows along
/// with who asked whom (#265).
#[must_use]
pub fn house_view(mut view: PlayerView) -> PlayerView {
    view.decision_remaining_ms = None;
    view.policy_acts.clear();
    view.shared_hands.clear();
    view.hand_shared_with = SeatSet::new();
    view.hand_requests = SeatSet::new();
    view.hand_requested = SeatSet::new();
    view
}
