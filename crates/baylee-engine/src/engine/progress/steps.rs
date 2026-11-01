//! Combat damage, advancing a step and the cleanup step.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

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
            self.state.change_player_counters(
                player,
                baylee_cards_dsl::CounterKind::Poison,
                i32::from(toxic),
            );
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
            if self.state.turn.step == Step::CombatEnd {
                player.mana_pool.expire_combat_retention();
            }
            player.mana_pool.empty_at_step_end();
        }
        let (phase, step) = (self.state.turn.phase, self.state.turn.step);
        let (next_phase, next_step) = match (phase, step) {
            (_, Step::Untap) => {
                self.queue_upkeep_delayed();
                (Phase::Beginning, Step::Upkeep)
            }
            (_, Step::Upkeep) => {
                self.state.effects.remove_where(|effect| {
                    effect.duration == baylee_cards_dsl::Duration::UntilEndOfCurrentUpkeep
                });

                // The draw step's turn-based action, which comes first and
                // before the active player has priority (CR 504.1, then
                // CR 504.2) — the upkeep step itself has none at all
                // (CR 503.1). The first player skips it on turn 1 of a
                // two-player game (CR 103.8).
                let skip = self.state.per_turn.skip_draw_step
                    || self.state.skips_draw_step(self.state.turn.active)
                    || (self.state.turn.number == 1
                        && self.state.players.len() == 2
                        && self.state.turn.active.get() == 0);
                if skip {
                    // CR 103.8a skips the *step*, not only its draw: there is
                    // no beginning of it for "at the beginning of your draw
                    // step" to trigger on, and Sylvan Library drew two cards
                    // on the play's first turn when this went on to
                    // `Step::Draw` without drawing. Straight on to the main
                    // phase, with the turn-based actions that begin it.
                    self.enter_first_main_or_skip()
                } else {
                    // The draw step has begun when its draw is made, so the
                    // draw is made in it: `draw_cards` reads the step to know
                    // that this card is the step's first.
                    self.state.turn.step = Step::Draw;
                    self.queue_draw_debts();
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
                self.enter_first_main_or_skip()
            }
            (Phase::FirstMain, Step::Main) => self.enter_combat_or_skip(),
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
                self.state.delayed.retain(|watch| {
                    !matches!(
                        watch.when,
                        crate::state::DelayedWhen::TriggeredThisTurn {
                            this_combat_only: true,
                            ..
                        }
                    )
                });
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
                self.enter_second_main_or_skip()
            }
            // A cleanup step closes through here only when CR 514.3a opened
            // it to priority, and then "another cleanup step begins"; the
            // turn itself ends in `end_cleanup`, from a step nobody was
            // given priority in.
            (_, Step::End | Step::Cleanup) => (Phase::Ending, Step::Cleanup),
            _ => unreachable!("invalid phase/step combination"),
        };
        self.begin_step(next_phase, next_step);
    }

    fn enter_first_main_or_skip(&mut self) -> (Phase, Step) {
        if self.state.per_turn.skip_main_phases {
            self.enter_combat_or_skip()
        } else {
            self.saga_precombat_main_counters();
            self.queue_main_phase_delayed();
            (Phase::FirstMain, Step::Main)
        }
    }

    fn enter_combat_or_skip(&mut self) -> (Phase, Step) {
        let active = usize::from(self.state.turn.active.get());
        let skipped_anyway = self.state.per_turn.skip_combat_phases;
        let player = &mut self.state.players[active];
        // "Skips their next combat phase" waits for a combat phase it can
        // skip (CR 614.10a): one another effect skips already does not use it.
        let delayed_skip = !skipped_anyway && player.combats_to_skip > 0;
        if delayed_skip {
            player.combats_to_skip -= 1;
        }
        if skipped_anyway || delayed_skip {
            self.enter_second_main_or_skip()
        } else {
            (Phase::Combat, Step::CombatBegin)
        }
    }

    fn enter_second_main_or_skip(&mut self) -> (Phase, Step) {
        if self.state.per_turn.skip_main_phases {
            self.queue_end_step_delayed();
            (Phase::Ending, Step::End)
        } else {
            self.queue_main_phase_delayed();
            (Phase::SecondMain, Step::Main)
        }
    }

    /// Publish the entered phase and its history before collecting beginning triggers.
    fn begin_step(&mut self, next_phase: Phase, next_step: Step) {
        if next_step == Step::Main {
            self.state.per_turn.main_phases_begun =
                self.state.per_turn.main_phases_begun.saturating_add(1);
        }
        self.state.turn.phase = next_phase;
        self.state.turn.step = next_step;
        if next_step == Step::Cleanup {
            self.cleanup = Cleanup::Due;
        }
        // "Until your next upkeep" ends as that upkeep begins, before
        // anything triggers at it (CR 503.1).
        if next_step == Step::Upkeep {
            let active = self.state.turn.active;
            self.state.begin_upkeep_history(active);
            self.state.play_permissions.retain(|permission| {
                permission.player != active
                    || permission.window != baylee_cards_dsl::PlayWindow::UntilYourNextUpkeep.into()
            });
            self.state.effects.begin_upkeep(active);
            self.state.effects.remove_where(|fx| {
                matches!(fx.duration, baylee_cards_dsl::Duration::UntilYourNextUpkeep)
                    && fx.controller == active
            });
        }
        // "At end of combat" triggers as the end of combat step begins
        // (CR 511.2), and two arms above enter it: after combat damage, and
        // straight from the declare attackers step when nothing attacked.
        if next_step == Step::CombatEnd {
            self.queue_end_of_combat_delayed();
        }
        self.state.journal.record(GameEvent::StepChanged {
            active: self.state.turn.active,
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

    /// The cleanup step's turn-based actions, in the order CR 514 gives
    /// them: the active player discards to their maximum hand size first
    /// (CR 514.1), and only then does damage wear off and do "until end of
    /// turn" effects end (CR 514.2). The order is visible: an effect that
    /// ends at 514.2 still applies while the discard is asked for.
    ///
    /// Returns `true` when the discard is a question. Its answer
    /// (`Engine::apply`) performs the discard and then 514.2 itself.
    pub(crate) fn cleanup_step(&mut self) -> bool {
        self.queue_delayed_at(crate::state::DelayedWhen::NextCleanup);
        let active = self.state.turn.active;
        let hand_size = self.state.zones.list(ZoneLocation::Hand(active)).len();
        if let Some(max_hand) = self.state.maximum_hand_size(active)
            && hand_size > max_hand
        {
            self.pending = Pending::DiscardChoice {
                player: active,
                count: u8::try_from(hand_size - max_hand).unwrap_or(u8::MAX),
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
                    || obj.riders.contains(&crate::object::Rider::Saddled)
            },
            |obj| {
                obj.riders
                    .retain(|rider| *rider != crate::object::Rider::Saddled);
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
        crate::copiable_abilities::temporary::restore(&mut self.state);
        self.state
            .effects
            .remove_where(|fx| matches!(fx.duration, baylee_cards_dsl::Duration::UntilEndOfTurn));
        let arena = &self.state.arena;
        self.state.play_permissions.retain(|permission| {
            permission.window != baylee_cards_dsl::PlayWindow::ThisTurn.into()
                && arena
                    .get(permission.card)
                    .is_some_and(|o| o.version == permission.version)
        });
        // Every prevention shield says "this turn" (`crate::prevention`).
        self.state.shields.clear();
        self.state.granted_actions.retain(|grant| {
            matches!(
                grant.offer.effect,
                crate::choice::GrantedActionKind::CancelDelayedLifeLoss { .. }
            )
        });
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
        // Sparse snapshots also restore a prior permanent copy and casting
        // choices inherited from a spell. Physical state is never rolled back.
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
        self.begin_next_turn(self.order_after())
    }
}
