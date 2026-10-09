//! Combat damage, advancing a step, the untap step and the cleanup step.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// One `Modifier::UntapAtMost` binding the active player's untap step.
pub(super) struct UntapLimit {
    /// What it counts.
    of: &'static Filter,
    /// The effect's controller, the filter's "you".
    you: PlayerId,
    /// The effect's source, the filter's "this".
    this: Option<ObjectId>,
    /// How many of them may untap.
    count: u8,
}

impl<L: CardLookup> Engine<L> {
    /// Deals one combat damage step's damage, and what toxic adds to it.
    ///
    /// "Combat damage dealt to a player by a creature with toxic causes that
    /// creature's controller to give the player a number of poison counters
    /// equal to that creature's total toxic value, in addition to the
    /// damage's other results" (CR 702.164c, 120.3g). It is a result of the
    /// damage, so it is read off what the step actually journalled: damage
    /// prevented to nothing gives nothing, and damage a trampler assigned to
    /// a planeswalker is not dealt to a player. The total is summed over the
    /// creature's toxic abilities as it has them now (CR 702.164b), asked of
    /// the object and not its card, so a copy's list answers.
    fn deal_combat_damage(&mut self, first_strike_step: bool) {
        let assignments = combat::collect_combat_damage(&mut self.state, first_strike_step);
        self.combat_damage = Some(crate::damage::DamageWork::new(&mut self.state, assignments));
    }

    pub(in crate::engine) fn finish_combat_damage(&mut self, work: &crate::damage::DamageWork) {
        for (source, player) in work.damaged_players() {
            let toxic: u16 = self.state.object(source).map_or(0, |obj| {
                obj.abilities(&self.lookup)
                    .iter()
                    .filter_map(|ability| {
                        if let baylee_cards_dsl::AbilityDef::Toxic { poison } = ability {
                            Some(u16::from(*poison))
                        } else {
                            None
                        }
                    })
                    .sum()
            });
            let counters = &mut self.state.players[usize::from(player.get())].poison;
            *counters = counters.saturating_add(toxic);
        }
    }

    /// Ends the current step and begins the next one.
    ///
    /// **An arm is named for the step being left, and its block runs the
    /// turn-based actions of the step being entered** — it fires after that
    /// step's last priority round and before anyone has priority in the
    /// next. Reading it the other way round is what put the precombat main
    /// phase's two turn-based actions in the `FirstMain` arm, where they
    /// fired as *combat* began: a Saga took its lore counter a whole phase
    /// late and Mana Drain's mana arrived after the main phase it was cast
    /// to pay for.
    pub(crate) fn advance_step(&mut self) {
        // The step that is ending, ends: every player's mana pool empties
        // (CR 500.5, and CR 106.4 from the other side). This is the only
        // place a step or phase ever changes, and it is *before* the match
        // because the turn-based actions in those arms belong to the step
        // being entered — the draw of CR 504.1 happens in the draw step, not
        // at the end of the upkeep, and mana made in the upkeep must not pay
        // for anything after it.
        //
        // `ManaPool::empty_at_step_end` was written for this and had exactly
        // one caller in the workspace: its own unit test. Mana therefore
        // floated across steps, phases and turns, which is a whole category
        // of illegal play — a Forest tapped in the first main phase paying
        // for an instant in the opponent's end step — and it silently
        // propped up any test that spent mana in a later step than it made
        // it in.
        for player in &mut self.state.players {
            player.mana_pool.empty_at_step_end();
        }
        let (phase, step) = (self.state.turn.phase, self.state.turn.step);
        let (next_phase, next_step) = match (phase, step) {
            (_, Step::Untap) => {
                self.queue_upkeep_delayed();
                (Phase::Beginning, Step::Upkeep)
            }
            (_, Step::Upkeep) => {
                // The draw step's turn-based action, which comes first and
                // before the active player has priority (CR 504.1, then
                // CR 504.2) — the upkeep step itself has none at all
                // (CR 503.1). The first player skips it on turn 1 of a
                // two-player game (CR 103.8).
                let skip = self.state.turn.number == 1
                    && self.state.players.len() == 2
                    && self.state.turn.active.get() == 0;
                if skip {
                    // CR 103.8a skips the *step*, not only its draw: there is
                    // no beginning of it for "at the beginning of your draw
                    // step" to trigger on, and Sylvan Library drew two cards
                    // on the play's first turn when this went on to
                    // `Step::Draw` without drawing. Straight on to the main
                    // phase, with the turn-based actions that begin it.
                    self.saga_precombat_main_counters();
                    self.queue_first_main_delayed();
                    (Phase::FirstMain, Step::Main)
                } else {
                    // The draw step has begun when its draw is made, so the
                    // draw is made in it: `draw_cards` reads the step to know
                    // that this card is the step's first.
                    self.state.turn.step = Step::Draw;
                    // Nobody draws for an active player who has left
                    // (CR 800.4j).
                    if !self.active_has_left() {
                        // A draw Island Sanctuary may replace waits to be
                        // asked about (`offer_queued_draw`).
                        let active = self.state.turn.active;
                        self.state.draw_cards(active, 1);
                        self.offer_queued_draw();
                    }
                    (Phase::Beginning, Step::Draw)
                }
            }
            (_, Step::Draw) => {
                // The precombat main phase's own turn-based actions, which
                // happen before anybody holds priority in it (CR 505.4 and
                // CR 505.6, in that order): each Saga the active player
                // controls takes a lore counter. The delayed triggers go
                // second because they are triggered abilities and wait for
                // the stack, which the turn-based action does not use.
                self.saga_precombat_main_counters();
                self.queue_first_main_delayed();
                (Phase::FirstMain, Step::Main)
            }
            (Phase::FirstMain, Step::Main) => (Phase::Combat, Step::CombatBegin),
            (Phase::SecondMain, Step::Main) => {
                self.queue_end_step_delayed();
                (Phase::Ending, Step::End)
            }
            (_, Step::CombatBegin) => (Phase::Combat, Step::DeclareAttackers),
            (_, Step::DeclareAttackers) => {
                // CR 508.8: with nothing attacking, the declare blockers and
                // combat damage steps do not happen at all. They were being
                // walked through anyway, which is not a step nobody notices:
                // the blockers step asks a question, so every turn where
                // neither seat swung stopped on "Declare blockers" with an
                // empty board and waited for an answer to a question the
                // rules never asked. Nothing is lost by leaving them out —
                // there is no damage to deal and no trigger can be waiting
                // on a step that is skipped.
                if self.state.combat.attackers().is_empty() {
                    (Phase::Combat, Step::CombatEnd)
                } else {
                    (Phase::Combat, Step::DeclareBlockers)
                }
            }
            (_, Step::DeclareBlockers) => {
                // Deal combat damage on entering the damage step(s).
                if self
                    .state
                    .combat
                    .prepared_first_strike()
                    .unwrap_or_else(|| self.any_first_or_double_striker())
                {
                    self.deal_combat_damage(true);
                    (Phase::Combat, Step::CombatDamageFirst)
                } else {
                    self.deal_combat_damage(false);
                    (Phase::Combat, Step::CombatDamage)
                }
            }
            (_, Step::CombatDamageFirst) => {
                self.deal_combat_damage(false);
                (Phase::Combat, Step::CombatDamage)
            }
            (_, Step::CombatDamage) => (Phase::Combat, Step::CombatEnd),
            (_, Step::CombatEnd) => {
                self.state.combat = crate::combat::CombatState::default();
                // Nothing is attacking any more, so an anthem conditioned on
                // it stops applying — and the removal below only bumps the
                // effect generation when there *was* an until-end-of-combat
                // effect to remove.
                self.state.board_state_changed();
                self.combat_declared = CombatDeclared::None;
                self.state.effects.remove_where(|fx| {
                    matches!(fx.duration, baylee_cards_dsl::Duration::UntilEndOfCombat)
                });
                (Phase::SecondMain, Step::Main)
            }
            // A cleanup step closes through here only when CR 514.3a opened
            // it to priority, and then "another cleanup step begins"; the
            // turn itself ends in `end_cleanup`, from a step nobody was
            // given priority in.
            (_, Step::End | Step::Cleanup) => (Phase::Ending, Step::Cleanup),
            _ => unreachable!("invalid phase/step combination"),
        };
        self.state.turn.phase = next_phase;
        self.state.turn.step = next_step;
        if next_step == Step::Cleanup {
            self.cleanup = Cleanup::Due;
        }
        // "At end of combat" triggers as the end of combat step begins
        // (CR 511.2), and two arms above enter it: after combat damage, and
        // straight from the declare attackers step when nothing attacked.
        if next_step == Step::CombatEnd {
            self.queue_end_of_combat_delayed();
        }
        self.state.journal.record(GameEvent::StepChanged {
            phase: next_phase,
            step: next_step,
        });
    }

    pub(crate) fn any_first_or_double_striker(&self) -> bool {
        use baylee_cards_dsl::KeywordSet as K;
        self.state
            .combat
            .attackers()
            .iter()
            .map(|a| a.creature)
            .chain(self.state.combat.blockers.iter().map(|b| b.blocker))
            .any(|id| {
                self.state.object(id).is_some_and(|o| {
                    let kw = o.characteristics().keywords;
                    kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
                })
            })
    }

    /// The untap step's second turn-based action (CR 502.2): the game looks
    /// at the previous turn and decides whether the designation flips.
    ///
    /// Both halves are one-sided on purpose. Day becomes night when the
    /// previous turn's active player cast **no** spells; night becomes day
    /// when they cast **two or more**. One spell holds the designation
    /// where it is, in either direction.
    ///
    /// A game with neither designation skips the check entirely and keeps
    /// having neither (CR 730.2c) — which is every game in this pool that
    /// has no daybound card in it, so the common case costs two compares.
    ///
    /// CR 502.2a states a variant for the shared team turns option, which
    /// is CR 805 and something this engine does not offer: `--teams` puts
    /// chairs on sides, and each of them still takes its own turn. The
    /// plain rule is therefore the whole rule here.
    fn check_day_night(&mut self) {
        let (Some(now), Some(previous)) = (self.state.day_night, self.state.previous_turn) else {
            return;
        };
        match now {
            DayNight::Day if previous.spells_cast == 0 => self.state.become_night(),
            DayNight::Night if previous.spells_cast >= 2 => self.state.become_day(),
            _ => {}
        }
    }

    /// Whether an effect keeps `id` from untapping this untap step
    /// (CR 502.3, and CR 613.11 for what kind of effect that is).
    ///
    /// One condition, and the missing second one is worth saying out loud:
    /// **whose** untap step is already answered by the loop that calls
    /// this. CR 502.3 untaps the permanents *the active player controls*
    /// and no others, and every printing of the sentence names that same
    /// player — Basalt Monolith says "during **your** untap step" about
    /// itself, Paralyze says "during **its controller's** untap step" about
    /// the creature it enchants, and the card-script reference writes both
    /// as `ValidStepTurnToController$ You`, where "you" is the *affected*
    /// card's controller rather than the effect's.
    ///
    /// Reading it as the effect's controller instead would have been a
    /// one-word mistake that worked on the monoliths — an ability a
    /// permanent has about itself puts all three players on the same seat —
    /// and broke the 45 Auras that are the commonest printing of this
    /// sentence: the Aura's controller is the one player whose untap step
    /// the enchanted creature never untaps in anyway.
    fn keeps_tapped(&self, id: ObjectId) -> bool {
        let Some(obj) = self.state.object(id) else {
            return false;
        };
        self.state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::DoesNotUntap)
                && crate::effects::applies_to(&self.state, fx, obj)
        })
    }

    /// The permanents CR 502.3 lets the active player decide about.
    ///
    /// Three conditions, and the third is the one that is easy to leave out.
    /// Tapped, because an untapped permanent has nothing to determine.
    /// Controlled by the active player, because CR 502.3 is only ever about
    /// their permanents. And **not already kept from untapping** — a Basalt
    /// Monolith that also said "you may choose not to untap" would otherwise
    /// be offered a question whose two answers do the same thing, which is
    /// the offer that contradicts its own apply.
    fn untap_optional(&self) -> Vec<ObjectId> {
        let active = self.state.turn.active;
        // What phased in a moment ago untaps with the rest (CR 502.1 before
        // 502.3); what is still phased out does not.
        self.state
            .battlefield_seen()
            .filter(|id| {
                let Some(obj) = self.state.object(*id) else {
                    return false;
                };
                obj.controller == active
                    && obj.status.contains(Status::TAPPED)
                    && !self.keeps_tapped(*id)
                    && self.state.effects.iter().any(|fx| {
                        let applies =
                            matches!(fx.modifier, baylee_cards_dsl::Modifier::MayChooseNotToUntap)
                                && crate::effects::applies_to(&self.state, fx, obj);
                        // Read here and never by the projection, so this is
                        // where the static is applied: the recorder's door.
                        #[cfg(test)]
                        if applies {
                            crate::ability_log::static_applied(fx);
                        }
                        applies
                    })
            })
            .collect()
    }

    /// The untap step, up to the point where it may have to ask.
    ///
    /// Returns `true` when it suspended on a question, which is the
    /// contract [`Self::progress_step`] has with every other step.
    ///
    /// The split is where CR 502.3's two halves already are. Phasing
    /// (CR 702.26a) and the day/night check (CR 502.2) come first and
    /// happen exactly once; then "the active player **determines** which
    /// permanents they control will untap", which is a question whenever a
    /// permanent gives it a second answer; then "they untap them all
    /// simultaneously", which is [`Self::finish_untap_step`] and is
    /// reachable from either side of the question. Nothing before the
    /// suspension may run again on the way back, which is why the resume
    /// path enters at the second function rather than re-entering this one.
    ///
    /// **No priority is granted here.** CR 502.4 says no player receives
    /// priority during the untap step; it does not say the turn-based
    /// action may not take the answer its own rule asks a player for.
    pub(crate) fn untap_step(&mut self) -> bool {
        let active = self.state.turn.active;
        // "Players skip their untap steps" (Stasis) replaces the step with
        // nothing (CR 614.1b, 614.10): no phasing, no day/night check, no
        // untap, and an effect waiting for the player's *next* untap step
        // keeps waiting for one that is not skipped (CR 614.10a) — which is
        // why this goes straight on and not through `finish_untap_step`.
        if self.state.skips_untap_step(active) {
            self.advance_step();
            return false;
        }
        // "All phased-out permanents that the active player controlled when
        // they phased out phase in" (CR 502.1): a phased-out permanent is
        // not projected, so its controller is still that one. One that
        // phased out indirectly phases in with its host and never by itself
        // (CR 702.26g).
        let coming: Vec<ObjectId> = self
            .state
            .zones
            // phasing: the walk is for the permanents that are phased out.
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|&id| {
                self.state.object(id).is_some_and(|o| {
                    o.controller == active
                        && o.status.contains(Status::PHASED_OUT)
                        && !o.status.contains(Status::PHASED_OUT_INDIRECTLY)
                })
            })
            .collect();
        for id in coming {
            self.state.phase_in(id);
        }
        self.check_day_night();
        // CR 502.3, the third turn-based action: "the active player
        // determines which permanents they control will untap. Then they
        // untap them all simultaneously."
        let optional = self.untap_optional();
        if optional.is_empty() {
            return self.untap_under_limits(Vec::new(), Vec::new());
        }
        // The answer names the permanents that stay tapped, so an empty one
        // is "untap everything" — the determination every board without
        // such a permanent makes, and the one an automated seat produces by
        // answering the minimum.
        let max = u8::try_from(optional.len()).unwrap_or(u8::MAX);
        self.pending_plan = Some(PlanKind::UntapChoice);
        self.pending = Pending::ChooseCards {
            player: active,
            options: optional,
            min: 0,
            max,
            prompt: crate::choice::ChoicePrompt::LeaveTapped,
            total: None,
        };
        self.awaiting_answer = true;
        true
    }

    /// The limits on the active player's untap step
    /// (`Modifier::UntapAtMost`), each with the "you" and "this" its filter
    /// is read against.
    fn untap_limits(&self) -> Vec<UntapLimit> {
        let active = self.state.turn.active;
        self.state
            .effects
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::UntapAtMost { who, of, count } = fx.modifier else {
                    return None;
                };
                crate::eval::players(who, &self.state, fx.controller)
                    .is_some_and(|p| p.contains(&active))
                    .then_some(UntapLimit {
                        of,
                        you: fx.controller,
                        this: fx.source,
                        count,
                    })
            })
            .collect()
    }

    /// Whether `limit` counts the permanent `id`.
    fn counts(&self, limit: &UntapLimit, id: ObjectId) -> bool {
        self.state.object(id).is_some_and(|obj| {
            crate::eval::matches(
                limit.of,
                &self.state,
                obj,
                limit.you,
                limit.this.unwrap_or(id),
            )
        })
    }

    /// CR 502.3's determination under untap limits (Smoke, Winter Orb):
    /// of the permanents that would untap, the ones a limit counts untap as
    /// the active player names them, and the rest of them stay tapped once
    /// no limit has room left for any of them.
    ///
    /// "Can't untap more than one" keeps a permanent tapped only when the
    /// limit is full: the default is still that everything untaps (CR
    /// 502.3), so the player chooses *which*, not *whether*. The question
    /// is asked again after every answer, and each answer counts against
    /// every limit the permanents in it match — an animated land under
    /// Smoke and Winter Orb is the one creature and the one land (the Smoke
    /// ruling), and Static Orb beside Winter Moon lets two permanents untap,
    /// at most one of them a nonbasic land (the Winter Moon ruling).
    ///
    /// `max` is the most a single answer can name without breaking a limit
    /// whatever it names: the smallest room among the limits that cannot
    /// take everything still on the menu. Anything from one to that is
    /// legal, so the menu never offers an answer the apply would refuse.
    /// When every limit can take everything on the menu, nothing is asked.
    ///
    /// `kept` is what the player already chose to leave tapped
    /// (`ChoicePrompt::LeaveTapped`), asked first so that a permanent the
    /// player keeps by choice does not take a limit's room. Returns `true`
    /// when a question was asked.
    pub(crate) fn untap_under_limits(
        &mut self,
        kept: Vec<ObjectId>,
        chosen: Vec<ObjectId>,
    ) -> bool {
        let active = self.state.turn.active;
        let limits = self.untap_limits();
        let counted = |id: ObjectId| limits.iter().any(|l| self.counts(l, id));
        // What would untap if no limit applied. A phased-out permanent is
        // treated as though it does not exist (CR 702.26b): no limit counts
        // it and no menu offers it.
        let would: Vec<ObjectId> =
            self.state
                .battlefield_seen()
                .filter(|id| {
                    self.state.object(*id).is_some_and(|o| {
                        o.controller == active && o.status.contains(Status::TAPPED)
                    }) && !kept.contains(id)
                        && !self.keeps_tapped(*id)
                })
                .collect();
        let room: Vec<usize> = limits
            .iter()
            .map(|l| {
                let used = chosen.iter().filter(|c| self.counts(l, **c)).count();
                usize::from(l.count).saturating_sub(used)
            })
            .collect();
        // Counted, not yet named, and every limit counting it has room.
        let open: Vec<ObjectId> = would
            .iter()
            .copied()
            .filter(|id| {
                !chosen.contains(id)
                    && counted(*id)
                    && limits
                        .iter()
                        .zip(&room)
                        .all(|(l, r)| *r > 0 || !self.counts(l, *id))
            })
            .collect();
        let most = limits
            .iter()
            .zip(&room)
            .filter(|(l, r)| open.iter().filter(|id| self.counts(l, **id)).count() > **r)
            .map(|(_, r)| *r)
            .min();
        let Some(most) = most else {
            // Everything still open fits: it untaps with what was named, and
            // what the limits shut out stays tapped.
            let mut stays = kept;
            stays.extend(
                would
                    .iter()
                    .filter(|id| counted(**id) && !chosen.contains(id) && !open.contains(id)),
            );
            self.finish_untap_step(&stays);
            return false;
        };
        self.pending_plan = Some(PlanKind::UntapLimit { kept, chosen });
        self.pending = Pending::ChooseCards {
            player: active,
            options: open,
            min: 1,
            max: u8::try_from(most).unwrap_or(u8::MAX),
            prompt: crate::choice::ChoicePrompt::Untap,
            total: None,
        };
        self.awaiting_answer = true;
        true
    }

    /// "Then they untap them all simultaneously" (CR 502.3), with `kept`
    /// left out of it.
    ///
    /// `kept` has been checked against the very list [`Self::untap_optional`]
    /// produced before this is reached, so it is read and not re-validated —
    /// the same arrangement `cost_wizard::pay` has with its own menu.
    ///
    /// The `keeps_tapped` half is read off the effect table rather than off
    /// a projection: what a "doesn't untap" effect modifies is a rule and
    /// not a characteristic (CR 613.11), so there is nothing on the
    /// permanent to look at.
    pub(crate) fn finish_untap_step(&mut self, kept: &[ObjectId]) {
        let active = self.state.turn.active;
        for id in self.state.battlefield_view() {
            let tapped = self
                .state
                .object(id)
                .is_some_and(|o| o.controller == active && o.status.contains(Status::TAPPED));
            if tapped && !kept.contains(&id) && !self.keeps_tapped(id) {
                self.state.set_tapped(id, false);
                self.state.journal.record(GameEvent::ObjectUntapped {
                    object: id,
                    cause: Cause::TurnBased,
                });
            }
        }
        // "…doesn't untap during your **next** untap step": the step it was
        // waiting for has now happened, so the effect is spent.
        //
        // **After the loop above and not before it.** This is the fourth
        // duration that expires at a point in the turn structure — the other
        // three are `UntilYourNextTurn` in `start_turn`, `UntilEndOfCombat`
        // in `end_of_combat` and `UntilEndOfTurn` in
        // `cleanup_ends_the_turns_effects` — and it
        // is the first that ends inside the step it is about rather than at
        // a boundary between two. An expiry written at the top of the untap
        // step would let the land untap on schedule and leave a card that
        // compiles, claims `Implemented` and does nothing at all; one
        // written at the turn boundary instead would take the *following*
        // untap step with it and cost the land a second turn. Both
        // directions are pinned in `untap_tests`.
        //
        // `fx.controller` and not the affected permanent's controller, which
        // is the one place this and `keeps_tapped` read CR 502.3's "your"
        // from different seats. The sentence is only ever printed about the
        // source of the ability that created it, so the two seats are the
        // same one on every card that can say this; a permanent that changed
        // hands in between is the case where they would part, and no
        // printing reaches it.
        self.state.effects.remove_where(|fx| {
            matches!(
                fx.duration,
                baylee_cards_dsl::Duration::UntilYourNextUntapStep
            ) && fx.controller == active
        });
        self.advance_step();
    }

    /// The cleanup step's turn-based actions, in the order CR 514 gives
    /// them: the active player discards to their maximum hand size first
    /// (CR 514.1), and only then does damage wear off and do "until end of
    /// turn" effects end (CR 514.2). The order is visible: an effect that
    /// ends at 514.2 still applies while the discard is asked for.
    ///
    /// Returns `true` when the discard is a question. Its answer
    /// (`Engine::apply`) performs the discard and then 514.2 itself.
    pub(crate) fn cleanup_step(&mut self) -> bool {
        let active = self.state.turn.active;
        // Reliquary Tower & co.: no maximum hand size for this player.
        let no_max = self.state.no_max_hand_size(active);
        let max_hand = if no_max {
            i32::MAX
        } else {
            7i32 + i32::from(self.state.players[active.get() as usize].hand_modifier)
        };
        let hand_size = self.state.zones.list(ZoneLocation::Hand(active)).len() as i32;
        if hand_size > max_hand {
            self.pending = Pending::DiscardChoice {
                player: active,
                count: (hand_size - max_hand) as u8,
            };
            self.awaiting_answer = true;
            return true;
        }
        self.cleanup_ends_the_turns_effects();
        false
    }

    /// CR 514.2: all damage is removed and every "until end of turn" and
    /// "this turn" effect ends, simultaneously. What follows is the step's
    /// first check, which is the machine's own next pass (`Cleanup::Checking`).
    pub(crate) fn cleanup_ends_the_turns_effects(&mut self) {
        let mut reverted: Vec<ObjectId> = Vec::new();
        // Only objects with something to end are written, so the chunks of
        // an arena holding none stay shared with the answer's checkpoint.
        self.state.arena.update_where(
            |obj| {
                obj.damage != 0
                    || obj.deathtouched
                    || obj.regeneration_shields != 0
                    || obj.own_abilities_until_eot
            },
            |obj| {
                obj.damage = 0;
                obj.deathtouched = false;
                // "The next time it would be destroyed **this turn**"
                // (CR 701.19a) — an unspent shield does not keep.
                obj.regeneration_shields = 0;
                if obj.own_abilities_until_eot {
                    obj.drop_own_abilities();
                    obj.own_abilities_until_eot = false;
                    reverted.push(obj.id);
                }
            },
        );
        self.state
            .effects
            .remove_where(|fx| matches!(fx.duration, baylee_cards_dsl::Duration::UntilEndOfTurn));
        // Every prevention shield says "this turn" (`crate::prevention`).
        self.state.shields.clear();
        self.state.granted_actions.clear();
        self.state.prune_damage_sources();
        for player in &mut self.state.players {
            player.mana_pool.expire_turn_retention();
        }
        // A temporary copy (Cursed Mirror) reverts in *two* places here, and
        // only the characteristics half is the line above: that half is a
        // `Layer::Copy` continuous effect with `Duration::UntilEndOfTurn`, so
        // expiring it is the whole of its revert and nothing has to undo a
        // base. The ability half cannot expire, because abilities are not
        // layer-projected — the copy wrote them into `own_abilities`, and the
        // loop above is what takes them back.
        //
        // `None` rather than a stashed list because the object is card-backed
        // and `GameObject::abilities` falls through to its own face. Nothing
        // card-less can be flagged: the clause is on a printed card, and a
        // token copy of a Mirror that had become a creature is handed the
        // *creature's* list by `settle_copied_rules_text` and is not a copy
        // that ends.
        //
        // The flag is why this is not a sweep over every `own_abilities`.
        // There used to be a sweep restoring `original_base` at this point,
        // which reverted nothing because no path ever set the field — and
        // would now revert the *permanent* copies that do, turning a Glasspool
        // Mimic back into a 0/0 on the turn it was cast. That field is spent
        // at the zone change instead (CR 400.7, `GameState::move_object`), and
        // this one is spent at whichever of the two comes first.
        //
        // A copy ending is a source departing as far as its effects are
        // concerned, so what `sync_static_effects` does for a permanent that
        // left the battlefield is what happens here: drop the continuous
        // effects and replacement rules registered from the copied list, and
        // let the next pass register the printed ones from the reverted list.
        // Without it a Mirror that spent a turn as a Karmic Guide would still
        // have protection from black as an artifact on the next.
        if !reverted.is_empty() {
            self.state.effects.remove_where(|fx| {
                matches!(
                    fx.duration,
                    baylee_cards_dsl::Duration::WhileSourceOnBattlefield
                ) && fx.source.is_some_and(|s| reverted.contains(&s))
            });
            self.state
                .replacement_rules
                .retain(|r| !reverted.contains(&r.source));
        }
        self.state.invalidate_projections();
        // What a player who has left controls by default is exiled as the
        // last effect giving it to somebody else ends (CR 800.4c), which for
        // an "until end of turn" effect is here. The check that follows is
        // the machine's own pass, and its step 0a does it: the removal above
        // moved the effect generation, and the step is still this one.
        self.cleanup = Cleanup::Checking;
    }

    /// The step's first check performed a state-based action or found a
    /// triggered ability (CR 514.3a), so the step gives priority instead of
    /// ending. Called from where the machine performs them, and it is not
    /// only the stack that shows it afterwards: an Equipment falling off a
    /// land whose animation just ended (CR 704.5n) or a +1/+1 counter
    /// cancelling a -1/-1 counter (CR 704.5q) moves nothing and journals
    /// nothing, and either one opens the window.
    pub(crate) fn cleanup_check_acted(&mut self) {
        if self.cleanup == Cleanup::Checking {
            self.cleanup = Cleanup::Open;
        }
    }

    /// The cleanup step ends without anyone having had priority in it, and
    /// with it the turn. Returns `true` when the next turn's player is
    /// asked whether to skip it.
    pub(crate) fn end_cleanup(&mut self) -> bool {
        self.state.combat = crate::combat::CombatState::default();
        self.state.board_state_changed();
        self.combat_declared = CombatDeclared::None;
        self.begin_next_turn(self.state.turn.active)
    }
}
