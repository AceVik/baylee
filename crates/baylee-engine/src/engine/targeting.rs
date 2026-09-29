//! Explicit shortcuts over an existing sequence of target decisions.
use super::{
    CardLookup, Engine, EngineError, ObjectId, Pending, PerOpponent, PlanKind, PlayerAction,
    PlayerId, SmallVec,
};
use crate::choice::TargetPrompt;
use crate::eval;
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

/// `spec` with "that player" read off the first instance of "target" of the
/// ability `obj` on the stack: the player it named, or the controller of the
/// permanent it named (last known, if it has left since). Every other spec
/// comes back as it was.
fn bind_to_first_targets_player(
    state: &crate::state::GameState,
    obj: &crate::object::GameObject,
    spec: baylee_cards_dsl::TargetSpec,
) -> baylee_cards_dsl::TargetSpec {
    let baylee_cards_dsl::TargetSpec::ObjectOfFirstTargetsPlayer(filter) = spec else {
        return spec;
    };
    let player = obj.target_players.iter().next().or_else(|| {
        obj.targets
            .first()
            .and_then(|id| state.object_or_departed(*id))
            .map(|o| o.controller)
    });
    match player {
        Some(player) => baylee_cards_dsl::TargetSpec::ObjectControlledBy(filter, player),
        // Nobody was named, so there is no "that player" and nothing to
        // offer; left unbound, it enumerates empty.
        None => spec,
    }
}

impl<L: CardLookup> Engine<L> {
    /// `controller`'s opponents, starting with the next seat in turn order
    /// (CR 101.4's order, the one a table reads round).
    pub(super) fn opponents_in_turn_order(&self, controller: PlayerId) -> Vec<PlayerId> {
        let seats = self.state.players.len();
        (1..seats)
            .map(|step| PlayerId::new(((usize::from(controller.get()) + step) % seats) as u8))
            .filter(|p| self.state.is_opponent(*p, controller))
            .collect()
    }

    /// Asks `controller` for the next opponent's target of a
    /// `TargetSpec::ObjectOfEachOpponent` trigger: up to one permanent that
    /// opponent controls. An opponent with nothing to point at is passed
    /// over rather than shown an empty menu.
    ///
    /// `None` means a question is now pending; `Some` hands back every
    /// target gathered, once nobody is left to ask about.
    pub(super) fn ask_next_opponent(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        mode: Option<u8>,
        mut asking: PerOpponent,
    ) -> Option<SmallVec<[ObjectId; 2]>> {
        while !asking.remaining.is_empty() {
            let opponent = asking.remaining.remove(0);
            let options: Vec<ObjectId> =
                eval::target_options(&asking.spec, &self.state, controller, source)
                    .into_iter()
                    .filter(|id| {
                        self.state
                            .object(*id)
                            .is_some_and(|o| o.controller == opponent)
                    })
                    .collect();
            if options.is_empty() {
                continue;
            }
            self.pending_plan = Some(PlanKind::Trigger {
                source,
                ability_index,
                mode,
                per_opponent: Some(Box::new(asking)),
            });
            self.pending = Pending::ChooseTargets {
                player: controller,
                options,
                player_options: Vec::new(),
                min: 0,
                max: 1,
                reason: TargetPrompt::Targets,
            };
            self.awaiting_answer = true;
            return None;
        }
        Some(asking.gathered)
    }

    /// Asks the second instance of "target" of the triggered ability that was
    /// just put on the stack, if it prints one.
    ///
    /// CR 603.3d makes putting a trigger on the stack the casting process
    /// from 601.2c on, and 601.2c announces the targets instance by instance,
    /// so the second is asked after the first is chosen — which is what lets
    /// "target creature **that player or that planeswalker's controller**
    /// controls" (Ravager of the Fells) name a player at all. The spec is
    /// bound to that player here and written onto the stack object, so the
    /// resolution-time re-check (CR 608.2b) asks the question the offer did.
    ///
    /// Returns whether a question is now pending.
    pub(crate) fn ask_trigger_second_target(&mut self) -> bool {
        let Some(top) = self
            .state
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .last()
            .copied()
        else {
            return false;
        };
        let Some(obj) = self.state.object(top) else {
            return false;
        };
        if obj.kind != crate::object::ObjectKind::AbilityOnStack
            || obj.second_target_req().is_some()
        {
            return false;
        }
        let Some(mut req) = self.stack_second_target_req(top) else {
            return false;
        };
        req.spec = bind_to_first_targets_player(&self.state, obj, req.spec);
        let controller = obj.controller;
        let (options, _) = eval::stack_target_options(&self.state, obj, &req.spec);
        if let Some(obj) = self.state.object_mut(top) {
            obj.set_second(SmallVec::new(), Some(req));
        }
        if options.len() < usize::from(req.min) {
            // No legal choice for a required target: "the ability is simply
            // removed from the stack" (CR 603.3d). No card prints a required
            // second target on a trigger today; Ravager's is "up to one".
            self.state
                .zones
                .remove(top, crate::zone::ZoneLocation::Stack);
            let _ = self.state.arena.remove(top);
            return false;
        }
        if options.is_empty() {
            // "Up to one" with nothing to point at: nothing to decide.
            return false;
        }
        let max = req.max.min(u8::try_from(options.len()).unwrap_or(u8::MAX));
        self.pending_plan = Some(PlanKind::TriggerSecondTarget { on_stack: top });
        self.pending = Pending::ChooseTargets {
            player: controller,
            options,
            player_options: Vec::new(),
            min: req.min,
            max,
            reason: TargetPrompt::Targets,
        };
        self.awaiting_answer = true;
        true
    }

    /// Number of consecutive occurrences of the exact current trigger.
    /// Non-trigger questions (including payments and modal choices) never
    /// qualify. Only the already collected queue is examined.
    #[must_use]
    pub fn target_batch_count(&self) -> u32 {
        // A question per opponent is one trigger's several answers, never
        // several triggers' one.
        if !matches!(self.pending, Pending::ChooseTargets { .. })
            || !matches!(
                self.pending_plan,
                Some(PlanKind::Trigger {
                    per_opponent: None,
                    ..
                })
            )
        {
            return 0;
        }
        let Some(first) = self.trigger_queue.front() else {
            return 0;
        };
        // A trigger that asks a second instance of "target" asks it between
        // one occurrence and the next, so the series is not one answer
        // repeated.
        if matches!(
            self.trigger_abilities(first)
                .get(first.ability_index as usize),
            Some(baylee_cards_dsl::AbilityDef::Triggered {
                second_targets: Some(_),
                ..
            })
        ) {
            return 0;
        }
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
