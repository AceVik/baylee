//! A frozen opponent and one human: changing transport or presentation cannot reroll a task.
use super::Session;
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

impl Session {
    /// One accepted opponent input, shared by live play and record reconstruction.
    pub(super) fn challenge_answered(&mut self, seat: PlayerId, by: Source) {
        if self.is_challenge() && seat.get() == 1 && by == Source::House {
            self.challenge_decisions += 1;
        }
    }

    /// Rebuild the fixed agent's state while replaying, and verify every claimed
    /// house action. Otherwise a restarted host would forget its stall memory.
    pub(super) fn challenge_replay_answer(
        &self,
        seat: PlayerId,
        by: Source,
        action: &PlayerAction,
    ) -> bool {
        match (seat.get(), by) {
            (0, Source::Seat) => action.is_automation_setting() || !self.needs_pump(),
            (1, Source::House) => {
                self.engine
                    .pending_for(seat)
                    .and_then(|pending| self.table().house_answer(seat, pending))
                    .as_ref()
                    == Some(action)
            }
            _ => false,
        }
    }

    /// Whether this game uses a frozen challenge opponent and may never replace its human.
    /// This says nothing about Central admission: only the official host may submit a score.
    #[must_use]
    pub const fn is_challenge(&self) -> bool {
        self.house_rules.challenge_policy_seed.is_some()
    }
}
