//! Player counters are public, persistent resources, never mana.
use super::{CounterKind, GameEvent, GameState, Player, PlayerId};

impl Player {
    /// Poison, energy and radiation: all player counter kinds used by this pool.
    #[must_use]
    pub const fn counters(&self) -> [(CounterKind, u16); 3] {
        [
            (CounterKind::Poison, self.poison),
            (CounterKind::Energy, self.energy),
            (CounterKind::Rad, self.rad),
        ]
    }
}

impl GameState {
    /// Changes a player's counter total. Negative amounts remove up to what is
    /// present; payments check affordability first. Departed seats receive none.
    pub fn change_player_counters(&mut self, player: PlayerId, kind: CounterKind, amount: i32) {
        if self.has_left(player) || amount == 0 {
            return;
        }
        let Some(seat) = self.players.get_mut(usize::from(player.get())) else {
            return;
        };
        let count = match kind {
            CounterKind::Poison => &mut seat.poison,
            CounterKind::Energy => &mut seat.energy,
            CounterKind::Rad => &mut seat.rad,
            _ => {
                self.numeric_failure = Some("unsupported player counter kind");
                return;
            }
        };
        let old = *count;
        let next = i64::from(old) + i64::from(amount);
        let Ok(next) = u16::try_from(next.max(0)) else {
            self.numeric_failure = Some("player counter amount exceeds the supported range");
            return;
        };
        *count = next;
        self.journal.record(GameEvent::PlayerCountersChanged {
            player,
            kind,
            old,
            new: *count,
        });
    }

    /// Pays exactly the requested energy, without making a partial payment.
    pub(crate) fn pay_energy(&mut self, player: PlayerId, amount: u32) -> bool {
        if self.has_left(player)
            || self
                .players
                .get(usize::from(player.get()))
                .is_none_or(|p| u32::from(p.energy) < amount)
        {
            return false;
        }
        self.change_player_counters(
            player,
            CounterKind::Energy,
            -i32::try_from(amount).expect("bounded by held energy"),
        );
        true
    }
}
