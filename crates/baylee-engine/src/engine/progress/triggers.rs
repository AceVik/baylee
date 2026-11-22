//! Triggered abilities: collected, queued, bound and put on the stack.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Writes the chosen mode onto the ability just put on the stack.
    ///
    /// The same shape as the `event_object` write beside every one of these
    /// calls, and for the same reason: `push_ability_to_stack` takes the
    /// handle and the targets, and everything else a stack object carries is
    /// written onto it afterwards rather than threaded through a signature
    /// most callers pass empty. Every trigger push site calls this —
    /// `resolve_stack_top` reads the mode back with an `expect`, so a site
    /// that forgot would panic rather than silently resolve mode 0, which is
    /// the failure entry 34 is.
    pub(crate) fn set_top_mode(&mut self, mode: Option<u8>) {
        let Some(mode) = mode else {
            return;
        };
        if let Some(top) = self.state.zones.list(ZoneLocation::Stack).last().copied()
            && let Some(obj) = self.state.object_mut(top)
        {
            obj.mode_index = Some(mode);
            if let (Some(abilities), Some(loc)) = (&obj.own_abilities, obj.ability) {
                obj.target_req =
                    crate::object::ability_target_req(abilities, loc.index, Some(mode));
            }
        }
    }

    /// The frozen selected trigger, not its source's later printed identity.
    pub(in crate::engine) fn trigger_wall_only(
        &self,
        trigger: &crate::trigger::PendingTrigger,
    ) -> bool {
        if trigger.synthetic_effects.is_some() {
            return trigger
                .synthetic_target
                .is_some_and(|req| eval::target_domain::wall_spec(req.spec));
        }
        self.trigger_abilities(trigger)
            .get(trigger.ability_index as usize)
            .is_some_and(|definition| {
                eval::target_domain::ability_walls(definition, trigger.chosen_mode)
            })
    }

    /// The ability list a queued trigger's index points into.
    ///
    /// The source answers for itself in every ordinary case, and the trigger
    /// carries its own list in exactly one: a look-back trigger (CR 603.10a)
    /// whose source has stopped being a copy on the way off the battlefield.
    /// Four places read an ability out of a queued trigger — the modes, the
    /// target requirement, and two of the three doors to the stack — at four
    /// different moments, and the object underneath is free to change between
    /// them, so all four ask here.
    pub(in crate::engine) fn trigger_abilities(
        &self,
        t: &crate::trigger::PendingTrigger,
    ) -> crate::copiable_abilities::AbilityDefs {
        t.abilities.as_ref().map_or_else(
            || {
                self.state
                    .object(t.source)
                    .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |o| {
                        o.printed_abilities(&self.lookup)
                    })
            },
            |list| list.abilities.clone(),
        )
    }

    /// Puts a look-back trigger's own ability list in the slot
    /// [`Self::push_ability_to_stack`] captures from (CR 608.2).
    ///
    /// The ability on the stack takes the list its index points into with it,
    /// and reads it off the source unless something has left it there. An
    /// activation leaves it there because paying the cost may already have
    /// moved the source; a look-back trigger leaves it there because the
    /// source stopped being a copy on the way to the graveyard — the hazard
    /// the capture's own comment names, met by the one case that meets it.
    pub(crate) fn hand_over_trigger_abilities(&mut self, t: &crate::trigger::PendingTrigger) {
        if let Some(abilities) = t.abilities.clone() {
            self.activating_abilities = Some((t.source, abilities));
        }
    }

    pub(in crate::engine) fn record_trigger_fired(&mut self, t: &crate::trigger::PendingTrigger) {
        if t.once_per_turn {
            let version = t
                .source_version
                .unwrap_or_else(|| self.state.object(t.source).expect("trigger source").version);
            self.state.ability_fires.insert(
                (
                    baylee_core::ids::DamageSourceRef {
                        object: t.source,
                        version,
                    },
                    t.fire_key(),
                ),
                1,
            );
        }
    }

    /// The Aura's graveyard incarnation a departure trigger of its host
    /// finds, if any.
    ///
    /// CR 400.7f: an ability that triggers on its enchanted permanent
    /// leaving finds the Aura in its owner's graveyard when the Aura went
    /// there at the same time or by a state-based action. Nothing else
    /// has happened between the event and this moment, so an Aura the
    /// host was wearing as it left, now in its owner's graveyard one
    /// move after the incarnation that triggered, is that object.
    fn aura_successor(&self, trigger: &crate::trigger::PendingTrigger) -> Option<u32> {
        trigger
            .event_departure
            .and(trigger.event_object)
            .filter(|host| *host != trigger.source)
            .filter(|host| {
                self.state
                    .ltb_attachments
                    .iter()
                    .any(|(left, worn)| left == host && worn.contains(&trigger.source))
            })
            .and_then(|_| {
                let old = trigger.source_version?;
                let aura = self.state.object(trigger.source)?;
                (aura.zone == crate::zone::Zone::Graveyard
                    && aura.zone_owner == Some(aura.owner)
                    && old.checked_add(1) == Some(aura.version))
                .then_some(aura.version)
            })
    }

    /// Preserve event context independently of the source and event object.
    #[allow(clippy::too_many_lines)] // one rider per thing a trigger remembers
    pub(crate) fn bind_top_trigger(&mut self, trigger: &crate::trigger::PendingTrigger) {
        let Some(top) = self.state.zones.list(ZoneLocation::Stack).last().copied() else {
            return;
        };
        if let Some(reference) = self.state.source_identity(top) {
            self.state.text_changes.set(reference, trigger.text);
        }
        let bound = self.stack_target_req(top).map(|mut req| {
            req.spec = self.bound_trigger_target(trigger, req.spec);
            req
        });
        let graveyard_source = is_graveyard_trigger(&self.state, trigger);
        // An instruction about the event's exact incarnation ("destroy it",
        // Kudzu) remembers which object that was as the trigger goes on the
        // stack, so one that has left the battlefield since is a new object
        // it does not touch (CR 400.7).
        let identity = trigger.event_object_identity.or_else(|| {
            let event = trigger.event_object?;
            let needs = self
                .trigger_abilities(trigger)
                .get(trigger.ability_index as usize)
                .is_some_and(|ability| match ability {
                    AbilityDef::Triggered { effects, .. } => effects.iter().any(|effect| {
                        matches!(
                            effect,
                            baylee_cards_dsl::Effect::DestroyEventThenMayReattach { .. }
                        )
                    }),
                    _ => false,
                });
            let object = self.state.object(event).filter(|_| needs)?;
            Some((object.version, object.characteristics().power.unwrap_or(0)))
        });
        let aura_successor = self.aura_successor(trigger);
        let (source_snow_spent, source_entry_sacrifices) = self
            .state
            .damage_source(trigger.source, trigger.source_version)
            .map_or((0, 0), |src| {
                let snow = src
                    .paid
                    .as_ref()
                    .map(|p| p.snow_mana_spent())
                    .or_else(|| {
                        src.riders.iter().find_map(|r| match r {
                            crate::object::Rider::SnowManaSpent(n) => Some(*n),
                            _ => None,
                        })
                    })
                    .unwrap_or(0);
                let sacs = src
                    .riders
                    .iter()
                    .find_map(|r| match r {
                        crate::object::Rider::EntrySacrifices(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(0);
                (snow, sacs)
            });
        if let Some(object) = self.state.object_mut(top) {
            if source_snow_spent > 0 {
                object
                    .riders
                    .push(crate::object::Rider::SnowManaSpent(source_snow_spent));
            }
            if source_entry_sacrifices > 0 {
                object.riders.push(crate::object::Rider::EntrySacrifices(
                    source_entry_sacrifices,
                ));
            }
            object.event_object = trigger.event_object;
            if let Some(grantor) = trigger.grantor {
                object
                    .riders
                    .push(crate::object::Rider::GrantedSource(grantor));
            }
            if let Some(value) = trigger.event_mana_value {
                object
                    .riders
                    .push(crate::object::Rider::EventManaValue(value));
            }
            if let Some(ability) = trigger.event_ability {
                object
                    .riders
                    .push(crate::object::Rider::EventAbility(ability));
            }
            if let Some(subject) = trigger.event_subject {
                object
                    .riders
                    .push(crate::object::Rider::EventSubject(subject));
            }
            if let Some(related) = &trigger.related_objects {
                object.riders.extend(
                    related
                        .iter()
                        .copied()
                        .map(crate::object::Rider::DelayedReturnObject),
                );
            }
            if let Some(found) = &trigger.found_objects {
                object.riders.extend(
                    found
                        .iter()
                        .copied()
                        .map(crate::object::Rider::EventFoundObject),
                );
            }
            if let Some((version, power)) = identity {
                object
                    .riders
                    .push(crate::object::Rider::EventObjectIdentity(version, power));
            }
            if let Some(version) = trigger.source_version {
                object
                    .riders
                    .retain(|r| !matches!(r, crate::object::Rider::AbilitySourceVersion(_)));
                object
                    .riders
                    .push(crate::object::Rider::AbilitySourceVersion(version));
                if graveyard_source {
                    object
                        .riders
                        .push(crate::object::Rider::TriggerSourceVersion(version));
                }
            }
            if let Some((controller, toughness)) =
                trigger.event_departure.or(trigger.event_tap_snapshot)
            {
                object
                    .riders
                    .push(crate::object::Rider::EventDeparture(controller, toughness));
            }
            if let Some(version) = aura_successor {
                object
                    .riders
                    .push(crate::object::Rider::SourceAuraSuccessor(version));
            }
            if let Some(version) = trigger.counter_source_version {
                object
                    .riders
                    .retain(|r| !matches!(r, crate::object::Rider::CounterSourceVersion(_)));
                object
                    .riders
                    .push(crate::object::Rider::CounterSourceVersion(version));
            }
            if let Some(amount) = trigger.event_amount {
                object
                    .riders
                    .push(crate::object::Rider::EventAmount(amount));
            }
            if let Some(player) = trigger.event_player {
                object
                    .riders
                    .push(crate::object::Rider::EventPlayer(player));
            }
            if let Some(condition) = trigger.synthetic_condition {
                object
                    .riders
                    .push(crate::object::Rider::SyntheticCondition(condition));
            }
            object.target_req = bound;
        }
    }

    /// Bind an attack's defending player while the declaration is still
    /// known. An Equipment's trigger can be retargeted after the wearer
    /// blinks, when its new incarnation has never attacked (CR 508.5).
    fn bound_trigger_target(
        &self,
        trigger: &crate::trigger::PendingTrigger,
        spec: baylee_cards_dsl::TargetSpec,
    ) -> baylee_cards_dsl::TargetSpec {
        use baylee_cards_dsl::{TargetSpec, Trigger};
        let Some(remaining) = eval::defenders_target_filter(spec) else {
            return trigger.bind_target(spec);
        };
        let Some(attacker) = trigger.event_object else {
            return spec;
        };
        if matches!(
            self.trigger_abilities(trigger)
                .get(trigger.ability_index as usize),
            Some(
                AbilityDef::Triggered {
                    trigger: Trigger::Attacks(_),
                    ..
                } | AbilityDef::ModalTriggered {
                    trigger: Trigger::Attacks(_),
                    ..
                }
            )
        ) && let Some(defender) =
            crate::combat::historical_defending_player(&self.state, attacker)
        {
            return TargetSpec::ObjectControlledBy(remaining, defender);
        }
        spec
    }

    fn trigger_target_options(
        &self,
        trigger: &crate::trigger::PendingTrigger,
        spec: baylee_cards_dsl::TargetSpec,
        wall_only: bool,
    ) -> (Vec<ObjectId>, Vec<PlayerId>) {
        let spec = self.bound_trigger_target(trigger, spec);
        let context = crate::text_changes::RuleContext {
            source: trigger.source,
            text: trigger.text,
            resolved_name: None,
            resolved_subtype: None,
            reference: trigger.event_object,
        };
        (
            eval::target_options_in_domain(
                &spec,
                &self.state,
                trigger.controller,
                context,
                None,
                wall_only,
            ),
            eval::target_player_options_from(&self.state, &spec, trigger.controller, context),
        )
    }

    /// The modes of a queued trigger, if it is a modal one.
    fn modal_trigger_modes(
        &self,
        t: &crate::trigger::PendingTrigger,
    ) -> Option<(
        &'static [baylee_cards_dsl::SpellMode],
        baylee_cards_dsl::ModalRestriction,
    )> {
        self.trigger_abilities(t)
            .get(t.ability_index as usize)
            .and_then(|a| match a {
                AbilityDef::ModalTriggered {
                    modes, restriction, ..
                } => Some((*modes, *restriction)),
                _ => None,
            })
    }

    fn modal_trigger_mask(
        &self,
        t: &crate::trigger::PendingTrigger,
        restriction: baylee_cards_dsl::ModalRestriction,
    ) -> u64 {
        use baylee_cards_dsl::ModalRestriction;
        let Some(version) = t.source_version else {
            return 0;
        };
        let key = (
            baylee_core::ids::DamageSourceRef {
                object: t.source,
                version,
            },
            t.fire_key(),
        );
        match restriction {
            ModalRestriction::None => None,
            ModalRestriction::OncePerTurn => self.state.per_turn.modal_modes_this_turn.0.get(&key),
            ModalRestriction::OncePerPermanent => self.state.modal_modes_used.get(&key),
        }
        .copied()
        .unwrap_or_default()
    }

    pub(in crate::engine) fn record_modal_trigger_mode(&mut self, mode: usize) {
        use baylee_cards_dsl::ModalRestriction;
        let Some(t) = self.trigger_queue.front() else {
            return;
        };
        if let Some((_, restriction)) = self.modal_trigger_modes(t)
            && let Some(version) = t.source_version
            && mode < 64
        {
            let key = (
                baylee_core::ids::DamageSourceRef {
                    object: t.source,
                    version,
                },
                t.fire_key(),
            );
            let used = match restriction {
                ModalRestriction::None => return,
                ModalRestriction::OncePerTurn => &mut self.state.per_turn.modal_modes_this_turn.0,
                ModalRestriction::OncePerPermanent => &mut self.state.modal_modes_used,
            };
            *used.entry(key).or_default() |= 1 << mode;
        }
    }

    fn trigger_second_depends_on_first(spec: baylee_cards_dsl::TargetSpec) -> bool {
        crate::casting::spec_reads_reference(spec)
            || matches!(
                spec,
                baylee_cards_dsl::TargetSpec::ObjectOfFirstTargetsPlayer(_)
                    | baylee_cards_dsl::TargetSpec::CardInFirstTargetsGraveyard(_)
            )
    }

    fn trigger_second_satisfied_for_first(
        &self,
        t: &crate::trigger::PendingTrigger,
        second_req: baylee_cards_dsl::TargetReq,
        wall_only: bool,
        first_obj: Option<ObjectId>,
        first_player: Option<PlayerId>,
    ) -> bool {
        if second_req.min == 0 {
            return true;
        }
        let mut spec = self.bound_trigger_target(t, second_req.spec);
        let player = first_player.or_else(|| {
            first_obj
                .and_then(|id| self.state.object_or_departed(id))
                .map(|o| o.controller)
        });
        match (spec, player) {
            (baylee_cards_dsl::TargetSpec::ObjectOfFirstTargetsPlayer(filter), Some(player)) => {
                spec = baylee_cards_dsl::TargetSpec::ObjectControlledBy(filter, player);
            }
            (baylee_cards_dsl::TargetSpec::CardInFirstTargetsGraveyard(filter), Some(player)) => {
                spec = baylee_cards_dsl::TargetSpec::CardInPlayersGraveyard(filter, player);
            }
            (
                baylee_cards_dsl::TargetSpec::ObjectOfFirstTargetsPlayer(_)
                | baylee_cards_dsl::TargetSpec::CardInFirstTargetsGraveyard(_),
                None,
            ) => return false,
            _ => {}
        }
        let ctx = crate::text_changes::RuleContext {
            source: t.source,
            text: t.text,
            resolved_name: None,
            resolved_subtype: None,
            reference: first_obj.or(t.event_object),
        };
        let object_list =
            eval::target_options_in_domain(&spec, &self.state, t.controller, ctx, None, wall_only);
        if second_req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
            return !eval::eligible_same_controller_groups(
                &self.state,
                &object_list,
                u32::from(second_req.min),
            )
            .is_empty();
        }
        if second_req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
            return eval::one_per_player_satisfied(
                &self.state,
                &spec,
                t.controller,
                &object_list,
                second_req.min,
            );
        }
        let objects = object_list.len();
        let players = eval::target_player_options_from(&self.state, &spec, t.controller, ctx).len();
        objects + players >= usize::from(second_req.min)
    }

    /// Whether `mode` can legally be chosen for `t` right now (CR 603.3c).
    ///
    /// A mode that needs targets it cannot find is off the list. The count
    /// is the same sum the target question below uses — objects and players
    /// together, because "any target" offers both at once (CR 115.4) — so a
    /// mode that survives this test is one the question can actually be
    /// answered for.
    #[allow(clippy::too_many_lines)]
    fn mode_is_choosable(&self, t: &crate::trigger::PendingTrigger, mode: &SpellMode) -> bool {
        let wall_only = eval::target_domain::walls(mode.targets, mode.second_targets);
        let ctx = crate::text_changes::RuleContext {
            source: t.source,
            text: t.text,
            resolved_name: None,
            resolved_subtype: None,
            reference: t.event_object,
        };
        let req_satisfied = |req: baylee_cards_dsl::TargetReq| {
            if req.min == 0 {
                return true;
            }
            if matches!(req.spec, baylee_cards_dsl::TargetSpec::EventObject) {
                return t.event_object.is_some();
            }
            let spec = self.bound_trigger_target(t, req.spec);
            let object_list = eval::target_options_in_domain(
                &spec,
                &self.state,
                t.controller,
                ctx,
                None,
                wall_only,
            );
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
                return !eval::eligible_same_controller_groups(
                    &self.state,
                    &object_list,
                    u32::from(req.min),
                )
                .is_empty();
            }
            if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                return eval::one_per_player_satisfied(
                    &self.state,
                    &spec,
                    t.controller,
                    &object_list,
                    req.min,
                );
            }
            let objects = object_list.len();
            let players =
                eval::target_player_options_from(&self.state, &spec, t.controller, ctx).len();
            objects + players >= usize::from(req.min)
        };
        if let Some(first_req) = mode.targets {
            if !req_satisfied(first_req) {
                return false;
            }
            if let Some(second_req) = mode.second_targets
                && second_req.min > 0
            {
                if Self::trigger_second_depends_on_first(second_req.spec) {
                    if matches!(first_req.spec, baylee_cards_dsl::TargetSpec::EventObject) {
                        return self.trigger_second_satisfied_for_first(
                            t,
                            second_req,
                            wall_only,
                            t.event_object,
                            None,
                        );
                    }
                    let first_spec = self.bound_trigger_target(t, first_req.spec);
                    let first_objs = eval::target_options_in_domain(
                        &first_spec,
                        &self.state,
                        t.controller,
                        ctx,
                        None,
                        wall_only,
                    );
                    let first_players = eval::target_player_options_from(
                        &self.state,
                        &first_spec,
                        t.controller,
                        ctx,
                    );
                    return first_objs.into_iter().any(|id| {
                        self.trigger_second_satisfied_for_first(
                            t,
                            second_req,
                            wall_only,
                            Some(id),
                            None,
                        )
                    }) || first_players.into_iter().any(|p| {
                        self.trigger_second_satisfied_for_first(
                            t,
                            second_req,
                            wall_only,
                            None,
                            Some(p),
                        )
                    });
                }
                return req_satisfied(second_req);
            }
            true
        } else if let Some(second_req) = mode.second_targets {
            req_satisfied(second_req)
        } else {
            true
        }
    }

    /// Resolves every queued triggered mana ability at once, off the stack
    /// (CR 605.4a): Badgermole Cub's "whenever you tap a creature for mana,
    /// add an additional {G}" puts its {G} in the pool before the player who
    /// tapped acts again, and before any ordinary trigger of the same batch
    /// is asked about. Returns `true` when one suspended on a choice.
    ///
    /// Which triggers are mana abilities is `AbilityDef::is_triggered_mana_ability`'s
    /// answer (CR 605.1b); every other trigger stays in the queue, in its order.
    pub(super) fn resolve_triggered_mana_abilities(&mut self) -> bool {
        let mut i = 0;
        while i < self.trigger_queue.len() {
            let t = &self.trigger_queue[i];
            let effects = if t.ability_index == baylee_core::ids::AbilityRef::SYNTHETIC {
                if t.event_mana.is_some()
                    && t.synthetic_target.is_none()
                    && t.synthetic_effects
                        .is_some_and(baylee_cards_dsl::mana_rule::could_add_mana)
                {
                    t.synthetic_effects
                } else {
                    None
                }
            } else {
                match self.trigger_abilities(t).get(t.ability_index as usize) {
                    Some(ability @ AbilityDef::Triggered { effects, .. })
                        if ability.is_triggered_mana_ability() =>
                    {
                        Some(*effects)
                    }
                    _ => None,
                }
            };
            let Some(effects) = effects else {
                i += 1;
                continue;
            };
            let Some(t) = self.trigger_queue.remove(i) else {
                break;
            };
            // CR 800.4d, as for every trigger.
            if self.state.has_left(t.controller) {
                continue;
            }
            let mut res = crate::resolve::Resolution {
                event_mana: t.event_mana,
                retarget_left: None,
                loot: None,
                draw_watch: None,
                source: t.source,
                on_stack: t.source,
                controller: t.controller,
                effects: crate::resolve::flatten(effects),
                pc: 0,
                targets: SmallVec::new(),
                second_targets: SmallVec::new(),
                x: None,
                chosen_player: None,
                target_players: baylee_core::ids::SeatSet::new(),
                event_object: t.event_object,
                targeted: false,
                awaiting: None,
                mana_ability: true,
                countered_source: None,
                target_lki: None,
                subject: crate::resolve::SubjectContext::default()
                    .with_event_player(t.event_player)
                    .with_source_version(t.source_version)
                    .with_found(
                        t.found_objects
                            .as_ref()
                            .map_or_default(|f| f.as_ref().clone()),
                    )
                    .with_event_identity(t.event_object.zip(t.event_object_identity).map(
                        |(object, (version, power))| {
                            (baylee_core::ids::DamageSourceRef { object, version }, power)
                        },
                    )),
                text: t.text,
                this_way_from: None,
                coin_stake: None,
            };
            let flow = crate::resolve::run(&mut self.state, &mut res);
            #[cfg(any(test, feature = "test-support"))]
            crate::ability_log::triggered_mana(
                &self.state,
                &self.lookup,
                t.source,
                t.ability_index,
                matches!(flow, crate::resolve::Flow::Complete),
            );
            match flow {
                crate::resolve::Flow::Complete => {}
                crate::resolve::Flow::Enter => {
                    self.resolution = Some(res);
                    self.awaiting_answer = false;
                }
                crate::resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                    return true;
                }
            }
        }
        false
    }

    fn prune_spent_delayed_watches(state: &mut crate::state::GameState, scan_from: usize) {
        let entries = &state.journal.entries()[scan_from..];
        let on_event_consumed: smallvec::SmallVec<[bool; 4]> = if state
            .delayed
            .iter()
            .any(|d| matches!(d.when, crate::state::DelayedWhen::OnEvent { .. }))
        {
            state
                .delayed
                .iter()
                .map(|d| trigger::on_event_watch_consumed(state, entries, d))
                .collect()
        } else {
            smallvec::SmallVec::new()
        };
        let arena = &state.arena;
        let mut watch_idx = 0usize;
        state.delayed.retain(|d| {
            let consumed = on_event_consumed.get(watch_idx).copied().unwrap_or(false);
            watch_idx += 1;
            if consumed {
                return false;
            }
            match d.when {
                crate::state::DelayedWhen::Observed { watch, .. }
                | crate::state::DelayedWhen::ObservedGeneral { watch, .. } => {
                    arena.get(watch.object).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield && o.version == watch.version
                    }) && !d.when.observation_fired(entries, d.controller)
                }
                crate::state::DelayedWhen::DiesOrIsExiled { card, version, .. }
                | crate::state::DelayedWhen::DiesThisTurn { card, version, .. }
                | crate::state::DelayedWhen::DealtDamageThisTurn { card, version, .. }
                | crate::state::DelayedWhen::LeavesBattlefield { card, version, .. }
                | crate::state::DelayedWhen::LeavesBattlefieldThisTurn { card, version, .. }
                | crate::state::DelayedWhen::AttacksAndIsntBlockedThisTurn {
                    card, version, ..
                } => arena.get(card).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.version == version
                }),
                crate::state::DelayedWhen::LeavesBattlefieldOrUntaps {
                    card,
                    version,
                    after,
                } => {
                    arena.get(card).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield && o.version == version
                    }) && !entries.iter().any(|entry| {
                        entry.seq > after
                            && matches!(
                                entry.event,
                                GameEvent::ObjectUntapped { object, .. } if object == card
                            )
                    })
                }
                _ => true,
            }
        });
    }

    /// Remember event-time abilities before a state-based action removes them.
    pub(super) fn queue_new_triggers(&mut self) {
        if self.loop_control.is_breaking() {
            return;
        }
        let scan_from = self.trigger_scan_seq;
        for entry in &self.state.journal.entries()[scan_from as usize..] {
            if let crate::event::GameEvent::CounterChanged {
                object,
                version,
                kind: baylee_cards_dsl::CounterKind::Time,
                old,
                new: 0,
                ..
            } = entry.event
                && old > 0
                && let Some(owner) = self
                    .state
                    .object(object)
                    .filter(|o| {
                        o.zone == crate::zone::Zone::Exile
                            && o.version == version
                            && o.riders.contains(&crate::object::Rider::Suspend)
                    })
                    .map(|o| o.owner)
            {
                self.delayed_queue.push_back((
                    owner,
                    crate::state::DelayedAction::CastFromExileWithoutPaying {
                        card: object,
                        version,
                    },
                ));
            }
        }
        let mut found = trigger::collect(&self.state, &self.lookup, scan_from);
        self.collect_squad_and_offspring_triggers(scan_from, &mut found);
        self.collect_gift_etb_triggers(scan_from, &mut found);
        self.collect_impending_end_step_triggers(scan_from, &mut found);
        crate::trigger::bird_entry::consume(&mut self.state, scan_from);
        self.trigger_scan_seq = self.state.journal.last_seq();
        crate::resolve::linked_reanimation::prune(&mut self.state);
        // The scan has passed every journal entry a departed object could
        // be named in, so nothing can ask about one again and the list is
        // dead weight from here (CR 111.7; `GameState::ceased`). Cleared
        // in exactly the two branches that move `trigger_scan_seq` and
        // never outside them: a clear on a pass that did *not* rescan
        // would throw away what the next one still needs, and no clear at
        // all is an unbounded leak in a deck that makes thousands of
        // tokens. `clear` keeps the capacity, so a token-heavy turn pays
        // for its allocation once.
        //
        // The other two look-back lists are bounded here for the same
        // reason and by the same argument. `ltb_abilities` and
        // `ltb_attachments` drop an entry on the object's *next* move,
        // which is a complete rule for a card and no rule at all for a
        // token: one that has ceased to exist never moves again, so its
        // entry would sit there for the rest of the game. This is the one
        // moment that knows an id is gone for good, and past this scan
        // nothing may ask about it anyway.
        let gone: Vec<baylee_core::ids::ObjectId> =
            self.state.ceased.iter().map(|o| o.id).collect();
        if !gone.is_empty() {
            self.state
                .ltb_abilities
                .retain(|(id, _)| !gone.contains(id));
            self.state
                .ltb_attachments
                .retain(|(id, _)| !gone.contains(id));
        }
        self.state.ceased.clear();
        self.state.damage_deaths.clear();
        self.state.ltb_mana_values.clear();
        // A watch whose object has left the battlefield is spent, whether
        // the scan just fired it or the object left some other way: the
        // object it watched no longer exists (CR 400.7, 603.7b). Here and
        // not in the scan, which reads the state and writes nothing.
        Self::prune_spent_delayed_watches(&mut self.state, scan_from as usize);
        self.trigger_queue.extend(found);
        // State triggers (CR 603.8) read the state, not the journal, so they
        // are looked for on every pass, and an ability that is still waiting
        // here or is on the stack does not trigger again until it has left
        // it. The stack names an ability by its source and index, which is
        // what the queue keys on too.
        let stack = self.state.zones.list(crate::zone::ZoneLocation::Stack);
        let queued = &self.trigger_queue;
        let state = &self.state;
        let in_flight = |source: ObjectId, index: u32| {
            queued
                .iter()
                .any(|t| t.source == source && t.fire_key() == index)
                || stack.iter().any(|id| {
                    state.object(*id).is_some_and(|o| {
                        let Some(loc) = o.ability else {
                            return false;
                        };
                        if loc.source != source {
                            return false;
                        }
                        let stack_key = if let Some(bundle) = &o.own_abilities
                            && bundle.entries().get(loc.index as usize).is_some_and(|e| {
                                e.provenance.index == baylee_core::ids::AbilityRef::SYNTHETIC
                            }) {
                            u32::MAX - 16_384 - (o.rules_acquisition() as u32)
                        } else {
                            loc.index
                        };
                        stack_key == index
                    })
                })
        };
        let standing = trigger::state_triggers(&self.state, &self.lookup, in_flight);
        self.trigger_queue.extend(standing);
        let active = self.state.turn.active.get();
        let seats = self.state.players.len() as u8;
        // APNAP, then timestamps, except that an order a player chose stands
        // (CR 603.3b): the sort is stable, and what it was chosen against
        // comes first, as it was chosen before anything new was found.
        self.trigger_queue.make_contiguous().sort_by_key(|t| {
            let timestamp = if t.in_chosen_order { 0 } else { t.timestamp };
            (
                (t.controller.get() + seats - active) % seats,
                !t.in_chosen_order,
                timestamp,
            )
        });
    }

    /// Queues synthetic ETB triggered abilities for permanents entering the
    /// battlefield with `Rider::SquadPaid` (`CR 702.157a`) or
    /// `Rider::OffspringPaid` (`CR 702.175a`).
    fn collect_squad_and_offspring_triggers(
        &self,
        scan_from: u64,
        out: &mut Vec<crate::trigger::PendingTrigger>,
    ) {
        let all_events = self.state.journal.entries();
        let start = all_events.partition_point(|e| e.seq <= scan_from);
        let events = &all_events[start..];
        for (idx, entry) in events.iter().enumerate() {
            let GameEvent::ZoneChanged {
                object,
                to: crate::zone::Zone::Battlefield,
                ..
            } = entry.event
            else {
                continue;
            };
            let later_moves = events[idx + 1..]
                .iter()
                .filter(|later| {
                    matches!(later.event, GameEvent::ZoneChanged { object: id, .. } if id == object)
                })
                .count();
            let entrant = if later_moves == 0 {
                self.state.object(object)
            } else {
                self.state.object(object).and_then(|current| {
                    self.state.source_object(baylee_core::ids::DamageSourceRef {
                        object,
                        version: current
                            .version
                            .wrapping_sub(u32::try_from(later_moves).unwrap_or(0)),
                    })
                })
            }
            .or_else(|| self.state.ceased.iter().find(|o| o.id == object));
            let Some(entrant) = entrant else {
                continue;
            };
            if entrant.status.contains(crate::object::Status::FACE_DOWN)
                || entrant.characteristics().abilities_lost.is_some()
                || entrant.characteristics().rules_text_lost
            {
                continue;
            }
            let squad_paid = entrant.riders.iter().find_map(|r| match r {
                crate::object::Rider::SquadPaid(n) if *n > 0 => Some(*n),
                _ => None,
            });
            let offspring_paid = entrant
                .riders
                .contains(&crate::object::Rider::OffspringPaid);
            if squad_paid.is_none() && !offspring_paid {
                continue;
            }
            let times = crate::trigger::times_triggered(
                &self.state,
                baylee_cards_dsl::TriggerEventKind::EntersBattlefield,
                object,
            );
            for _ in 0..times {
                if let Some(n) = squad_paid {
                    out.push(synthetic_copy_etb_trigger(
                        object,
                        entrant,
                        SQUAD_ETB_COPIES,
                        Some(u32::from(n)),
                    ));
                }
                if offspring_paid {
                    out.push(synthetic_copy_etb_trigger(
                        object,
                        entrant,
                        OFFSPRING_ETB_COPY,
                        None,
                    ));
                }
            }
        }
    }

    /// Queues synthetic ETB triggered abilities for permanents entering the
    /// battlefield with `Rider::GiftPromisedTo` (`CR 702.174b`).
    fn collect_gift_etb_triggers(
        &self,
        scan_from: u64,
        out: &mut Vec<crate::trigger::PendingTrigger>,
    ) {
        let all_events = self.state.journal.entries();
        let start = all_events.partition_point(|e| e.seq <= scan_from);
        let events = &all_events[start..];
        for (idx, entry) in events.iter().enumerate() {
            let GameEvent::ZoneChanged {
                object,
                to: crate::zone::Zone::Battlefield,
                ..
            } = entry.event
            else {
                continue;
            };
            let later_moves = events[idx + 1..]
                .iter()
                .filter(|later| {
                    matches!(later.event, GameEvent::ZoneChanged { object: id, .. } if id == object)
                })
                .count();
            let entrant = if later_moves == 0 {
                self.state.object(object)
            } else {
                self.state.object(object).and_then(|current| {
                    self.state.source_object(baylee_core::ids::DamageSourceRef {
                        object,
                        version: current
                            .version
                            .wrapping_sub(u32::try_from(later_moves).unwrap_or(0)),
                    })
                })
            }
            .or_else(|| self.state.ceased.iter().find(|o| o.id == object));
            let Some(entrant) = entrant else {
                continue;
            };
            if entrant.status.contains(crate::object::Status::FACE_DOWN)
                || entrant.characteristics().abilities_lost.is_some()
                || entrant.characteristics().rules_text_lost
                || !entrant
                    .riders
                    .iter()
                    .any(|r| matches!(r, crate::object::Rider::GiftPromisedTo(_)))
            {
                continue;
            }
            let Some(kind) = entrant
                .printed_face()
                .and_then(|f| {
                    self.lookup
                        .card(f.card())
                        .and_then(|def| def.faces.get(usize::from(f.face())))
                })
                .and_then(|face| face.gift)
            else {
                continue;
            };
            let already_printed = entrant
                .printed_abilities(&self.lookup)
                .iter()
                .any(|ability| match ability {
                    AbilityDef::Triggered { effects, .. } => {
                        let mut has = false;
                        baylee_cards_dsl::Effect::walk(effects, &mut 0, &mut |e| {
                            has |= matches!(e, baylee_cards_dsl::Effect::DeliverGift(_));
                        });
                        has
                    }
                    _ => false,
                });
            if already_printed {
                continue;
            }
            let times = crate::trigger::times_triggered(
                &self.state,
                baylee_cards_dsl::TriggerEventKind::EntersBattlefield,
                object,
            );
            for _ in 0..times {
                let mut trig = synthetic_copy_etb_trigger(object, entrant, gift_etb_fx(kind), None);
                trig.synthetic_condition = Some(&GIFT_ETB_CONDITION);
                out.push(trig);
            }
        }
    }

    /// Queues synthetic end-step counter-removal triggers for permanents whose
    /// impending cost was paid while they have a time counter (`CR 702.176a`, `CR 603.4`).
    fn collect_impending_end_step_triggers(
        &self,
        scan_from: u64,
        out: &mut Vec<crate::trigger::PendingTrigger>,
    ) {
        let all_events = self.state.journal.entries();
        let start = all_events.partition_point(|e| e.seq <= scan_from);
        for entry in &all_events[start..] {
            let GameEvent::StepChanged {
                step: crate::turn::Step::End,
                active,
                ..
            } = entry.event
            else {
                continue;
            };
            for id in self.state.battlefield_seen() {
                let Some(perm) = self.state.object(id) else {
                    continue;
                };
                if perm.controller != active
                    || perm.status.contains(crate::object::Status::FACE_DOWN)
                    || perm.characteristics().abilities_lost.is_some()
                    || perm.characteristics().rules_text_lost
                    || !perm.riders.contains(&crate::object::Rider::ImpendingPaid)
                    || perm.counters.get(baylee_cards_dsl::CounterKind::Time) < 1
                {
                    continue;
                }
                let has_impending = perm
                    .printed_face()
                    .and_then(|f| {
                        self.lookup
                            .card(f.card())
                            .and_then(|def| def.faces.get(usize::from(f.face())))
                    })
                    .is_some_and(|face| face.impending.is_some());
                if !has_impending {
                    continue;
                }
                let times = crate::trigger::times_triggered(
                    &self.state,
                    baylee_cards_dsl::TriggerEventKind::Any,
                    id,
                );
                for _ in 0..times {
                    let mut trig =
                        synthetic_copy_etb_trigger(id, perm, IMPENDING_END_STEP_FX, None);
                    trig.event_object = None;
                    trig.synthetic_condition = Some(&IMPENDING_END_STEP_CONDITION);
                    out.push(trig);
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)] // the trigger queue processor is a flat state machine
    pub(crate) fn collect_triggers(&mut self) {
        if self.loop_control.is_breaking() {
            // The house rule broke an endless loop in this segment: the
            // abilities that were feeding it do not go back on the stack.
            // Everything already on the stack still resolves, so the loop
            // has happened once and then stops.
            self.trigger_queue.clear();
            self.trigger_scan_seq = self.state.journal.last_seq();
            crate::resolve::linked_reanimation::prune(&mut self.state);
            self.state.ceased.clear();
            self.state.damage_deaths.clear();
            self.state.ltb_mana_values.clear();
            return;
        }
        self.queue_new_triggers();
        if self.resolve_triggered_mana_abilities() {
            return;
        }
        while let Some(t) = self.trigger_queue.front().cloned() {
            // A triggered ability controlled by a player who has left the
            // game isn't put on the stack (CR 800.4d). Every queued trigger
            // comes through here before it is asked about or stacked, one
            // that triggered before they left included.
            if self.state.has_left(t.controller) {
                self.trigger_queue.pop_front();
                continue;
            }
            // "This ability triggers only once each turn." The fire is
            // recorded when the trigger goes on the stack and `ability_fires`
            // is cleared at end of turn, but nothing ever read it back, so the
            // clause did nothing at all.
            if self.trigger_spent(&t) {
                self.trigger_queue.pop_front();
                continue;
            }
            // Their order comes first of all: what is chosen as each goes on
            // the stack is chosen in that order (CR 603.3b).
            if self.ask_trigger_order() {
                return;
            }
            // Modal triggers: offer the mode choice first (CR 603.3c — the
            // controller announces it as the ability goes on the stack, and
            // this is that moment). Only while `chosen_mode` is still empty:
            // the answer writes it back onto this same queue entry and the
            // engine comes round again, at which point the trigger takes the
            // ordinary path below with its mode's own target requirement.
            if t.ability_index != baylee_core::ids::AbilityRef::SYNTHETIC
                && t.chosen_mode.is_none()
                && let Some((modes, restriction)) = self.modal_trigger_modes(&t)
            {
                // "If one of the modes would be illegal (due to an inability
                // to choose legal targets, for example), that mode can't be
                // chosen" — so a mode is offered only if its own requirement
                // can be met. The option's `kind` carries the mode number
                // because this filtering breaks the identity between a
                // position in the list and the mode it names.
                let used = self.modal_trigger_mask(&t, restriction);
                let options: Vec<CastModeDesc> = modes
                    .iter()
                    .enumerate()
                    .filter(|(index, mode)| {
                        (*index >= 64 || used & (1 << index) == 0)
                            && self.mode_is_choosable(&t, mode)
                    })
                    .enumerate()
                    .map(|(position, (mode_index, _))| CastModeDesc {
                        index: position as u32,
                        kind: CastModeKind::Mode(mode_index),
                        cost: baylee_core::mana::ManaCost::ZERO,
                    })
                    .collect();
                if options.is_empty() {
                    // "If no mode is chosen, the ability is removed from the
                    // stack." Nothing to ask and nothing to resolve — but it
                    // was waiting to go there, which is what a cleanup step's
                    // check asks (CR 514.3a).
                    self.cleanup_check_acted();
                    self.trigger_queue.pop_front();
                    continue;
                }
                self.pending_plan = Some(PlanKind::ModalTrigger {
                    source: t.source,
                    ability_index: t.ability_index,
                });
                self.pending = Pending::ChooseCastMode {
                    player: t.controller,
                    object: t.source,
                    options,
                };
                self.awaiting_answer = true;
                return;
            }
            let req = self
                .trigger_abilities(&t)
                .get(t.ability_index as usize)
                .and_then(|a| match a {
                    AbilityDef::Triggered { targets, .. }
                    | AbilityDef::SagaChapter { targets, .. } => *targets,
                    // A modal trigger's target requirement belongs to the
                    // mode, not to the ability: Aether Channeler's bounce
                    // takes one and its Bird token takes none.
                    AbilityDef::ModalTriggered { modes, .. } => t
                        .chosen_mode
                        .and_then(|m| modes.get(m as usize))
                        .and_then(|m| m.targets),
                    _ => None,
                });
            if let Some(req) = req {
                if matches!(req.spec, baylee_cards_dsl::TargetSpec::EventObject) {
                    let targets: SmallVec<[ObjectId; 2]> = t.event_object.into_iter().collect();
                    self.trigger_queue.pop_front();
                    self.record_trigger_fired(&t);
                    self.hand_over_trigger_abilities(&t);
                    self.push_ability_to_stack(t.controller, t.source, t.ability_index, targets);
                    self.set_top_mode(t.chosen_mode);
                    self.bind_top_trigger(&t);
                    if let Some(event_object) = t.event_object {
                        let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                        if let Some(top) = top
                            && let Some(obj) = self.state.object_mut(top)
                        {
                            obj.event_object = Some(event_object);
                        }
                    }
                    if self.ask_trigger_second_target() {
                        return;
                    }
                    continue;
                }
                let wall_only = self.trigger_wall_only(&t);
                // A trigger may point at a player as readily as a spell does
                // ("it deals 1 damage to target opponent"), and "any target"
                // offers both lists at once (CR 115.4). The choice is one
                // choice, so the counts add up.
                let (mut options, mut player_options) =
                    self.trigger_target_options(&t, req.spec, wall_only);
                let second_req = crate::object::ability_second_target_req(
                    self.trigger_abilities(&t),
                    t.ability_index,
                    t.chosen_mode,
                );
                let third_req = crate::object::ability_third_target_req(
                    self.trigger_abilities(&t),
                    t.ability_index,
                    t.chosen_mode,
                );
                if let Some(second_req) = second_req
                    && second_req.min > 0
                {
                    if Self::trigger_second_depends_on_first(second_req.spec) {
                        if req.max == 1 && !req.count_is_x {
                            options.retain(|&id| {
                                self.trigger_second_satisfied_for_first(
                                    &t,
                                    second_req,
                                    wall_only,
                                    Some(id),
                                    None,
                                )
                            });
                            player_options.retain(|&p| {
                                self.trigger_second_satisfied_for_first(
                                    &t,
                                    second_req,
                                    wall_only,
                                    None,
                                    Some(p),
                                )
                            });
                        }
                    } else if !self
                        .trigger_second_satisfied_for_first(&t, second_req, wall_only, None, None)
                    {
                        self.cleanup_check_acted();
                        self.trigger_queue.pop_front();
                        continue;
                    }
                }
                if let Some(third_req) = third_req
                    && third_req.min > 0
                    && !Self::trigger_second_depends_on_first(third_req.spec)
                    && !self
                        .trigger_second_satisfied_for_first(&t, third_req, wall_only, None, None)
                {
                    self.cleanup_check_acted();
                    self.trigger_queue.pop_front();
                    continue;
                }
                let mut later_pools: Vec<eval::DistinctGroupPool> = Vec::new();
                for later_req in [second_req, third_req].into_iter().flatten() {
                    if !Self::trigger_second_depends_on_first(later_req.spec) {
                        let (objs_l, pls_l) =
                            self.trigger_target_options(&t, later_req.spec, wall_only);
                        later_pools.push(eval::DistinctGroupPool {
                            needed: usize::from(later_req.min),
                            distinct_from_prior: later_req.constraint()
                                == Some(
                                    baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups,
                                ),
                            objects: objs_l,
                            players: pls_l,
                        });
                    }
                }
                let chooser = match req.chooser {
                    baylee_cards_dsl::TargetChooser::EventPlayer => t
                        .event_player
                        .or_else(|| {
                            t.event_object
                                .and_then(|id| self.state.last_known_controller(id))
                        })
                        .expect("event-player target chooser requires a captured event player"),
                    baylee_cards_dsl::TargetChooser::Controller
                    | baylee_cards_dsl::TargetChooser::ConstrainedController(_) => t.controller,
                    baylee_cards_dsl::TargetChooser::Opponent => {
                        panic!("opponent chooser requires an activation announcement")
                    }
                };
                let (etb_x, div_total) =
                    match self.trigger_abilities(&t).get(t.ability_index as usize) {
                        Some(AbilityDef::Triggered {
                            trigger, effects, ..
                        }) => {
                            let x = if matches!(trigger, &baylee_cards_dsl::Trigger::ETB) {
                                self.state.object(t.source).map_or(0, |o| o.x_value)
                            } else {
                                0
                            };
                            let ctx = crate::text_changes::RuleContext {
                                source: t.source,
                                text: t.text,
                                resolved_name: None,
                                resolved_subtype: None,
                                reference: t.event_object,
                            };
                            let div = crate::engine::division::effects_divided_spec(
                                effects,
                                &self.state,
                                t.controller,
                                ctx,
                                t.source_version,
                                x,
                            )
                            .map(|(_, total)| total);
                            (x, div)
                        }
                        _ => (0, None),
                    };
                let (min, mut max) = req.bounds(etb_x);
                if let Some(total) = div_total {
                    max = max.min(total);
                }
                if min > max {
                    self.cleanup_check_acted();
                    self.trigger_queue.pop_front();
                    continue;
                }
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                    let bound_spec = self.bound_trigger_target(&t, req.spec);
                    if !eval::one_per_player_satisfied(
                        &self.state,
                        &bound_spec,
                        t.controller,
                        &options,
                        req.min,
                    ) {
                        self.cleanup_check_acted();
                        self.trigger_queue.pop_front();
                        continue;
                    }
                    if max > 0 {
                        let seats =
                            eval::one_per_player_seats(&self.state, &bound_spec, t.controller);
                        let first = super::super::PerOpponent::one_per_player(
                            bound_spec, seats, req.min, chooser,
                        );
                        let (source, ability_index, mode) =
                            (t.source, t.ability_index, t.chosen_mode);
                        if self
                            .ask_next_opponent_with_context(
                                t.controller,
                                crate::text_changes::RuleContext {
                                    source: t.source,
                                    text: t.text,
                                    resolved_name: None,
                                    resolved_subtype: None,
                                    reference: t.event_object,
                                },
                                None,
                                wall_only,
                                first,
                                |asking| PlanKind::Trigger {
                                    source,
                                    ability_index,
                                    mode,
                                    per_opponent: Some(asking),
                                },
                            )
                            .is_none()
                        {
                            return;
                        }
                    }
                } else {
                    if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController)
                    {
                        if let Some(chosen) = self.activation_graveyard.take() {
                            options.retain(|id| {
                                eval::same_controller_group(&self.state, *id) == Some(chosen)
                            });
                        } else {
                            let groups =
                                eval::eligible_same_controller_groups(&self.state, &options, min);
                            match groups.as_slice() {
                                [] => options.clear(),
                                [only] => {
                                    options.retain(|id| {
                                        eval::same_controller_group(&self.state, *id) == Some(*only)
                                    });
                                }
                                _ => {
                                    self.pending_plan = Some(PlanKind::ChooseTriggerTargetGroup {
                                        source: t.source,
                                        ability_index: t.ability_index,
                                        mode: t.chosen_mode,
                                    });
                                    self.pending = Pending::ChoosePlayer {
                                        player: chooser,
                                        options: groups,
                                    };
                                    self.awaiting_answer = true;
                                    return;
                                }
                            }
                        }
                    }
                    eval::retain_distinct_group_options(
                        req,
                        &mut options,
                        &mut player_options,
                        &[],
                        baylee_core::ids::SeatSet::new(),
                        &later_pools,
                    );
                    if later_pools
                        .iter()
                        .any(|p| p.distinct_from_prior && p.needed > 0)
                    {
                        let mut all_pools = Vec::with_capacity(1 + later_pools.len());
                        all_pools.push(eval::DistinctGroupPool {
                            needed: min as usize,
                            distinct_from_prior: false,
                            objects: options.clone(),
                            players: player_options.clone(),
                        });
                        all_pools.extend(later_pools);
                        if !eval::can_complete_target_groups(
                            &[],
                            baylee_core::ids::SeatSet::new(),
                            &all_pools,
                        ) {
                            self.cleanup_check_acted();
                            self.trigger_queue.pop_front();
                            continue;
                        }
                    }
                    let offered = options.len() + player_options.len();
                    if u32::try_from(offered).unwrap_or(u32::MAX) < min {
                        // No legal target: the trigger is removed from the stack
                        // entirely (CR 603.3d). It was put there first, so a
                        // cleanup step's check has found it (CR 514.3a).
                        self.cleanup_check_acted();
                        self.trigger_queue.pop_front();
                        continue;
                    }
                    if let baylee_cards_dsl::TargetSpec::ObjectOfEachOpponent(_) = req.spec {
                        let opponents = self.opponents_in_turn_order(t.controller);
                        let first = super::super::PerOpponent::for_each_opponent(
                            req.spec,
                            opponents,
                            t.controller,
                        );
                        let (source, ability_index, mode) =
                            (t.source, t.ability_index, t.chosen_mode);
                        if self
                            .ask_next_opponent_with_context(
                                t.controller,
                                crate::text_changes::RuleContext {
                                    source: t.source,
                                    text: t.text,
                                    resolved_name: None,
                                    resolved_subtype: None,
                                    reference: t.event_object,
                                },
                                None,
                                self.trigger_wall_only(&t),
                                first,
                                |asking| PlanKind::Trigger {
                                    source,
                                    ability_index,
                                    mode,
                                    per_opponent: Some(asking),
                                },
                            )
                            .is_none()
                        {
                            return;
                        }
                        // Nobody has anything to point at: it stacks targeting
                        // nothing, like any "up to one" with nothing legal.
                    } else if offered > 0 && max > 0 {
                        self.pending_plan = Some(PlanKind::Trigger {
                            source: t.source,
                            ability_index: t.ability_index,
                            mode: t.chosen_mode,
                            per_opponent: None,
                        });
                        let max = max.min(u32::try_from(offered).unwrap_or(u32::MAX));
                        self.pending = Pending::ChooseTargets {
                            player: chooser,
                            options,
                            player_options,
                            min,
                            max,
                            reason: TargetPrompt::Targets,
                        };
                        self.awaiting_answer = true;
                        return;
                    }
                }
            }
            self.trigger_queue.pop_front();
            self.record_trigger_fired(&t);
            // Synthetic triggers with a target requirement (granted
            // triggered abilities): ask for the target first.
            if t.synthetic_effects.is_some()
                && let Some(req) = t.synthetic_target
            {
                let spec = req.spec;
                // "For each opponent, … up to one target creature that player
                // controls": one question per opponent, as the printed path
                // asks it, and the trigger stacks even when nobody had
                // anything to point at — "up to one" can always be targeted
                // legally (CR 603.3d removes only a trigger that cannot).
                if let TargetSpec::ObjectOfEachOpponent(_) = spec {
                    let first = super::super::PerOpponent::for_each_opponent(
                        self.bound_trigger_target(&t, spec),
                        self.opponents_in_turn_order(t.controller),
                        t.controller,
                    );
                    let plan_t = t.clone();
                    let gathered = self.ask_next_opponent_with_context(
                        t.controller,
                        crate::text_changes::RuleContext {
                            source: t.source,
                            text: t.text,
                            resolved_name: None,
                            resolved_subtype: None,
                            reference: None,
                        },
                        None,
                        self.trigger_wall_only(&t),
                        first,
                        |asking| PlanKind::SyntheticTriggerTarget {
                            trigger: plan_t,
                            per_opponent: Some(asking),
                        },
                    );
                    match gathered {
                        None => return,
                        Some(done) => {
                            self.push_synthetic_trigger_with_targets(
                                &t,
                                done.gathered,
                                baylee_core::ids::SeatSet::new(),
                            );
                            continue;
                        }
                    }
                }
                let (mut options, player_options) =
                    self.trigger_target_options(&t, spec, self.trigger_wall_only(&t));
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
                    let bound_spec = self.bound_trigger_target(&t, spec);
                    if !eval::one_per_player_satisfied(
                        &self.state,
                        &bound_spec,
                        t.controller,
                        &options,
                        req.min,
                    ) {
                        continue;
                    }
                    if req.max == 0 {
                        self.push_synthetic_trigger_with_targets(
                            &t,
                            SmallVec::new(),
                            baylee_core::ids::SeatSet::new(),
                        );
                        continue;
                    }
                    let seats = eval::one_per_player_seats(&self.state, &bound_spec, t.controller);
                    let first = super::super::PerOpponent::one_per_player(
                        bound_spec,
                        seats,
                        req.min,
                        t.controller,
                    );
                    let plan_t = t.clone();
                    match self.ask_next_opponent_with_context(
                        t.controller,
                        crate::text_changes::RuleContext {
                            source: t.source,
                            text: t.text,
                            resolved_name: None,
                            resolved_subtype: None,
                            reference: None,
                        },
                        None,
                        self.trigger_wall_only(&t),
                        first,
                        |asking| PlanKind::SyntheticTriggerTarget {
                            trigger: plan_t,
                            per_opponent: Some(asking),
                        },
                    ) {
                        None => return,
                        Some(done) => {
                            if let Some(id) = self.push_synthetic_trigger_with_targets(
                                &t,
                                done.gathered,
                                baylee_core::ids::SeatSet::new(),
                            ) && !done.groups.is_empty()
                            {
                                self.state.bind_one_per_player_groups(id, 0, &done.groups);
                            }
                            continue;
                        }
                    }
                }
                if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
                    let groups = eval::eligible_same_controller_groups(
                        &self.state,
                        &options,
                        u32::from(req.min),
                    );
                    match groups.as_slice() {
                        [] => options.clear(),
                        [only] => {
                            options.retain(|id| {
                                eval::same_controller_group(&self.state, *id) == Some(*only)
                            });
                        }
                        _ => {
                            let controller = t.controller;
                            self.pending_plan =
                                Some(PlanKind::ChooseSyntheticTriggerGroup { trigger: t });
                            self.pending = Pending::ChoosePlayer {
                                player: controller,
                                options: groups,
                            };
                            self.awaiting_answer = true;
                            return;
                        }
                    }
                }
                // Objects and players are answers to the same target choice.
                // Counting only objects loses a mandatory "any target" ability
                // or caps a multi-target one below its legal maximum.
                let offered = options.len() + player_options.len();
                if offered < usize::from(req.min) {
                    // No legal target, so the trigger is removed (CR 603.3d)
                    // — and it is *already* removed: the pop above took this
                    // queue entry off before the synthetic target was asked
                    // about. Popping again here took the trigger queued
                    // behind it as well, unread and unresolved.
                    continue;
                }
                if offered == 0 {
                    // "Up to one" with no legal target: stacks targeting nothing (CR 603.3d).
                    self.push_synthetic_trigger_with_targets(
                        &t,
                        SmallVec::new(),
                        baylee_core::ids::SeatSet::new(),
                    );
                    continue;
                }
                let plan_t = t.clone();
                self.pending_plan = Some(PlanKind::SyntheticTriggerTarget {
                    trigger: plan_t,
                    per_opponent: None,
                });
                let (min, max) = req.bounds(0);
                let max = max.min(u32::try_from(offered).unwrap_or(u32::MAX));
                self.pending = Pending::ChooseTargets {
                    player: t.controller,
                    options,
                    player_options,
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return;
            }
            if let Some(synthetic) = t.synthetic_effects.as_ref() {
                // Synthetic keyword trigger: effects live in the side map.
                //
                // The card is identity and nothing else, so a source that
                // has none still triggers. It used to be read out and the
                // whole branch skipped when it came back empty, which threw
                // the *trigger* away to avoid writing a handle: a token copy
                // of a warded or prowessed creature — Rite of Replication,
                // Progenitor Mimic, Helm of the Host — kept the keyword on
                // its own ability list, was queued a trigger for it, and
                // then quietly never fired one.
                let card = self
                    .state
                    .object(t.source)
                    .or_else(|| self.state.damage_source(t.source, t.source_version))
                    .filter(|o| !o.status.contains(crate::object::Status::FACE_DOWN))
                    .and_then(|o| o.card)
                    .map(|c| c.index);
                let name = self.synthetic_source_name(t.source, t.source_version, synthetic);
                let base = self.state.bare_base(name);
                // The implicit target (prowess: itself; ward: the targeting
                // spell; a granted trigger: none, so its "this" is its
                // source). Not the event object: that is carried below, and
                // a granted trigger's event object is whatever set it off.
                let targets: SmallVec<[ObjectId; 2]> = t.implicit_target.into_iter().collect();
                let id = self.state.arena.insert_with(|id| {
                    GameObject::new_ability_on_stack(
                        id,
                        t.controller,
                        AbilityLoc {
                            card,
                            index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            source: t.source,
                        },
                        targets,
                        base,
                    )
                });
                self.synthetic_fx.insert(id, synthetic);
                self.state
                    .zones
                    .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
                self.bind_top_trigger(&t);
                self.state.capture_linked_references(id, synthetic);
                self.state.journal.record(GameEvent::AbilityTriggered {
                    object: id,
                    source: t.source,
                    ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                    controller: t.controller,
                });
            } else {
                self.hand_over_trigger_abilities(&t);
                let top = self.push_ability_to_stack(
                    t.controller,
                    t.source,
                    t.ability_index,
                    SmallVec::new(),
                );
                self.set_top_mode(t.chosen_mode);
                self.bind_top_trigger(&t);
                // Carry the event object onto the fresh stack object.
                if let Some(event_object) = t.event_object
                    && let Some(obj) = self.state.object_mut(top)
                {
                    obj.event_object = Some(event_object);
                    obj.targets.extend(t.implicit_target);
                }
                if self.discard_empty_trigger_without_effect(top) {
                    continue;
                }
                if self.ask_trigger_second_target() {
                    return;
                }
            }
        }
    }

    /// Whether the top of the stack printed an intervening-`if` clause that
    /// has stopped being true (CR 603.4's second check).
    ///
    /// The clause is asked of the ability's own controller and its own
    /// source, both read off the object on the stack rather than off the
    /// permanent: the two have been separate objects since it was put there
    /// (CR 113.7a), and a source that has left the battlefield in the
    /// meantime is exactly the case the clause has to be able to fail on.
    ///
    /// Only a *triggered* ability has one. An activated ability's condition
    /// is a restriction on activating it, spent once at CR 602.5, and the
    /// synthetic keyword triggers (prowess, ward) print no clause at all.
    ///
    /// Nor is a station threshold one ([`Condition::Station`], CR 722.2a):
    /// it decided whether the permanent had the ability when it triggered,
    /// and the ability on the stack no longer depends on its source.
    ///
    /// [`Condition::Station`]: baylee_cards_dsl::Condition::Station
    pub(super) fn intervening_if_failed(&self, on_stack: ObjectId) -> bool {
        let Some(obj) = self.state.object(on_stack) else {
            return false;
        };
        if obj.kind != ObjectKind::AbilityOnStack {
            return false;
        }
        let Some(loc) = obj.ability else {
            return false;
        };
        let condition = if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
            obj.riders
                .iter()
                .find_map(|rider| match rider {
                    crate::object::Rider::SyntheticCondition(cond) => Some(**cond),
                    _ => None,
                })
                .filter(|c| {
                    !matches!(
                        c,
                        baylee_cards_dsl::Condition::Station(_)
                            | baylee_cards_dsl::Condition::TriggerOnly(_)
                    )
                })
        } else {
            let abilities = obj.own_abilities.as_ref().map_or_else(
                || {
                    self.state
                        .object(loc.source)
                        .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |o| {
                            o.printed_abilities(&self.lookup)
                        })
                },
                crate::copiable_abilities::AbilityDefs::from,
            );
            abilities
                .get(loc.index as usize)
                .and_then(crate::trigger::intervening_clause)
                .filter(|c| {
                    !matches!(
                        c,
                        baylee_cards_dsl::Condition::Station(_)
                            | baylee_cards_dsl::Condition::TriggerOnly(_)
                    )
                })
        };
        if condition.is_none() {
            return false;
        }
        let version = obj
            .riders
            .iter()
            .find_map(|rider| match rider {
                crate::object::Rider::TriggerSourceVersion(version) => Some(*version),
                _ => None,
            })
            .or_else(|| {
                obj.riders.iter().find_map(|rider| match rider {
                    crate::object::Rider::AbilitySourceVersion(version) => Some(*version),
                    _ => None,
                })
            });
        !crate::eval::intervening_if_for_incarnation(
            &self.state,
            condition,
            obj.controller,
            loc.source,
            version,
            crate::resolve::source_attachment_lki(&self.state, on_stack),
            obj.event_object.and_then(|object| {
                obj.riders.iter().find_map(|rider| match rider {
                    crate::object::Rider::EventObjectIdentity(version, _) => {
                        Some(baylee_core::ids::DamageSourceRef {
                            object,
                            version: *version,
                        })
                    }
                    _ => None,
                })
            }),
        )
    }

    pub(in crate::engine) fn resume_synthetic_trigger_group(
        &mut self,
        trigger: crate::trigger::PendingTrigger,
        chosen: PlayerId,
    ) {
        let Some(req) = trigger.synthetic_target else {
            return;
        };
        let (mut options, player_options) =
            self.trigger_target_options(&trigger, req.spec, self.trigger_wall_only(&trigger));
        options.retain(|id| eval::same_controller_group(&self.state, *id) == Some(chosen));
        let offered = options.len() + player_options.len();
        if offered < usize::from(req.min) {
            return;
        }
        if offered == 0 {
            self.push_synthetic_trigger_with_targets(
                &trigger,
                SmallVec::new(),
                baylee_core::ids::SeatSet::new(),
            );
            return;
        }
        let controller = trigger.controller;
        self.pending_plan = Some(PlanKind::SyntheticTriggerTarget {
            trigger,
            per_opponent: None,
        });
        let (min, max) = req.bounds(0);
        let max = max.min(u32::try_from(offered).unwrap_or(u32::MAX));
        self.pending = Pending::ChooseTargets {
            player: controller,
            options,
            player_options,
            min,
            max,
            reason: TargetPrompt::Targets,
        };
        self.awaiting_answer = true;
    }
}

fn is_graveyard_trigger(
    state: &crate::state::GameState,
    trigger: &crate::trigger::PendingTrigger,
) -> bool {
    trigger.abilities.as_ref().is_some_and(|list| {
        let Some(AbilityDef::Triggered { zone, .. } | AbilityDef::ModalTriggered { zone, .. }) =
            list.abilities.get(trigger.ability_index as usize)
        else {
            return false;
        };
        match zone {
            baylee_cards_dsl::TriggerZone::Graveyard
            | baylee_cards_dsl::TriggerZone::Exile
            | baylee_cards_dsl::TriggerZone::Command => true,
            baylee_cards_dsl::TriggerZone::BattlefieldOrCommand => trigger
                .source_version
                .and_then(|version| {
                    state.source_object(baylee_core::ids::DamageSourceRef {
                        object: trigger.source,
                        version,
                    })
                })
                .is_some_and(|o| o.zone == crate::zone::Zone::Command),
            baylee_cards_dsl::TriggerZone::Battlefield => false,
        }
    })
}

/// The engine's own steps, named for the card test crates (`test-support`) and
/// their kit: the steps stay the engine's, these only call them.
#[cfg(any(test, feature = "test-support"))]
impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    #[doc(hidden)]
    pub fn collect_triggers_for_tests(&mut self) {
        self.collect_triggers();
    }
}

static SQUAD_ETB_COPIES: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::CreateTokenCopy {
        target: baylee_cards_dsl::TargetSpec::ThisObject,
        controller: baylee_cards_dsl::PlayerRel::You,
        amount: baylee_cards_dsl::Amount::EventAmount,
        mods: &[],
        tapped: false,
        attacking: false,
        haste_until_eot: false,
        cleanup: baylee_cards_dsl::CopyCleanup::None,
    }];

static OFFSPRING_ETB_MODS: &[baylee_cards_dsl::CopyMod] = &[baylee_cards_dsl::CopyMod::SetPT(1, 1)];

static OFFSPRING_ETB_COPY: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::CreateTokenCopy {
        target: baylee_cards_dsl::TargetSpec::ThisObject,
        controller: baylee_cards_dsl::PlayerRel::You,
        amount: baylee_cards_dsl::Amount::Fixed(1),
        mods: OFFSPRING_ETB_MODS,
        tapped: false,
        attacking: false,
        haste_until_eot: false,
        cleanup: baylee_cards_dsl::CopyCleanup::None,
    }];

static IMPENDING_END_STEP_CLAUSES: &[baylee_cards_dsl::Condition] = &[
    baylee_cards_dsl::Condition::SourceInZone(baylee_cards_dsl::ZoneRef::Battlefield),
    baylee_cards_dsl::Condition::CountersOnSelf(baylee_cards_dsl::CounterKind::Time, 1),
];

static IMPENDING_END_STEP_CONDITION: baylee_cards_dsl::Condition =
    baylee_cards_dsl::Condition::All(IMPENDING_END_STEP_CLAUSES);

static IMPENDING_END_STEP_FX: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::RemoveCounterSelf {
        kind: baylee_cards_dsl::CounterKind::Time,
        n: 1,
    }];

static GIFT_ETB_CONDITION: baylee_cards_dsl::Condition =
    baylee_cards_dsl::Condition::GiftWasPromised;

static GIFT_CARD_FX: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::DeliverGift(
    baylee_cards_dsl::GiftKind::Card,
)];
static GIFT_FOOD_FX: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::DeliverGift(
    baylee_cards_dsl::GiftKind::Food,
)];
static GIFT_TAPPED_FISH_FX: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::DeliverGift(
    baylee_cards_dsl::GiftKind::TappedFish,
)];
static GIFT_TREASURE_FX: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::DeliverGift(
    baylee_cards_dsl::GiftKind::Treasure,
)];
static GIFT_OCTOPUS_FX: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::DeliverGift(
    baylee_cards_dsl::GiftKind::Octopus,
)];
static GIFT_EXTRA_TURN_FX: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::DeliverGift(
    baylee_cards_dsl::GiftKind::ExtraTurn,
)];

const fn gift_etb_fx(kind: baylee_cards_dsl::GiftKind) -> &'static [baylee_cards_dsl::Effect] {
    match kind {
        baylee_cards_dsl::GiftKind::Card => GIFT_CARD_FX,
        baylee_cards_dsl::GiftKind::Food => GIFT_FOOD_FX,
        baylee_cards_dsl::GiftKind::TappedFish => GIFT_TAPPED_FISH_FX,
        baylee_cards_dsl::GiftKind::Treasure => GIFT_TREASURE_FX,
        baylee_cards_dsl::GiftKind::Octopus => GIFT_OCTOPUS_FX,
        baylee_cards_dsl::GiftKind::ExtraTurn => GIFT_EXTRA_TURN_FX,
    }
}

fn synthetic_copy_etb_trigger(
    object: ObjectId,
    entrant: &crate::object::GameObject,
    effects: &'static [baylee_cards_dsl::Effect],
    amount: Option<u32>,
) -> crate::trigger::PendingTrigger {
    crate::trigger::PendingTrigger {
        text: crate::text_changes::TextChangeMap::IDENTITY,
        source_version: Some(entrant.version),
        related_objects: None,
        found_objects: None,
        grantor: None,
        event_subject: None,
        event_ability: None,
        event_object_identity: None,
        counter_source_version: None,
        event_mana: None,
        event_mana_value: None,
        event_tap_snapshot: None,
        event_departure: None,
        event_amount: amount,
        event_player: None,
        source: object,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        controller: entrant.controller,
        timestamp: entrant.timestamp,
        event_object: Some(object),
        implicit_target: None,
        abilities: None,
        synthetic_effects: Some(effects),
        once_per_turn: false,
        synthetic_target: None,
        chosen_mode: None,
        synthetic_condition: None,
        in_chosen_order: false,
    }
}
