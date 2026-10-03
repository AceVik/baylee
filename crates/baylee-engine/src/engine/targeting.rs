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
        && a.text == b.text
        && a.source_version == b.source_version
        && a.event_object_identity == b.event_object_identity
        && a.counter_source_version == b.counter_source_version
        && a.timestamp == b.timestamp
        && a.ability_index == b.ability_index
        && a.abilities.as_ref().map(|v| (v.printed, &v.abilities))
            == b.abilities.as_ref().map(|v| (v.printed, &v.abilities))
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
    /// target gathered, once nobody is left to ask about. `plan` is what the
    /// answer returns to — a printed trigger's or a synthetic one's — given
    /// the question's state to carry.
    /// Every question uses the ability's captured wording.
    pub(super) fn ask_next_opponent_with_context(
        &mut self,
        controller: PlayerId,
        context: crate::text_changes::RuleContext,
        mut asking: PerOpponent,
        plan: impl FnOnce(Box<PerOpponent>) -> PlanKind,
    ) -> Option<SmallVec<[ObjectId; 2]>> {
        while !asking.remaining.is_empty() {
            let opponent = asking.remaining.remove(0);
            let options: Vec<ObjectId> =
                eval::target_options_with_context(&asking.spec, &self.state, controller, context)
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
            self.pending_plan = Some(plan(Box::new(asking)));
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
        let (min, max) = req.bounds(0);
        let max = max.min(u32::try_from(options.len()).unwrap_or(u32::MAX));
        self.pending_plan = Some(PlanKind::TriggerSecondTarget { on_stack: top });
        self.pending = Pending::ChooseTargets {
            player: controller,
            options,
            player_options: Vec::new(),
            min,
            max,
            reason: TargetPrompt::Targets,
        };
        self.awaiting_answer = true;
        true
    }

    /// Asks how the triggered ability just put on the stack divides its
    /// damage, if it prints "damage divided as you choose"
    /// ([`baylee_cards_dsl::Effect::DealDamageDivided`]).
    ///
    /// CR 601.2d, which CR 603.3d applies to a triggered ability: the
    /// division is announced as it is put on the stack, after its targets,
    /// and each target gets at least 1. One target takes it all and is not
    /// asked; no target divides nothing. Asked only when the second instance
    /// of "target" did not ask first — no card prints both.
    ///
    /// Returns whether a question is now pending.
    pub(crate) fn ask_trigger_division(&mut self) -> bool {
        let Some(top) = self
            .state
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .last()
            .copied()
        else {
            return false;
        };
        let Some(total) = self.stack_divided_amount(top) else {
            return false;
        };
        let Some(obj) = self.state.object(top) else {
            return false;
        };
        let (controller, targets) = (obj.controller, obj.targets.clone());
        // Entries for abilities that have left the stack since are dead.
        let state = &mut self.state;
        let on_stack = state.zones.list(crate::zone::ZoneLocation::Stack);
        state.divided.retain(|(id, _)| on_stack.contains(id));
        if targets.is_empty() {
            return false;
        }
        self.ask_division(controller, top, targets, Vec::new(), total)
    }

    /// Asks the next target's share of a division, or, when only the last
    /// target is left, gives it the rest and writes the division down.
    /// Returns whether a question is now pending.
    pub(super) fn ask_division(
        &mut self,
        controller: PlayerId,
        on_stack: ObjectId,
        targets: SmallVec<[ObjectId; 2]>,
        mut shares: Vec<u32>,
        total: u32,
    ) -> bool {
        let given: u32 = shares.iter().sum();
        let left = total.saturating_sub(given);
        let asked = shares.len();
        if asked + 1 >= targets.len() {
            shares.push(left);
            self.state.capture_source_references();
            let division = shares
                .into_iter()
                .enumerate()
                .map(|(index, share)| {
                    (
                        self.state
                            .recorded_target_reference(
                                on_stack,
                                false,
                                u32::try_from(index).expect("target slot"),
                            )
                            .expect("announced target"),
                        share,
                    )
                })
                .collect();
            self.state.divided.push((on_stack, division));
            return false;
        }
        // Each target still to come needs at least 1 (CR 601.2d); the
        // `TargetReq` asks for no more targets than there is damage, so this
        // leaves at least 1 for this one.
        let after = u32::try_from(targets.len() - asked - 1).unwrap_or(u32::MAX);
        let max = left.saturating_sub(after).max(1);
        let reason = crate::choice::NumberPrompt::DivideDamage {
            target: targets[asked],
            index: u8::try_from(asked).unwrap_or(u8::MAX),
            of: u8::try_from(targets.len()).unwrap_or(u8::MAX),
            left,
        };
        self.pending_plan = Some(PlanKind::DivideDamage {
            on_stack,
            targets,
            shares,
            total,
        });
        self.pending = Pending::ChooseNumber {
            player: controller,
            min: 1,
            max,
            reason,
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
        if self.decision_actor() != Some(player) {
            return Err(EngineError::MismatchedAction);
        }
        let subject = self.pending.asked();
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
                if Some(*chooser) == subject
                    && (u64::from(*min)..=u64::from(*max)).contains(&u64::try_from(objects.len() + players.len()).unwrap_or(u64::MAX))
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
            let answered = self.apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: objects.to_vec(),
                    players: players.to_vec(),
                },
            );
            // Only the first answer may refuse the batch: past it, earlier
            // answers are taken, and a refusal would tell the record the
            // batch changed nothing when it did (the batch ends here instead,
            // as it does on an answer that stops fitting).
            if answered.is_err() {
                if at == 0 {
                    return answered;
                }
                break;
            }
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

    fn frozen_granted_targets(per_opponent: bool) -> (Engine<RegistryLookup>, [ObjectId; 2]) {
        use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
        use crate::event::GameEvent;
        use crate::text_changes::TextReplacement;
        use baylee_cards_dsl::{
            Duration, Effect, Filter, Layer, Modifier, PlayerRel, StepKind, TargetSpec,
            TextWordKind, Trigger,
        };
        use baylee_core::color::{Color, ColorSet};
        use baylee_core::ids::EffectId;

        static GREEN: Filter = Filter::HasColor(ColorSet::of(Color::Green));
        let player = PlayerId::new(0);
        let mut engine = Duel::table(927, card("Forest"), 3)
            .battlefield(0, &[card("Forest"), card("Llanowar Elves")])
            .battlefield(1, &[card("Hill Giant"), card("Air Elemental")])
            .battlefield(2, &[card("Hill Giant"), card("Air Elemental")])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, player);
        let grantor = on_battlefield(&engine, player, card("Forest")).unwrap();
        let recipient = on_battlefield(&engine, player, card("Llanowar Elves")).unwrap();
        let red = [1, 2]
            .map(|seat| on_battlefield(&engine, PlayerId::new(seat), card("Hill Giant")).unwrap());
        for (source, color) in [(grantor, Color::Red), (recipient, Color::Blue)] {
            let reference = engine.state.source_identity(source).unwrap();
            assert!(engine.state.text_changes.replace(
                reference,
                TextReplacement {
                    kind: TextWordKind::Color,
                    from: Color::Green as u8,
                    to: color as u8,
                }
            ));
        }
        engine.state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(grantor),
            controller: player,
            origin: EffectOrigin::Static,
            layer: Layer::Ability,
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::object(&engine.state, recipient),
            modifier: Modifier::GrantTriggered {
                trigger: Trigger::StepBegin {
                    step: StepKind::Upkeep,
                    whose: PlayerRel::You,
                },
                effects: &[Effect::TapTarget],
                target: Some(if per_opponent {
                    TargetSpec::ObjectOfEachOpponent(&GREEN)
                } else {
                    TargetSpec::Object(&GREEN)
                }),
            },
        });
        let from = engine.state.journal.last_seq();
        engine.state.journal.record(GameEvent::StepChanged {
            phase: crate::turn::Phase::Beginning,
            step: crate::turn::Step::Upkeep,
        });
        let captured = crate::trigger::collect(&engine.state, &engine.lookup, from);
        assert_eq!(captured.len(), 1);
        engine.trigger_queue.extend(captured);
        engine.trigger_scan_seq = engine.state.journal.last_seq();
        // Neither the recipient's blue wording nor the grantor's later
        // change may replace the red target criterion already captured.
        let reference = engine.state.source_identity(grantor).unwrap();
        assert!(engine.state.text_changes.replace(
            reference,
            TextReplacement {
                kind: TextWordKind::Color,
                from: Color::Red as u8,
                to: Color::Blue as u8,
            }
        ));
        engine.collect_triggers();
        (engine, red)
    }

    #[test]
    fn granted_trigger_target_offers_use_frozen_grantor_words() {
        let (engine, red) = frozen_granted_targets(false);
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("the granted trigger asks for its target");
        };
        assert_eq!(options.as_slice(), &red);
    }

    #[test]
    fn each_opponents_granted_targets_keep_the_same_frozen_words() {
        let (mut engine, red) = frozen_granted_targets(true);
        for target in red {
            let Pending::ChooseTargets { options, .. } = engine.pending() else {
                panic!("one frozen target question per opponent");
            };
            assert_eq!(options, &[target]);
            engine
                .apply(
                    PlayerId::new(0),
                    PlayerAction::ChooseObjects {
                        objects: vec![target],
                    },
                )
                .unwrap();
        }
        let stack = engine.state.zones.list(crate::zone::ZoneLocation::Stack);
        let ability = engine.state.object(*stack.last().unwrap()).unwrap();
        assert_eq!(ability.targets.as_slice(), &red);
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
