//! Per-player upkeep windows: exact combat-participation history, and the
//! stamps "since the beginning of your last upkeep" is measured against.
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
        let stamp = self.timestamp;
        let player = &mut self.players[usize::from(active.get())];
        player.blocked_before_upkeep = std::mem::take(&mut player.blocked_since_upkeep);
        player.last_upkeep_began = player.upkeep_began;
        player.upkeep_began = stamp;
    }
}
