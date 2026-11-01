//! Per-player upkeep windows for exact combat-participation history.
use super::GameState;
use baylee_core::ids::{DamageSourceRef, PlayerId};

impl GameState {
    pub(crate) fn record_blocked_or_blocking(
        &mut self,
        blocker: DamageSourceRef,
        attacker: DamageSourceRef,
    ) {
        for player in &mut self.players {
            for reference in [blocker, attacker] {
                if !player.blocked_since_upkeep.contains(&reference) {
                    player.blocked_since_upkeep.push(reference);
                }
            }
        }
    }

    pub(crate) fn begin_upkeep_history(&mut self, active: PlayerId) {
        let player = &mut self.players[usize::from(active.get())];
        player.blocked_before_upkeep = std::mem::take(&mut player.blocked_since_upkeep);
    }
}
