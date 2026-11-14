//! A frozen opponent and one human: changing transport or presentation cannot reroll a task.
use super::{Asked, Session};
use crate::record::Source;
use baylee_core::ids::PlayerId;
use baylee_core::preset::{GamePreset, HouseRules, SeatController};
use baylee_engine::choice::PlayerAction;

pub(super) fn valid(preset: &GamePreset) -> bool {
    let rules = &preset.house_rules;
    if rules.challenge_policy_seed.is_none() {
        return true;
    }
    preset.seats.len() == 2
        && matches!(
            preset.seats[0].controller,
            SeatController::Open | SeatController::Human { .. }
        )
        && matches!(preset.seats[1].controller, SeatController::Ai(profile) if profile.foresight().is_none())
        && rules.decision_timeout_secs == 0
        && rules.reconnect_window_secs == 0
        && !rules.takebacks
        && !rules.time_extension_votes
        && preset.seats.iter().all(|seat| seat.team.is_none())
}

pub(super) fn policy_seed(rules: &HouseRules, game: &str, seat: u8) -> u64 {
    rules.challenge_policy_seed.map_or_else(
        || baylee_ai::policy_seed(game, seat),
        |seed| baylee_ai::policy_seed(&format!("baylee/challenge/policy/v1/{seed}"), seat),
    )
}

impl<D: baylee_engine::engine::LoopDetection, R: crate::session::SessionRegistry> Session<D, R> {
    /// Frozen-policy serialization never delays conceding (CR 104.3a).
    pub(super) fn challenge_allows_seat_action(&self, action: &PlayerAction) -> bool {
        matches!(action, PlayerAction::Concede)
            || action.is_automation_setting()
            || !self.needs_pump()
    }

    /// One accepted opponent input, shared by live play and record reconstruction.
    pub(super) fn challenge_answered(&mut self, seat: PlayerId, by: Source) {
        if self.is_challenge() && seat.get() == 1 && by == Source::House {
            self.challenge_decisions += 1;
        }
    }

    /// Apply the exact fixed-policy answer, including deterministic recovery after
    /// a refused proposal. On mismatch the caller discards this replay session:
    /// the genuine policy action may already have been applied, never the forgery.
    pub(super) fn challenge_replay_apply(
        &mut self,
        seat: PlayerId,
        by: Source,
        action: &PlayerAction,
    ) -> Option<Asked> {
        match (seat.get(), by) {
            (0, Source::Seat) if self.challenge_allows_seat_action(action) => {
                let asked = self.answering(seat, action);
                self.engine.apply(seat, action.clone()).ok()?;
                Some(asked)
            }
            (1, Source::House) => {
                let proposal = self
                    .engine
                    .pending_for(seat)
                    .and_then(|pending| self.table().house_answer(seat, pending))?;
                self.replay_house_proposal(seat, proposal, action)
            }
            _ => None,
        }
    }

    /// Check the accepted result of the policy proposal, including its recovery.
    /// A caller seeing `None` must discard the replay session.
    pub(super) fn replay_house_proposal(
        &mut self,
        seat: PlayerId,
        proposal: PlayerAction,
        recorded: &PlayerAction,
    ) -> Option<Asked> {
        let (accepted, asked) = self.accept_house_proposal(seat, proposal, Source::House)?;
        (accepted == *recorded).then_some(asked)
    }

    /// Whether this game uses a frozen challenge opponent and may never replace its human.
    /// This says nothing about Central admission: only the official host may submit a score.
    #[must_use]
    pub const fn is_challenge(&self) -> bool {
        self.house_rules.challenge_policy_seed.is_some()
    }
}
