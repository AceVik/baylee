//! A turn's beginning, the skips offered, sagas, and the delayed triggers each step queues.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    // ------------------------------------------------------------ combat

    pub(crate) fn begin_turn(&mut self, first_turn: bool) {
        // CR 502.2 asks the *previous* turn how many spells its active
        // player cast, and this turn's untap step is the first place that
        // can ask — by which time `per_turn.reset()` below has zeroed the
        // count and the swap under it has moved `turn.active` on. So the
        // pair is taken here, at the one instant both are still true.
        self.state.previous_turn = (!first_turn).then(|| {
            let active = self.state.turn.active;
            let spells = &self.state.per_turn.spells_cast;
            crate::state::PreviousTurn {
                active,
                spells_cast: spells[active.get() as usize],
                spells_by_all: spells.iter().sum(),
                most_by_one: spells.iter().copied().max().unwrap_or(0),
            }
        });
        if !first_turn {
            // Turn order goes on from the last normal turn that ended or
            // was skipped (CR 614.10).
            let after = match self.cleanup {
                Cleanup::Ended { after } => after,
                _ => self.order_after(),
            };
            self.cleanup = Cleanup::Due;
            // Extra turns (CR 500.7) come directly after the turn they were
            // added after, before the normal successor; that successor is
            // still the one `after`'s turn would have had.
            let extra = self.state.extra_turns.pop_front();
            self.state.resume_after = extra.map(|_| after);
            let next = extra.unwrap_or_else(|| self.next_alive_after(after));
            self.state.turn.active = next;
            self.state.turn.number += 1;
            // "When that creature dies this turn" watches no other turn.
            self.state
                .delayed
                .retain(|d| !matches!(d.when, crate::state::DelayedWhen::DiesThisTurn { .. }));
        }
        // Only the seat whose turn this is: summoning sickness is measured
        // against *their* most recent turn (CR 302.6), so an opponent's
        // creature stays asleep while this turn runs.
        let stamp = self.state.timestamp;
        let seat = self.state.turn.active.get() as usize;
        self.state.players[seat].turn_start_timestamp = stamp;
        // Capture the turn boundary itself, before untap-step actions or
        // effects ending as this turn begins change the battlefield.
        // Always record it: an ability may arrive later and ask about it.
        let untapped_lands = self
            .state
            .battlefield_seen()
            .filter_map(|id| self.state.object(id))
            .filter(|o| {
                o.controller == self.state.turn.active
                    && o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::LAND)
                    && !o.status.contains(Status::TAPPED)
            })
            .count();
        // "Until your next turn" effects end as their controller's turn
        // begins (Elspeth's flying, Teferi's sorcery-flash).
        let new_active = self.state.turn.active;
        self.state.effects.remove_where(|fx| {
            matches!(fx.duration, baylee_cards_dsl::Duration::UntilYourNextTurn)
                && fx.controller == new_active
        });
        self.state.per_turn.reset();
        self.state.per_turn.untapped_lands_at_start =
            u32::try_from(untapped_lands).unwrap_or(u32::MAX);
        self.state.ability_fires.clear();
        self.loyalty_used_this_turn.clear();
        let active = self.state.turn.active;
        self.state.players[active.get() as usize].lands_played_this_turn = 0;
        self.state.turn.phase = Phase::Beginning;
        self.state.turn.step = Step::Untap;
        self.combat_declared = CombatDeclared::None;
        self.state.journal.record(GameEvent::TurnStarted {
            number: self.state.turn.number,
            active,
        });
        // What a skipped turn left to do is "the first thing that happens
        // during the next step, phase, or turn to actually occur"
        // (CR 614.10b): this one. A permanent that has since left the
        // battlefield is a new object (CR 400.7) and is not untapped.
        for (id, version) in std::mem::take(&mut self.state.skip_followups) {
            if self
                .state
                .object(id)
                .is_some_and(|o| o.version == version && o.zone == Zone::Battlefield)
                && self.state.set_tapped(id, false)
            {
                // Journaled like every effect's untap, so "whenever ...
                // becomes untapped" sees it (`resolve::untap`).
                self.state.journal.record(GameEvent::ObjectUntapped {
                    object: id,
                    cause: crate::event::Cause::Effect,
                });
            }
        }
    }

    /// The player whose turn the normal order goes on from: the current
    /// turn's, or, during an extra turn, the normal turn it was added after
    /// (CR 500.7). In a duel an extra turn taken in the opponent's turn is
    /// followed by the extra turn's own player again, not by the opponent.
    pub(crate) fn order_after(&self) -> PlayerId {
        self.state.resume_after.unwrap_or(self.state.turn.active)
    }

    /// The turn after `after`'s would begin: an extra turn if one is queued
    /// (CR 500.7), else the next player's. "An effect that causes a player
    /// to skip an event, step, phase, or turn is a replacement effect"
    /// (CR 614.10), so the player whose turn it would be is first offered
    /// every skip they control; only then does it begin. Returns `true`
    /// when a question was asked.
    pub(crate) fn begin_next_turn(&mut self, after: PlayerId) -> bool {
        self.cleanup = Cleanup::Ended { after };
        let next = self
            .state
            .extra_turns
            .front()
            .copied()
            .unwrap_or_else(|| self.next_alive_after(after));
        if self.offer_turn_skip(next, Vec::new()) {
            return true;
        }
        self.begin_turn(false);
        false
    }

    /// Asks `player` about the first skip replacement they control that
    /// applies to the turn they would begin and that they have not
    /// declined for it: a tapped permanent with
    /// `ReplacementRule::SkipTurnToUntapSelf` (Time Vault). Each one gets a
    /// single opportunity at the event (CR 614.5), so `declined` carries
    /// those already answered no. Returns `false` when none is left.
    pub(crate) fn offer_turn_skip(&mut self, player: PlayerId, declined: Vec<ObjectId>) -> bool {
        use baylee_cards_dsl::{AbilityDef, ReplacementRule};
        if self.state.players[usize::from(player.get())].has_lost() {
            return false;
        }
        let state = &self.state;
        let Some(source) = state
            .replacement_rules
            .iter()
            .filter(|r| r.rule == ReplacementRule::SkipTurnToUntapSelf && r.controller == player)
            .map(|r| r.source)
            .filter(|s| !declined.contains(s))
            .find(|s| {
                state.object(*s).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && o.controller == player
                        && o.status.contains(Status::TAPPED)
                        && !o.status.contains(Status::PHASED_OUT)
                })
            })
        else {
            return false;
        };
        let ability = state.printed_ability_list(source).and_then(|list| {
            let index = list.abilities.iter().position(|a| {
                matches!(
                    a,
                    AbilityDef::Replacement(ReplacementRule::SkipTurnToUntapSelf)
                )
            })?;
            list.entry(index)?.provenance.ability_ref()
        });
        self.pending_plan = Some(PlanKind::SkipTurn { source, declined });
        self.pending = Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::SkipTurn { source },
            source: ability,
        };
        self.awaiting_answer = true;
        true
    }

    /// Asks about the first waiting draw Island Sanctuary could replace
    /// (`GameState::draws_to_offer`), after making every draw ahead of it
    /// that nobody may skip. The turn-based draw (CR 504.1) comes here, and
    /// so does anything queued outside a resolution. Returns `true` when a
    /// question was asked.
    pub(crate) fn offer_queued_draw(&mut self) -> bool {
        let Some((player, source)) = self.state.next_draw_offer() else {
            return false;
        };
        self.ask_draw_skip(player, source, Vec::new());
        true
    }

    /// Puts the question about the front waiting draw on the table.
    pub(crate) fn ask_draw_skip(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        declined: Vec<ObjectId>,
    ) {
        self.pending_plan = Some(PlanKind::SkipDraw { source, declined });
        self.pending = self.state.draw_offer_question(player, source);
        self.awaiting_answer = true;
    }

    /// Queues delayed actions (suspend finishes, pact payments) when the
    /// upkeep step begins.
    pub(crate) fn queue_upkeep_delayed(&mut self) {
        let active = self.state.turn.active;
        // Suspend countdown: decrement time counters, cast at zero.
        let suspended: Vec<ObjectId> = self
            .state
            .zones
            .list(ZoneLocation::Exile(active))
            .iter()
            .filter(|id| {
                self.state.object(**id).is_some_and(|o| {
                    o.riders
                        .iter()
                        .any(|r| matches!(r, crate::object::Rider::Suspend))
                })
            })
            .copied()
            .collect();
        for card in suspended {
            let remaining = self
                .state
                .object(card)
                .map_or(0, |o| o.counters.get(baylee_cards_dsl::CounterKind::Time));
            if remaining <= 1 {
                // Last counter removed: cast it without paying (CR 702.62a).
                if let Some(obj) = self.state.object_mut(card) {
                    obj.counters.set(baylee_cards_dsl::CounterKind::Time, 0);
                }
                self.delayed_queue.push_back((
                    active,
                    crate::state::DelayedAction::CastFromExileWithoutPaying {
                        card,
                        version: self.state.object(card).map_or(0, |o| o.version),
                    },
                ));
            } else if let Some(obj) = self.state.object_mut(card) {
                obj.counters
                    .set(baylee_cards_dsl::CounterKind::Time, remaining - 1);
                self.state.journal.record(GameEvent::CounterChanged {
                    object: card,
                    kind: baylee_cards_dsl::CounterKind::Time,
                    old: remaining,
                    new: remaining - 1,
                });
            }
        }
        // Delayed triggers registered for this upkeep.
        let mut i = 0;
        while i < self.state.delayed.len() {
            let fire = match self.state.delayed[i].when {
                crate::state::DelayedWhen::NextUpkeep | crate::state::DelayedWhen::EachUpkeep => {
                    self.state.delayed[i].controller == active
                }
                crate::state::DelayedWhen::NextUpkeepOfAnyone => true,
                _ => false,
            };
            if fire {
                let trigger = if self.state.delayed[i].when == crate::state::DelayedWhen::EachUpkeep
                {
                    let trigger = self.state.delayed[i].clone();
                    i += 1;
                    trigger
                } else {
                    self.state.delayed.remove(i)
                };
                // A payment waits for this upkeep's priority window; see
                // `upkeep_payments`. Everything else does what it does now.
                if matches!(
                    trigger.action,
                    crate::state::DelayedAction::PayCostOrLose { .. }
                        | crate::state::DelayedAction::PayCostOrSacrifice { .. }
                ) {
                    self.upkeep_payments.push_back(trigger.action);
                } else {
                    self.delayed_queue
                        .push_back((trigger.controller, trigger.action));
                }
            } else {
                i += 1;
            }
        }
    }

    /// The name a synthetic ability on the stack goes by: its source's, or,
    /// for the monarch's abilities, which have none (CR 724.2), the
    /// designation's. Falling back to name `0` named them after whatever
    /// card was interned first.
    pub(super) fn synthetic_source_name(&mut self, source: ObjectId) -> NameRef {
        if source == ObjectId::NO_SOURCE {
            return self.state.names.intern("Monarch");
        }
        self.state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name)
    }

    /// Pushes a synthetic trigger (prowess, ward, granted abilities, a
    /// reflexive one) with explicitly chosen targets onto the stack.
    pub(crate) fn push_synthetic_trigger_with_targets(
        &mut self,
        t: &crate::trigger::PendingTrigger,
        targets: SmallVec<[ObjectId; 2]>,
    ) {
        let Some(synthetic) = t.synthetic_effects else {
            return;
        };
        // Identity, not a precondition — see the sibling branch in
        // `stack_triggers`. A cardless source triggers like any other.
        let card = self
            .state
            .object(t.source)
            .filter(|o| !o.status.contains(crate::object::Status::FACE_DOWN))
            .and_then(|o| o.card)
            .map(|c| c.index);
        let name = self.synthetic_source_name(t.source);
        let base = self.state.bare_base(name);
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(
                id,
                t.controller,
                AbilityLoc {
                    card,
                    index: baylee_core::ids::AbilityRef::SYNTHETIC,
                    source: t.source,
                },
                targets,
                base,
            );
            // What the targets were chosen against. CR 608.2b re-checks
            // them against it at resolution, as it does a spell's.
            obj.target_req = t.synthetic_target.map(synthetic_target_req);
            obj
        });
        self.synthetic_fx.insert(id, synthetic);
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.bind_top_trigger(t);
        self.state.capture_linked_references(id, synthetic);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source: t.source,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            controller: t.controller,
        });
    }

    /// Queues delayed actions that fire at the first main phase (Mana
    /// Drain's mana).
    pub(crate) fn queue_first_main_delayed(&mut self) {
        let active = self.state.turn.active;
        let mut i = 0;
        while i < self.state.delayed.len() {
            let fire = matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::NextFirstMain
            ) && self.state.delayed[i].controller == active;
            if fire {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue
                    .push_back((trigger.controller, trigger.action));
            } else {
                i += 1;
            }
        }
    }

    /// Whether `id` is a Saga.
    ///
    /// "Has a chapter ability" rather than "has the Saga subtype", because
    /// that is already how CR 714.4's sacrifice is recognised further down
    /// this file, and two sites answering the same question two ways is how
    /// a permanent gets a counter nobody then reads. It is also the reading
    /// that survives a copy: [`Object::abilities`] follows `own_abilities`,
    /// so a permanent that has become a copy of a Saga answers with the
    /// chapters it has become.
    pub(super) fn is_saga(&self, id: ObjectId) -> bool {
        self.state.object(id).is_some_and(|o| {
            o.abilities(&self.lookup)
                .iter()
                .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::SagaChapter { .. }))
        })
    }

    /// Whether a chapter ability of `source` has triggered and not yet left
    /// the stack — the clause that holds CR 714.4's sacrifice back.
    ///
    /// An ability on the stack carries the list its source had when it
    /// triggered (`own_abilities`), so the index in its [`AbilityLoc`] is
    /// read against *that* and not against the permanent as it stands now:
    /// the question is what this ability is, and it stopped being the
    /// permanent's business the moment it went on the stack (CR 113.7a).
    /// An ability with no such entry — a synthetic one, whose index is
    /// [`baylee_core::ids::AbilityRef::SYNTHETIC`] — is not a chapter.
    ///
    /// [`AbilityLoc`]: crate::object::AbilityLoc
    fn a_chapter_of_it_is_still_on_the_stack(&self, source: ObjectId) -> bool {
        self.state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter_map(|id| self.state.object(*id))
            .filter(|o| o.kind == ObjectKind::AbilityOnStack)
            .any(|o| {
                o.ability.is_some_and(|loc| {
                    loc.source == source
                        && o.printed_abilities(&self.lookup)
                            .get(loc.index as usize)
                            .is_some_and(|a| {
                                matches!(a, baylee_cards_dsl::AbilityDef::SagaChapter { .. })
                            })
                })
            })
    }

    /// Whether ability `index` of `source` is one of its chapters.
    ///
    /// A synthetic index ([`baylee_core::ids::AbilityRef::SYNTHETIC`]) names
    /// no entry in the list and is never a chapter.
    fn is_a_chapter_of(&self, source: ObjectId, index: u32) -> bool {
        index != baylee_core::ids::AbilityRef::SYNTHETIC
            && self.state.object(source).is_some_and(|o| {
                matches!(
                    o.printed_abilities(&self.lookup).get(index as usize),
                    Some(baylee_cards_dsl::AbilityDef::SagaChapter { .. })
                )
            })
    }

    /// CR 714.4's second clause in full: a chapter of `source` that **has
    /// triggered** and has not yet left the stack.
    ///
    /// "Has triggered" is true from the moment the lore counter lands, and
    /// the stack is the *second* of two places such an ability can be. A
    /// chapter is not discovered from the journal the way other triggers
    /// are — [`Self::queue_saga_chapters`] pushes it straight into
    /// `trigger_queue` as the counter is placed, because the trigger is
    /// "the count crossed this number" and no event says that — so it sits
    /// in the queue until `collect_triggers` stacks it.
    ///
    /// Asking the stack alone therefore answers "nothing is waiting" about
    /// a chapter one step away from it, and sacrifices the Saga out from
    /// underneath its own last chapter. That is what the two existing saga
    /// tests said when this check was first written that way.
    fn a_chapter_of_it_has_triggered(&self, source: ObjectId) -> bool {
        self.a_chapter_of_it_is_still_on_the_stack(source)
            || self
                .trigger_queue
                .iter()
                .any(|t| t.source == source && self.is_a_chapter_of(source, t.ability_index))
    }

    /// One simultaneous SBA event, including Saga sacrifices and every
    /// answered legend choice, followed by its graveyard-order choices.
    pub(super) fn run_state_based_actions(&mut self) -> sba::SbaOutcome {
        let finished_sagas = self.finished_sagas();
        let since = self.state.journal.last_seq();
        let outcome = sba::run_with_sagas(&mut self.state, &self.lookup, &finished_sagas);
        crate::graveyard_order::capture(&mut self.state, since);
        outcome
    }

    /// CR 714.4: a Saga whose lore counters cover its final chapter, and
    /// which no chapter of its own has triggered and not yet left the stack,
    /// is sacrificed by its controller.
    ///
    /// It is a state-based action and the rule says so in as many words, so
    /// what matters is that it is asked about a *state* and not about an
    /// event. This lived in [`Self::finish_resolution`] instead, on the way
    /// past a chapter that had just resolved, which answers the same in
    /// every game where every chapter resolves — and a chapter can leave the
    /// stack without resolving. Countered by Tishana's Tidebinder, the last
    /// chapter took the sacrifice with it and the Saga stayed on the
    /// battlefield for the rest of the game.
    ///
    /// "Has a chapter ability" is read through [`Object::abilities`] rather
    /// than off the printed face, so a permanent that has *become* a copy of
    /// a Saga answers with the chapters it has become — the same reading
    /// [`Self::is_saga`] uses, and the reason the rule's own "with one or
    /// more chapter abilities" needs no subtype here.
    ///
    /// The applicable Saga sacrifices, planned before the simultaneous
    /// SBA pass mutates any object (CR 704.3).
    ///
    /// [`Object::abilities`]: crate::object::GameObject::abilities
    fn finished_sagas(&self) -> Vec<ObjectId> {
        let mut finished_sagas = Vec::new();
        // A phased-out Saga is not sacrificed (CR 702.26b).
        for id in self.state.battlefield_view() {
            let finished = self.state.object(id).is_some_and(|o| {
                // Every Saga on the battlefield has at least one lore
                // counter (CR 714.3a), so this is the whole step for a
                // board with no Saga on it and costs one field read per
                // permanent rather than an abilities lookup.
                if o.counters.get(baylee_cards_dsl::CounterKind::Lore) == 0 {
                    return false;
                }
                let max = o
                    .abilities(&self.lookup)
                    .iter()
                    .filter_map(|a| match a {
                        baylee_cards_dsl::AbilityDef::SagaChapter { chapter, .. } => Some(*chapter),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                max > 0 && o.counters.get(baylee_cards_dsl::CounterKind::Lore) >= u16::from(max)
            });
            // The second half of the sentence, asked only once the counters
            // say yes to the first: a Saga can arrive on several counters at
            // once and owe several chapters, and the stack is
            // last-in-first-out, so the *highest* one resolves first and
            // would otherwise carry the Saga to the graveyard with its
            // earlier chapters still waiting to resolve on a permanent that
            // is no longer there.
            if !finished || self.a_chapter_of_it_has_triggered(id) {
                continue;
            }
            finished_sagas.push(id);
        }
        finished_sagas
    }

    /// Queues every chapter ability the lore count just crossed, and says
    /// whether it queued any.
    ///
    /// CR 714.2b writes a chapter symbol out in full as "when one or more
    /// lore counters are put onto this Saga, **if the number of lore
    /// counters on it was less than N and became at least N**, [effect]".
    /// So the question is a window, `old < N <= new`, and not "which chapter
    /// is next". Both callers used to ask the second question — one matched
    /// `chapter: 1` and the other `lore + 1` — and a Saga that took two
    /// counters at once therefore ran one chapter and dropped the other on
    /// the floor. A Doubling Season is the way into that today; any effect
    /// that adds two lore counters would be another.
    ///
    /// They are queued low chapter first and the stack is
    /// last-in-first-out, so chapter II resolves before chapter I. That is
    /// legal — CR 603.3b lets a player put simultaneous triggers they
    /// control on the stack in any order they choose, and this engine has
    /// no `Pending` for that choice yet — and it is a consequence of the
    /// queue's order rather than a decision, so nothing here should be read
    /// as preferring it.
    pub(super) fn queue_saga_chapters(
        &mut self,
        id: ObjectId,
        controller: PlayerId,
        old: u16,
        new: u16,
        timestamp: u64,
    ) -> bool {
        let Some(object) = self.state.object(id) else {
            return false;
        };
        let hits: Vec<u32> = object
            .abilities(&self.lookup)
            .iter()
            .enumerate()
            .filter_map(|(i, a)| match a {
                baylee_cards_dsl::AbilityDef::SagaChapter { chapter, .. }
                    if old < u16::from(*chapter) && u16::from(*chapter) <= new =>
                {
                    Some(i as u32)
                }
                _ => None,
            })
            .collect();
        for ability_index in &hits {
            self.trigger_queue
                .push_back(crate::trigger::PendingTrigger {
                    text: crate::text_changes::TextChangeMap::IDENTITY,
                    source_version: None,
                    event_object_identity: None,
                    counter_source_version: None,
                    event_mana: None,
                    event_mana_value: None,
                    event_departure: None,
                    event_damage: None,
                    event_player: None,
                    source: id,
                    ability_index: *ability_index,
                    abilities: None,
                    controller,
                    timestamp,
                    event_object: None,
                    implicit_target: None,
                    synthetic_effects: None,
                    once_per_turn: false,
                    synthetic_target: None,
                    chosen_mode: None,
                });
        }
        !hits.is_empty()
    }

    /// As the active player's precombat main phase begins, each Saga they
    /// control takes a lore counter and every chapter that count crosses
    /// triggers (CR 505.4, CR 714.3b, CR 714.2b).
    ///
    /// This counter is **not** doubled, which is the whole reason
    /// [`crate::replacement::record_counters`] exists beside `put_counters`:
    /// CR 614.16 reaches what a resolving spell or ability's effect places
    /// and what another replacement effect places, and a turn-based action
    /// is neither. The counter a Saga takes as it *enters* is a replacement
    /// effect and is doubled — the two halves of CR 714.3 land on opposite
    /// sides of the same rule.
    pub(crate) fn saga_precombat_main_counters(&mut self) {
        let active = self.state.turn.active;
        for id in self.state.battlefield_view() {
            let Some(old) = self
                .state
                .object(id)
                .filter(|o| o.controller == active)
                .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::Lore))
            else {
                continue;
            };
            if !self.is_saga(id) {
                continue;
            }
            let ts = self.state.next_timestamp();
            crate::replacement::record_counters(
                &mut self.state,
                id,
                baylee_cards_dsl::CounterKind::Lore,
                1,
            );
            if let Some(obj) = self.state.object_mut(id) {
                obj.timestamp = ts;
            }
            self.queue_saga_chapters(id, active, old, old + 1, ts);
        }
    }

    /// Queues delayed actions that fire at the beginning of the end step
    /// (Venser +2's returned permanents) — fires for ANY controller, not
    /// just the active player.
    pub(crate) fn queue_end_step_delayed(&mut self) {
        self.queue_delayed_at(crate::state::DelayedWhen::NextEndStep);
    }

    /// Queues the delayed triggers that wait for "end of combat": they
    /// trigger as the end of combat step begins (CR 511.2), for any
    /// controller.
    pub(crate) fn queue_end_of_combat_delayed(&mut self) {
        self.queue_delayed_at(crate::state::DelayedWhen::EndOfCombat);
    }

    /// Takes every delayed trigger waiting for `when` off the list and
    /// queues it, whoever controls it.
    fn queue_delayed_at(&mut self, when: crate::state::DelayedWhen) {
        let mut i = 0;
        while i < self.state.delayed.len() {
            if self.state.delayed[i].when == when {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue
                    .push_back((trigger.controller, trigger.action));
            } else {
                i += 1;
            }
        }
    }
}
