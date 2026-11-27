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
        let new_val = *count;
        if kind == CounterKind::Energy && new_val < old && self.per_turn.energy_removed.tracked {
            self.per_turn
                .energy_removed
                .items
                .push(super::per_turn::EnergyRemovedRecord {
                    player,
                    amount: u32::from(old - new_val),
                });
        }
        self.journal.record(GameEvent::PlayerCountersChanged {
            player,
            kind,
            old,
            new: new_val,
        });
    }

    /// Pays exactly the requested energy, without making a partial payment.
    #[doc(hidden)]
    pub fn pay_energy(&mut self, player: PlayerId, amount: u32) -> bool {
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

    /// Multiplier from [`baylee_cards_dsl::Modifier::DoubleCountersPlacedByYou`]
    /// effects controlled by `by`.
    #[must_use]
    pub fn player_counter_multiplier(&self, by: PlayerId) -> u32 {
        let doubles = self
            .effects
            .iter()
            .filter(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::DoubleCountersPlacedByYou
                ) && fx.controller == by
            })
            .count();
        1u32.checked_shl(u32::try_from(doubles).unwrap_or(u32::MAX))
            .unwrap_or(u32::MAX)
    }

    /// Computes the replaced number of `kind` counters placed on `player` by
    /// `by`, applying [`baylee_cards_dsl::ReplacementRule::ModifyPlayerCounterPlacement`]
    /// and [`baylee_cards_dsl::Modifier::DoubleCountersPlacedByYou`] (CR 122.6,
    /// CR 614.1a, CR 614.16).
    #[must_use]
    pub fn replaced_player_counters(
        &self,
        by: PlayerId,
        player: PlayerId,
        kind: CounterKind,
        amount: u32,
    ) -> u32 {
        if amount == 0 {
            return 0;
        }
        let mut current = amount;
        for entry in &self.replacement_rules {
            if current == 0 {
                break;
            }
            let baylee_cards_dsl::ReplacementRule::ModifyPlayerCounterPlacement {
                recipient,
                kind: rule_kind,
                placer,
                op,
            } = entry.rule
            else {
                continue;
            };
            if !self.object(entry.source).is_some_and(|o| {
                o.zone == crate::zone::Zone::Battlefield
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
            }) {
                continue;
            }
            if rule_kind.is_some_and(|k| k != kind) {
                continue;
            }
            let recipient_ok = crate::eval::players(recipient, self, entry.controller)
                .is_some_and(|ps| ps.contains(&player));
            let placer_ok = placer.is_none_or(|rel| {
                crate::eval::players(rel, self, entry.controller).is_some_and(|ps| ps.contains(&by))
            });
            if !recipient_ok || !placer_ok {
                continue;
            }
            #[cfg(any(test, feature = "test-support"))]
            crate::ability_log::replaced(entry);
            current = match op {
                baylee_cards_dsl::CounterReplaceOp::Add(n) => current.saturating_add(u32::from(n)),
                baylee_cards_dsl::CounterReplaceOp::Sub(n) => current.saturating_sub(u32::from(n)),
                baylee_cards_dsl::CounterReplaceOp::Multiply(f) => {
                    current.saturating_mul(u32::from(f))
                }
                baylee_cards_dsl::CounterReplaceOp::HalveFloor => current / 2,
            };
        }
        current.saturating_mul(self.player_counter_multiplier(by))
    }

    /// Places `amount` counters of `kind` on `player` from `by`, applying
    /// player counter replacement rules and [`baylee_cards_dsl::Modifier::DoubleCountersPlacedByYou`]
    /// (CR 122.6, CR 614.16).
    pub fn put_player_counters_by(
        &mut self,
        by: PlayerId,
        player: PlayerId,
        kind: CounterKind,
        amount: u32,
    ) {
        if amount == 0 {
            return;
        }
        let scaled = self.replaced_player_counters(by, player, kind, amount);
        if scaled == 0 {
            return;
        }
        self.change_player_counters(player, kind, i32::try_from(scaled).unwrap_or(i32::MAX));
    }
}
