//! Triggered abilities: collected, queued, bound and put on the stack.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
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
        }
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
    pub(crate) fn bind_top_trigger(&mut self, trigger: &crate::trigger::PendingTrigger) {
        let Some(top) = self.state.zones.list(ZoneLocation::Stack).last().copied() else {
            return;
        };
        if let Some(reference) = self.state.source_identity(top) {
            self.state.text_changes.set(reference, trigger.text);
        }
        let bound = self.stack_target_req(top).map(|mut req| {
            req.spec = trigger.bind_target(req.spec);
            req
        });
        let graveyard_source = trigger.abilities.as_ref().is_some_and(|list| {
            matches!(
                list.abilities.get(trigger.ability_index as usize),
                Some(
                    AbilityDef::Triggered {
                        zone: baylee_cards_dsl::TriggerZone::Graveyard,
                        ..
                    } | AbilityDef::ModalTriggered {
                        zone: baylee_cards_dsl::TriggerZone::Graveyard,
                        ..
                    }
                )
            )
        });
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
        if let Some(object) = self.state.object_mut(top) {
            object.event_object = trigger.event_object;
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
            if let Some((controller, toughness)) = trigger.event_departure {
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
            if let Some(amount) = trigger.event_damage {
                object
                    .riders
                    .push(crate::object::Rider::EventAmount(amount));
            }
            if let Some(player) = trigger.event_player {
                object
                    .riders
                    .push(crate::object::Rider::EventPlayer(player));
            }
            object.target_req = bound;
        }
    }

    /// The modes of a queued trigger, if it is a modal one.
    fn modal_trigger_modes(
        &self,
        t: &crate::trigger::PendingTrigger,
    ) -> Option<&'static [baylee_cards_dsl::SpellMode]> {
        self.trigger_abilities(t)
            .get(t.ability_index as usize)
            .and_then(|a| match a {
                AbilityDef::ModalTriggered { modes, .. } => Some(*modes),
                _ => None,
            })
    }

    /// Whether `mode` can legally be chosen for `t` right now (CR 603.3c).
    ///
    /// A mode that needs targets it cannot find is off the list. The count
    /// is the same sum the target question below uses — objects and players
    /// together, because "any target" offers both at once (CR 115.4) — so a
    /// mode that survives this test is one the question can actually be
    /// answered for.
    fn mode_is_choosable(&self, t: &crate::trigger::PendingTrigger, mode: &SpellMode) -> bool {
        let Some(req) = mode.targets else {
            return true;
        };
        if req.min == 0 {
            // "Up to one target": choosable with nothing to point at.
            return true;
        }
        if matches!(req.spec, baylee_cards_dsl::TargetSpec::EventObject) {
            return t.event_object.is_some();
        }
        let objects = eval::target_options_with_context(
            &t.bind_target(req.spec),
            &self.state,
            t.controller,
            crate::text_changes::RuleContext {
                source: t.source,
                text: t.text,
            },
        )
        .len();
        let players = eval::target_player_options(&self.state, &req.spec, t.controller).len();
        objects + players >= req.min as usize
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
                None
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
                subject: crate::resolve::SubjectContext::default(),
                text: crate::text_changes::TextChangeMap::IDENTITY,
            };
            let flow = crate::resolve::run(&mut self.state, &mut res);
            #[cfg(test)]
            crate::ability_log::triggered_mana(
                &self.state,
                &self.lookup,
                t.source,
                t.ability_index,
                matches!(flow, crate::resolve::Flow::Complete),
            );
            match flow {
                crate::resolve::Flow::Complete => {}
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

    /// Remember event-time abilities before a state-based action removes them.
    pub(super) fn queue_new_triggers(&mut self) {
        if self.breaking_loop {
            return;
        }
        let found = trigger::collect(&self.state, &self.lookup, self.trigger_scan_seq);
        self.trigger_scan_seq = self.state.journal.last_seq();
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
        let delayed = std::mem::take(&mut self.state.delayed);
        self.state.delayed = delayed
            .into_iter()
            .filter(|d| match d.when {
                crate::state::DelayedWhen::DiesOrIsExiled { card, version, .. }
                | crate::state::DelayedWhen::LeavesBattlefield { card, version, .. } => {
                    self.state.object(card).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield && o.version == version
                    })
                }
                _ => true,
            })
            .collect();
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
                .any(|t| t.source == source && t.ability_index == index)
                || stack.iter().any(|id| {
                    state
                        .object(*id)
                        .and_then(|o| o.ability)
                        .is_some_and(|loc| loc.source == source && loc.index == index)
                })
        };
        let standing = trigger::state_triggers(&self.state, &self.lookup, in_flight);
        self.trigger_queue.extend(standing);
        let active = self.state.turn.active.get();
        let seats = self.state.players.len() as u8;
        self.trigger_queue
            .make_contiguous()
            .sort_by_key(|t| ((t.controller.get() + seats - active) % seats, t.timestamp));
    }

    #[allow(clippy::too_many_lines)] // the trigger queue processor is a flat state machine
    pub(crate) fn collect_triggers(&mut self) {
        if self.breaking_loop {
            // The house rule broke an endless loop in this segment: the
            // abilities that were feeding it do not go back on the stack.
            // Everything already on the stack still resolves, so the loop
            // has happened once and then stops.
            self.trigger_queue.clear();
            self.trigger_scan_seq = self.state.journal.last_seq();
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
            if t.once_per_turn
                && self.state.ability_fires.contains_key(&(
                    baylee_core::ids::DamageSourceRef {
                        object: t.source,
                        version: t.source_version.unwrap_or_else(|| {
                            self.state.object(t.source).expect("trigger source").version
                        }),
                    },
                    t.ability_index,
                ))
            {
                self.trigger_queue.pop_front();
                continue;
            }
            // Modal triggers: offer the mode choice first (CR 603.3c — the
            // controller announces it as the ability goes on the stack, and
            // this is that moment). Only while `chosen_mode` is still empty:
            // the answer writes it back onto this same queue entry and the
            // engine comes round again, at which point the trigger takes the
            // ordinary path below with its mode's own target requirement.
            if t.ability_index != baylee_core::ids::AbilityRef::SYNTHETIC
                && t.chosen_mode.is_none()
                && let Some(modes) = self.modal_trigger_modes(&t)
            {
                // "If one of the modes would be illegal (due to an inability
                // to choose legal targets, for example), that mode can't be
                // chosen" — so a mode is offered only if its own requirement
                // can be met. The option's `kind` carries the mode number
                // because this filtering breaks the identity between a
                // position in the list and the mode it names.
                let options: Vec<CastModeDesc> = modes
                    .iter()
                    .enumerate()
                    .filter(|(_, mode)| self.mode_is_choosable(&t, mode))
                    .enumerate()
                    .map(|(position, (mode_index, _))| CastModeDesc {
                        index: position as u8,
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
                    if t.once_per_turn {
                        self.state.ability_fires.insert(
                            (
                                baylee_core::ids::DamageSourceRef {
                                    object: t.source,
                                    version: t.source_version.unwrap_or_else(|| {
                                        self.state.object(t.source).expect("trigger source").version
                                    }),
                                },
                                t.ability_index,
                            ),
                            1,
                        );
                    }
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
                let options = eval::target_options_with_context(
                    &t.bind_target(req.spec),
                    &self.state,
                    t.controller,
                    crate::text_changes::RuleContext {
                        source: t.source,
                        text: t.text,
                    },
                );
                // A trigger may point at a player as readily as a spell does
                // ("it deals 1 damage to target opponent"), and "any target"
                // offers both lists at once (CR 115.4). The choice is one
                // choice, so the counts add up.
                let player_options =
                    eval::target_player_options(&self.state, &req.spec, t.controller);
                let offered = options.len() + player_options.len();
                if offered < req.min as usize {
                    // No legal target: the trigger is removed from the stack
                    // entirely (CR 603.3d). It was put there first, so a
                    // cleanup step's check has found it (CR 514.3a).
                    self.cleanup_check_acted();
                    self.trigger_queue.pop_front();
                    continue;
                }
                // Only when there is something to decide. `offered` reaching
                // `req.min` above leaves one case behind: a trigger that may
                // decline (`min` 0, "up to one target") with nothing legal to
                // point at. It still goes on the stack — CR 603.3d removes a
                // trigger that *cannot* be targeted legally, and one that
                // needs no target can always be — but with no targets and no
                // question, because the only answer is the empty list.
                //
                // Publishing it anyway was a stop the player could not
                // influence: a Skyclave Apparition entering against a board
                // of nothing but lands offered `ChooseTargets { options: [],
                // min: 0, max: 0 }` and waited. Falling out of this block
                // instead reaches the same tail an untargeted trigger takes,
                // which is what stacks it.
                //
                // `WizardStage::Targets` is the twin of this branch on the
                // spell side, and it needs both halves for the same reason
                // this one does: a wizard reads `max` off the requirement
                // before it knows the options, so Eerie Interlude's "any
                // number of target creatures you control" (`max` 255) walks
                // past the `max == 0` test and is stopped by the empty
                // option list beside it.
                if let baylee_cards_dsl::TargetSpec::ObjectOfEachOpponent(_) = req.spec {
                    let opponents = self.opponents_in_turn_order(t.controller);
                    let first = super::super::PerOpponent {
                        spec: req.spec,
                        gathered: SmallVec::new(),
                        remaining: opponents,
                    };
                    let (source, ability_index, mode) = (t.source, t.ability_index, t.chosen_mode);
                    if self
                        .ask_next_opponent_with_context(
                            t.controller,
                            crate::text_changes::RuleContext {
                                source: t.source,
                                text: t.text,
                            },
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
                } else if offered > 0 {
                    self.pending_plan = Some(PlanKind::Trigger {
                        source: t.source,
                        ability_index: t.ability_index,
                        mode: t.chosen_mode,
                        per_opponent: None,
                    });
                    // Saturated, never truncated: a board of 256 legal
                    // targets wrapped `offered as u8` to 0 and asked for
                    // one target of at most none (l29 game 1930). `offered`
                    // reaching `req.min` is checked above, so the count can
                    // only cap the maximum, never undercut the minimum.
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
            }
            self.trigger_queue.pop_front();
            if t.once_per_turn {
                self.state.ability_fires.insert(
                    (
                        baylee_core::ids::DamageSourceRef {
                            object: t.source,
                            version: t.source_version.unwrap_or_else(|| {
                                self.state.object(t.source).expect("trigger source").version
                            }),
                        },
                        t.ability_index,
                    ),
                    1,
                );
            }
            // Synthetic triggers with a target requirement (granted
            // triggered abilities): ask for the target first.
            if t.synthetic_effects.is_some()
                && let Some(spec) = t.synthetic_target
            {
                // "For each opponent, … up to one target creature that player
                // controls": one question per opponent, as the printed path
                // asks it, and the trigger stacks even when nobody had
                // anything to point at — "up to one" can always be targeted
                // legally (CR 603.3d removes only a trigger that cannot).
                if let TargetSpec::ObjectOfEachOpponent(_) = spec {
                    let first = super::super::PerOpponent {
                        spec: t.bind_target(spec),
                        gathered: SmallVec::new(),
                        remaining: self.opponents_in_turn_order(t.controller),
                    };
                    let plan_t = t.clone();
                    let gathered = self.ask_next_opponent_with_context(
                        t.controller,
                        crate::text_changes::RuleContext {
                            source: t.source,
                            text: t.text,
                        },
                        first,
                        |asking| PlanKind::SyntheticTriggerTarget {
                            trigger: plan_t,
                            per_opponent: Some(asking),
                        },
                    );
                    match gathered {
                        None => return,
                        Some(all) => {
                            self.push_synthetic_trigger_with_targets(&t, all);
                            continue;
                        }
                    }
                }
                let options = eval::target_options_with_context(
                    &t.bind_target(spec),
                    &self.state,
                    t.controller,
                    crate::text_changes::RuleContext {
                        source: t.source,
                        text: t.text,
                    },
                );
                if options.is_empty() {
                    // No legal target, so the trigger is removed (CR 603.3d)
                    // — and it is *already* removed: the pop above took this
                    // queue entry off before the synthetic target was asked
                    // about. Popping again here took the trigger queued
                    // behind it as well, unread and unresolved.
                    continue;
                }
                let plan_t = t.clone();
                self.pending_plan = Some(PlanKind::SyntheticTriggerTarget {
                    trigger: plan_t,
                    per_opponent: None,
                });
                self.pending = Pending::ChooseTargets {
                    player: t.controller,
                    options,
                    player_options: Vec::new(),
                    min: 1,
                    max: 1,
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
                    .filter(|o| !o.status.contains(crate::object::Status::FACE_DOWN))
                    .and_then(|o| o.card)
                    .map(|c| c.index);
                let name = self.synthetic_source_name(t.source);
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
                self.push_ability_to_stack(
                    t.controller,
                    t.source,
                    t.ability_index,
                    SmallVec::new(),
                );
                self.set_top_mode(t.chosen_mode);
                self.bind_top_trigger(&t);
                // Carry the event object onto the fresh stack object.
                if let Some(event_object) = t.event_object {
                    let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                    if let Some(top) = top
                        && let Some(obj) = self.state.object_mut(top)
                    {
                        obj.event_object = Some(event_object);
                        obj.targets.extend(t.implicit_target);
                    }
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
    /// Nor is a station threshold one ([`Condition::Station`], CR 721.2a):
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
        if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
            return false;
        }
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
        let condition = match abilities.get(loc.index as usize) {
            Some(
                AbilityDef::Triggered { condition, .. }
                | AbilityDef::ModalTriggered { condition, .. },
            ) => *condition,
            _ => None,
        }
        .filter(|c| !matches!(c, baylee_cards_dsl::Condition::Station(_)));
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
        )
    }
}
