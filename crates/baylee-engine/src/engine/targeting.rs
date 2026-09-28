//! Explicit shortcuts over an existing sequence of target decisions.
use super::{CardLookup, Engine, EngineError, ObjectId, Pending, PlanKind, PlayerAction, PlayerId};
use crate::trigger::PendingTrigger;

/// Same source, controller, captured rules and chosen mode. Each occurrence
/// keeps its own event payload; only the explicit targets are shared.
fn same(a: &PendingTrigger, b: &PendingTrigger) -> bool {
    a.source == b.source
        && a.timestamp == b.timestamp
        && a.ability_index == b.ability_index
        && a.abilities.map(|v| (v.printed, v.abilities))
            == b.abilities.map(|v| (v.printed, v.abilities))
        && a.controller == b.controller
        && a.chosen_mode == b.chosen_mode
        && a.implicit_target == b.implicit_target
        && !a.once_per_turn
        && !b.once_per_turn
        && a.synthetic_effects.is_none()
        && b.synthetic_effects.is_none()
}

impl<L: CardLookup> Engine<L> {
    /// Number of consecutive occurrences of the exact current trigger.
    /// Non-trigger questions (including payments and modal choices) never
    /// qualify. Only the already collected queue is examined.
    #[must_use]
    pub fn target_batch_count(&self) -> u32 {
        if !matches!(self.pending, Pending::ChooseTargets { .. })
            || !matches!(self.pending_plan, Some(PlanKind::Trigger { .. }))
        {
            return 0;
        }
        let Some(first) = self.trigger_queue.front() else {
            return 0;
        };
        u32::try_from(
            self.trigger_queue
                .iter()
                .take_while(|t| same(first, t))
                .count(),
        )
        .unwrap_or(u32::MAX)
    }

    pub(super) fn apply_target_batch(
        &mut self,
        player: PlayerId,
        objects: &[ObjectId],
        players: &[PlayerId],
        count: u32,
    ) -> Result<(), EngineError> {
        let available = self.target_batch_count();
        if count < 2 || count > available {
            return Err(EngineError::IllegalAction("target series changed"));
        }
        let first = self
            .trigger_queue
            .front()
            .cloned()
            .expect("nonempty target series");
        for at in 0..count {
            if at > 0
                && (self.target_batch_count() == 0
                    || !self.trigger_queue.front().is_some_and(|t| same(&first, t)))
            {
                break;
            }
            let valid = matches!(&self.pending, Pending::ChooseTargets { player: chooser, options, player_options, min, max, .. }
                if *chooser == player
                    && (usize::from(*min)..=usize::from(*max)).contains(&(objects.len() + players.len()))
                    && objects.iter().all(|o| options.contains(o))
                    && players.iter().all(|p| player_options.contains(p))
                    && objects.iter().enumerate().all(|(i,o)| !objects[..i].contains(o))
                    && players.iter().enumerate().all(|(i,p)| !players[..i].contains(p)));
            if !valid {
                if at == 0 {
                    return Err(EngineError::IllegalAction("invalid target selection"));
                }
                break;
            }
            self.apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: objects.to_vec(),
                    players: players.to_vec(),
                },
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::testkit::*;
    use baylee_core::ids::CardIndex;

    fn card(name: &str) -> CardIndex {
        baylee_cards::generated::ALL
            .iter()
            .find(|(_, c)| c.name() == name)
            .unwrap()
            .1
            .index
    }
    // A real targeted ETB, then repeat its collected occurrence as trigger
    // multipliers do. Compare the shortcut with the ordinary answer door.
    fn series(seats: usize, count: usize) -> Engine<RegistryLookup> {
        let mut e = Duel::table(913, card("Swamp"), seats)
            .battlefield(0, &[card("Swamp"); 3])
            .hand(0, &[card("Boggart Trawler")])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, PlayerId::new(0));
        cast_from_hand(&mut e, PlayerId::new(0), card("Boggart Trawler"));
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        assert_eq!(e.trigger_queue.len(), 1);
        let first = e.trigger_queue.front().unwrap().clone();
        e.trigger_queue
            .extend(std::iter::repeat_n(first, count - 1));
        e
    }
    #[test]
    fn fifty_answers_equal_fifty_manual_answers_in_duels_and_multiplayer() {
        for seats in [2, 3, 6] {
            let mut batch = series(seats, 50);
            let mut manual = series(seats, 50);
            let target = PlayerId::new(u8::try_from(seats - 1).unwrap());
            assert_eq!(batch.target_batch_count(), 50);
            batch
                .apply(
                    PlayerId::new(0),
                    PlayerAction::ChooseTargetBatch {
                        objects: vec![],
                        players: vec![target],
                        count: 50,
                    },
                )
                .unwrap();
            for _ in 0..50 {
                manual
                    .apply(
                        PlayerId::new(0),
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![target],
                        },
                    )
                    .unwrap();
            }
            assert_eq!(batch.snapshot_hash(), manual.snapshot_hash());
            assert_eq!(
                batch
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Stack)
                    .len(),
                50
            );
            assert_eq!(batch.target_batch_count(), 0);
        }
    }
    #[test]
    fn batch_rejects_stale_count_wrong_seat_and_duplicate_targets_without_mutation() {
        let mut e = series(3, 4);
        let before = e.snapshot_hash();
        for (seat, players, count) in [
            (0, vec![PlayerId::new(1)], 5),
            (1, vec![PlayerId::new(1)], 4),
            (0, vec![PlayerId::new(1); 2], 4),
            (0, vec![PlayerId::new(7)], 4),
        ] {
            assert!(
                e.apply(
                    PlayerId::new(seat),
                    PlayerAction::ChooseTargetBatch {
                        objects: vec![],
                        players,
                        count
                    }
                )
                .is_err()
            );
            assert_eq!(e.snapshot_hash(), before);
            assert_eq!(e.target_batch_count(), 4);
        }
    }
    #[test]
    fn explicit_count_and_different_abilities_bound_the_shortcut() {
        let mut e = series(3, 5);
        e.trigger_queue[3].ability_index += 1;
        assert_eq!(e.target_batch_count(), 3);
        e.apply(
            PlayerId::new(0),
            PlayerAction::ChooseTargetBatch {
                objects: vec![],
                players: vec![PlayerId::new(2)],
                count: 2,
            },
        )
        .unwrap();
        assert_eq!(e.trigger_queue.len(), 3);
        assert_eq!(e.target_batch_count(), 1);
        assert!(matches!(e.pending(), Pending::ChooseTargets { .. }));
    }
}
