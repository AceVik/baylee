use super::{
    AbilityDef, AbilityLoc, CardLookup, Cause, CombatDeclared, EndReason, Engine, GameEvent,
    GameObject, GameResult, NameRef, ObjectId, ObjectKind, Pending, Phase, PlanKind, PlayerId,
    Resolution, SmallVec, Status, Step, Zone, ZoneLocation, ZonePosition, combat, eval, mana_pay,
    resolve, sba, trigger,
};
use crate::choice::{
    CastModeDesc, CastModeKind, ChoicePrompt, PlayerAction, PriorityHold, SeatAutomation,
    TargetPrompt, YesNoPrompt,
};
use crate::state::Side;
use crate::turn::DayNight;
use crate::win::Victor;
use baylee_cards_dsl::{Filter, PlayerRel, SpellMode, TargetReq, TargetSpec};
use baylee_core::ids::{AbilityRef, SeatSet};
use baylee_core::preset::LoopPolicy;

/// What CR 608.2b's re-check found about the object on top of the stack.
///
/// Three answers and not two, because "no legal target left" and "this was
/// never a targeted spell" have to be told apart: the first removes the
/// spell from the stack and the second is most of the stack.
enum TargetLegality {
    /// Nothing to ask. The object specifies no targets, or the targets it
    /// specifies are not ones a player chose.
    NotAsked,
    /// At least one chosen target is still legal, and these are the ones
    /// that are. CR 608.2b: the spell resolves, and "the spell or ability
    /// won't do anything to an illegal target".
    Kept {
        /// The chosen objects that are still legal targets.
        objects: SmallVec<[ObjectId; 2]>,
        /// The chosen players that still are.
        players: SeatSet,
        /// The objects chosen for the second instance of the word that
        /// still are — a list of its own, narrowed by its own requirement.
        second: SmallVec<[ObjectId; 1]>,
    },
    /// Every chosen target is now illegal: it does not resolve.
    AllIllegal,
}

impl<L: CardLookup> Engine<L> {
    /// A seat's automation settings.
    #[must_use]
    pub fn automation(&self, player: PlayerId) -> &SeatAutomation {
        static EMPTY: std::sync::OnceLock<SeatAutomation> = std::sync::OnceLock::new();
        self.automation
            .get(player.get() as usize)
            .unwrap_or_else(|| EMPTY.get_or_init(SeatAutomation::default))
    }

    /// Updates seat automation before running the next decision.
    pub(crate) fn set_automation(&mut self, player: PlayerId, action: &PlayerAction) {
        let Some(seat) = self.automation.get_mut(player.get() as usize) else {
            return;
        };
        match action {
            PlayerAction::SetPriorityHold(hold) => {
                seat.hold = *hold;
                seat.priority_paused = matches!(hold, PriorityHold::Always);
            }
            PlayerAction::SetAbilityYield { ability, enabled } => {
                seat.set_yield(*ability, *enabled);
            }
            PlayerAction::SetAbilityPolicy {
                ability,
                pass,
                answer,
            } => {
                seat.set_yield(*ability, *pass);
                seat.set_standing_answer(*ability, *answer);
            }
            _ => {}
        }
    }

    /// Drops a seat's priority hold once the condition it named is met.
    ///
    /// Holds are cancelled here rather than where they are consumed so
    /// that the *client* sees an accurate setting: a hold that has already
    /// expired must not still be reported as active.
    fn expire_holds(&mut self) {
        let stack_depth = self.state.zones.list(ZoneLocation::Stack).len();
        let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
        let turn = self.state.turn.number;
        for seat in &mut self.automation {
            let expired = match seat.hold {
                PriorityHold::Always | PriorityHold::PassWhenNothingToDo => false,
                // Empty, or somebody responded to what was being let through.
                PriorityHold::UntilStackEmpty { depth } => {
                    stack_depth == 0 || stack_depth > depth as usize
                }
                // Reached the top, or left the stack without ever doing so.
                PriorityHold::UntilTopOfStack { object } => {
                    top == Some(object)
                        || self
                            .state
                            .object(object)
                            .is_none_or(|o| o.zone != crate::zone::Zone::Stack)
                }
                PriorityHold::UntilEndOfTurn { turn: set_on } => turn != set_on,
            };
            if expired {
                seat.priority_paused = true;
                seat.hold = PriorityHold::Always;
            }
        }
    }

    /// Answers the pending choice from the acting seat's automation, if it
    /// covers it. Returns `true` when it did, and the caller loops.
    ///
    /// Only two kinds of question are ever answered this way: priority
    /// (always by passing, which the seat could always have done) and a
    /// yes/no the seat has explicitly stored an answer for. Nothing that
    /// could *lose* a game is automated, and no automated answer is one
    /// the seat could not have given by hand.
    fn auto_answer(&mut self) -> bool {
        let answer = match &self.pending {
            Pending::Priority { player, legal } => {
                let settings = self.automation(*player);
                let top_ability = self
                    .state
                    .zones
                    .list(ZoneLocation::Stack)
                    .last()
                    .and_then(|id| self.state.object(*id))
                    .and_then(|obj| {
                        let loc = obj.ability?;
                        loc.card
                            .map(|card| baylee_core::ids::AbilityRef::new(card, loc.index))
                    });
                let hold = settings.hold;
                let pass = !settings.priority_paused
                    && (match hold {
                        PriorityHold::PassWhenNothingToDo => legal.nothing_but_passing(),
                        // Every other variant either withholds the decision or
                        // does not, and `suppresses` is the one place that says
                        // which — the same answer the view hands the client, so
                        // an indicator cannot disagree with the engine. The
                        // expiry pass above already cleared any hold whose
                        // condition is met, so an active one still means "keep
                        // going".
                        other => other.suppresses(),
                    } || top_ability.is_some_and(|ability| settings.yields_to(ability)));
                pass.then_some((*player, PlayerAction::PassPriority))
            }
            // Two conditions, and the second is the one that is easy to
            // leave out: the question has to be *of a kind* a standing
            // answer may cover. A handle alone is not enough, because a
            // kicker, a shockland's two life and a tax trigger all carry
            // one — see [`YesNoPrompt::automatable`].
            Pending::YesNo {
                player,
                prompt,
                source: Some(ability),
            } if prompt.automatable() => self
                .automation(*player)
                .standing_answer(*ability)
                .map(|a| (*player, PlayerAction::YesNo(a.as_bool()))),
            _ => None,
        };
        let Some((player, action)) = answer else {
            return false;
        };
        self.awaiting_answer = false;
        // An automated answer goes through the ordinary action path, so it
        // is validated and journaled exactly like a hand-played one — a
        // replay cannot tell the difference, which is the point.
        self.apply_inner(player, action).is_ok()
    }

    /// Runs the game to the next decision, letting seat automation answer
    /// the decisions it covers and running on.
    ///
    /// The bound is a safety net, not a budget: every automated answer is
    /// an action the game accepted, so it makes progress the same way a
    /// human answer does. It exists because "the engine spins forever" is
    /// a worse failure than "an automated seat is asked one question it
    /// meant to skip".
    pub(crate) fn run_until_choice(&mut self) {
        const AUTO_ANSWER_LIMIT: u32 = 4096;
        for step in 0..AUTO_ANSWER_LIMIT {
            self.run_machine();
            if !self.awaiting_answer {
                return;
            }
            self.expire_holds();
            // Leave a real, unanswered decision at the safety boundary.
            // Consuming the final answer without running the machine again
            // otherwise exposes a stale Pending to the host.
            if step + 1 == AUTO_ANSWER_LIMIT || !self.auto_answer() {
                return;
            }
        }
    }

    fn run_machine(&mut self) {
        // Nothing happens before turn 1, whatever the flag says: a
        // concession during the mulligans once cleared it and ran the game
        // on past every seat still deciding (#267).
        if self.awaiting_answer || self.mulligans.is_some() {
            return;
        }
        // One watch per decision-free segment: this is the exact stretch in
        // which nobody is asked anything, so it is the only place a game can
        // loop without a player being able to stop it (see `crate::loops`).
        let mut watch = crate::loops::LoopWatch::default();
        loop {
            let signature = watch.wants_sample().then(|| self.state.loop_signature());
            if let Some(period) = watch.step(signature)
                && self.on_loop_detected(period)
            {
                return;
            }
            // 0. A token copy's rules text (CR 707.2), before anything asks
            //    what a permanent can do.
            self.settle_copied_rules_text();
            // 0a. Continuous effects: sync statics with the battlefield and
            //    refresh characteristic caches (generation compare).
            self.sync_static_effects();
            self.state.refresh_characteristics();
            // 0b. As-it-enters modifiers (taplands, shockland choices).
            let wrote = self.apply_enter_modifiers();
            if self.awaiting_answer {
                return;
            }
            // A modifier that wrote to the board wrote *behind* 0a, and the
            // state-based actions two steps down read the projection rather
            // than the board. Counters placed as a permanent enters
            // (CR 614.1c) are a characteristic input (CR 613.4c), so a 0/0
            // that arrives under a +1/+1 counter is a 1/1 only once the
            // projection has been recomputed — read at 0a's value it is the
            // 0/0 it prints, and CR 704.5f puts it into a graveyard between
            // its own arrival and anybody being asked anything. Going round
            // once is the whole fix: the scan advances `entry_scan_seq`
            // before it does any work, so the next pass finds no arrivals
            // and falls through.
            if wrote {
                continue;
            }
            // 1. Game over?
            if let Some(result) = self.game_result() {
                self.end_game(result);
                return;
            }
            // Detect triggers while their sources still exist. CR 603.2 /
            // 117.5: detection precedes SBAs; stacking and target choices follow.
            self.queue_new_triggers();
            // 2. State-based actions (fixpoint).
            let outcome = sba::run(&mut self.state, &self.lookup);
            if let Some((player, options)) = outcome.legend_choice {
                self.pending = Pending::LegendChoice { player, options };
                self.awaiting_answer = true;
                return;
            }
            if let Some((player, card)) = outcome.commander_zone {
                self.ask_commander_zone(player, card);
                return;
            }
            if outcome.changed {
                continue;
            }
            // 2b. Sagas (CR 714.4). A state-based action like the ones
            //     above, out here only because it has to read a permanent's
            //     abilities and `sba::run` has no lookup to read them with —
            //     which is 2c's reason too, and the whole of the difference
            //     between the two steps is that 2c's rules say in as many
            //     words that they are *not* state-based actions.
            if self.finished_sagas() {
                continue;
            }
            // 2c. Daybound and nightbound (CR 702.145c–g). Explicitly *not*
            //     state-based actions, and they need the card lookup, so
            //     they sit here rather than inside `sba::run`.
            if self.day_night_statics() {
                continue;
            }
            // 3. Triggers from new events.
            self.collect_triggers();
            if self.awaiting_answer {
                return; // a trigger's target choice is pending
            }
            // 3b. Delayed actions queued by upkeep processing.
            //
            // The `continue` is the whole point and was missing. A delayed
            // action *does things* — Venser's +2 returns a permanent to the
            // battlefield — and what it does is owed its triggers before
            // anybody receives priority (CR 603.3b), and its state-based
            // actions too (CR 117.5). Falling through to step 5 handed the
            // player priority with the entry still sitting unread in the
            // journal, so a board full of Allies watching for "another Ally
            // you control enters" said nothing until the next pass. The
            // owner reported it as the triggers never firing at all, which
            // is what it looks like from a chair: the turn ends, and the
            // answer arrives after the question is gone.
            if !self.delayed_queue.is_empty() {
                if self.process_delayed() {
                    return; // a delayed action produced a pending choice
                }
                continue;
            }
            // 4. Resolve the top of the stack after all passed.
            if self.resolve_next {
                self.resolve_next = false;
                self.passes = 0;
                self.priority_holder = None;
                self.resolve_stack_top();
                if self.awaiting_answer {
                    return; // resolution suspended on a choice
                }
                continue;
            }
            // 5. Progress the step machine.
            if self.progress_step() {
                return; // a pending choice was set
            }
        }
    }

    /// Applies the house rule for an endless loop. Returns `true` when the
    /// game ended and the caller must stop.
    ///
    /// `RunOnceThenBreak` breaks the loop rather than ending the game: the
    /// abilities feeding it stop reaching the stack, the stack drains, and
    /// play continues from whatever the loop left behind. That is the
    /// "resolve it once, then break it" house rule — the loop's effect has
    /// happened, it just stops happening forever. A card that starts the
    /// same loop again next upkeep is broken again next upkeep; the game
    /// keeps moving, which is the point.
    ///
    /// Detecting a loop *while a break is still in force* means the break
    /// did not take: the loop is driven by something other than triggers —
    /// replacement effects, state-based actions — and withholding triggers
    /// changed nothing. Repeating the attempt would be a slower hang, so
    /// that case falls back to the Comprehensive Rules answer (CR 104.4b:
    /// a draw), as does `CompRulesDraw` from the start.
    ///
    /// The watch is reset either way: whatever comes next is a new
    /// question, and Brent's saved state is about the loop just handled.
    pub(crate) fn on_loop_detected(&mut self, period: u64) -> bool {
        let break_it =
            self.house_rules.loop_policy == LoopPolicy::RunOnceThenBreak && !self.breaking_loop;
        self.state.journal.record(GameEvent::LoopDetected {
            period,
            broken: break_it,
        });
        self.action_loops = crate::loops::LoopWatch::default();
        if break_it {
            self.loops_broken += 1;
            self.breaking_loop = true;
            self.trigger_queue.clear();
            self.trigger_scan_seq = self.state.journal.last_seq();
            return false;
        }
        let result = GameResult {
            winner: None,
            reason: EndReason::Draw,
        };
        self.pending = Pending::GameOver(result);
        self.awaiting_answer = true;
        self.state
            .journal
            .record(GameEvent::GameWon { winner: None });
        true
    }

    /// How many endless loops this game has broken (diagnostic; the
    /// journal's `LoopDetected` entries are authoritative).
    #[must_use]
    pub const fn loops_broken(&self) -> u32 {
        self.loops_broken
    }

    /// Offers the next pending miracle cast (first-of-turn draws with
    /// miracle still in hand). Returns `true` when a choice was produced.
    pub(crate) fn offer_miracle(&mut self) -> bool {
        while let Some((player, card)) = self.state.pending_miracle.pop_front() {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            if obj.zone != crate::zone::Zone::Hand || obj.zone_owner != Some(player) {
                continue;
            }
            let Some(def) = obj.card.and_then(|c| self.lookup.card(c.index)) else {
                continue;
            };
            if def.faces[obj.face_index as usize].miracle.is_none() {
                continue;
            }
            let source = obj
                .card
                .map(|c| AbilityRef::new(c.index, AbilityRef::MIRACLE));
            self.pending_plan = Some(PlanKind::Miracle { card });
            self.pending = Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::Miracle { card },
                source,
            };
            self.awaiting_answer = true;
            return true;
        }
        false
    }

    /// One step-machine transition. Returns `true` when a pending choice
    /// was produced (loop must stop).
    pub(crate) fn progress_step(&mut self) -> bool {
        match self.state.turn.step {
            Step::Untap => self.untap_step(),
            Step::Cleanup => self.cleanup_step(),
            Step::DeclareAttackers if self.combat_declared != CombatDeclared::Attackers => {
                let attacker = self.state.turn.active;
                // The turn goes on without an active player (CR 800.4j), and
                // nobody declares attackers in their place: the declaration
                // is empty, and CR 508.8 skips what an empty one skips.
                if self.active_has_left() {
                    self.declare_attackers(attacker, Vec::new())
                        .expect("declaring no attackers is always legal");
                    return false;
                }
                let attackers: Vec<ObjectId> = self
                    .state
                    .zones
                    .list(crate::zone::ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .filter(|id| combat::can_attack(&self.state, attacker, *id))
                    .collect();
                self.pending = Pending::ChooseAttackers {
                    player: attacker,
                    attackers,
                    defenders: combat::defender_options(&self.state, attacker),
                };
                self.awaiting_answer = true;
                true
            }
            Step::DeclareBlockers if self.combat_declared != CombatDeclared::Blockers => {
                let active = self.state.turn.active;
                // Whoever is actually being attacked declares the blocks —
                // which, once planeswalkers can be attacked, is the walker's
                // controller and not merely the next seat along. With no
                // attackers there is nobody to ask, so the seat order stands.
                let defending = self
                    .state
                    .combat
                    .attackers
                    .first()
                    .and_then(|a| combat::defending_player(&self.state, a.defending))
                    .unwrap_or_else(|| self.next_alive_after(active));
                let attacking: Vec<ObjectId> = self
                    .state
                    .combat
                    .attackers
                    .iter()
                    .map(|a| a.creature)
                    .collect();
                // CR 702.111b restricts the declaration and not the pair, so
                // `can_block` cannot answer it — but an attacker this
                // defender could never field two legal blockers against is
                // one no legal declaration blocks, and offering that pairing
                // would name a block `declare_blockers` has to refuse (#156).
                // Asked once per attacker rather than once per pair, because
                // the answer is the same for every blocker.
                let blockable: Vec<ObjectId> = attacking
                    .iter()
                    .copied()
                    .filter(|a| combat::menace_satisfiable(&self.state, defending, *a))
                    .collect();
                let blockers: Vec<crate::choice::BlockOption> = self
                    .state
                    .zones
                    .list(crate::zone::ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .filter_map(|blocker| {
                        let attackers: Vec<ObjectId> = blockable
                            .iter()
                            .copied()
                            .filter(|a| combat::can_block(&self.state, defending, blocker, *a))
                            .collect();
                        (!attackers.is_empty())
                            .then_some(crate::choice::BlockOption { blocker, attackers })
                    })
                    .collect();
                self.pending = Pending::ChooseBlockers {
                    player: defending,
                    attacker: active,
                    blockers,
                };
                self.awaiting_answer = true;
                true
            }
            _ => self.priority_round(),
        }
    }

    /// Runs the APNAP priority round for steps that grant priority.
    /// Returns `true` when a pending choice was produced.
    pub(crate) fn priority_round(&mut self) -> bool {
        // The acting player gets priority straight back (CR 117.3c). It is
        // asked here rather than answered on the spot in `after_action`
        // because everything above this line in `run_machine` — the layer
        // projection, the state-based actions, the triggers — is owed to the
        // board the action left behind, and a question published before any
        // of it ran was a question about a board that no longer existed.
        //
        // First of the three arms, and it has to be: after an action
        // `priority_holder` is `Some` and `passes` is zero, so the round
        // below would read it as a round in progress and hand priority to
        // the *next* player.
        if let Some(mut player) = self.regrant_priority.take() {
            // A player who left the game in the course of their own action
            // has no priority to be given back: it passes to the next player
            // in turn order still in the game (CR 800.4a).
            if self.state.players[usize::from(player.get())].has_lost() {
                player = self.next_alive_after(player);
                self.priority_holder = Some(player);
            }
            self.pending = Pending::Priority {
                player,
                legal: Box::new(self.compute_legal(player)),
            };
            self.awaiting_answer = true;
            return true;
        }
        if self.priority_holder.is_none() && self.passes == 0 {
            // Open a new round with the active player (CR 117.3a), or, once
            // they have left the game, with the next player in turn order
            // (CR 800.4j).
            let active = self.state.turn.active;
            let first = if self.active_has_left() {
                self.next_alive_after(active)
            } else {
                active
            };
            self.priority_holder = Some(first);
            self.pending = Pending::Priority {
                player: first,
                legal: Box::new(self.compute_legal(first)),
            };
            self.awaiting_answer = true;
            return true;
        }
        let alive = self.alive_players();
        if self.passes >= alive.len() as u8 {
            // Round complete: resolve or advance.
            self.passes = 0;
            self.priority_holder = None;
            if self.state.zones.stack_is_empty() {
                // A miracle is offered here and not at the draw itself
                // (CR 702.94a). The reveal is a triggered ability and the
                // cast happens when it resolves, which is after a priority
                // window — and a priority window is the only place this
                // engine lets a player float mana, because a cost is paid
                // from the pool and CR 601.2g is compressed away. Asked at
                // the moment of the draw, a miracle was a question nobody
                // could ever answer yes to: the previous step ended, so the
                // pool was empty (CR 500.5), and the turn-based draw comes
                // before anybody holds priority (CR 504.1, then CR 504.2).
                // Answered either way, the round that follows is the one the
                // rules give the step anyway.
                if self.offer_miracle() {
                    return true;
                }
                // An upkeep payment that triggered as this step began
                // (`upkeep_payments`) is answered here, where it would
                // resolve had it been put on the stack: after everyone has
                // passed on an empty stack, and before `advance_step` ends
                // the step and empties the pool the player has just made its
                // mana into (CR 500.5). It goes through the delayed queue so
                // that what it does — a sacrificed echo permanent — is owed
                // its state-based actions and triggers before anybody holds
                // priority again (CR 117.5), and returning `false` with the
                // round reset above reopens it with the active player
                // (CR 117.3b).
                // The payments are the active player's own, and one who has
                // left the game is asked for nothing (CR 800.4a).
                if self.active_has_left() {
                    self.upkeep_payments.clear();
                }
                if let Some(action) = self.upkeep_payments.pop_front() {
                    // Filled only by `queue_upkeep_delayed`, and every upkeep
                    // closes a round of its own, so nothing can be left in it
                    // to be answered in a later step.
                    debug_assert_eq!(
                        self.state.turn.step,
                        Step::Upkeep,
                        "an upkeep payment outlived its upkeep"
                    );
                    self.delayed_queue
                        .push_back((self.state.turn.active, action));
                    return false;
                }
                self.advance_step();
            } else {
                self.resolve_next = true;
            }
            false
        } else {
            let current = self.priority_holder.expect("round started");
            let next = self.next_alive_after(current);
            self.priority_holder = Some(next);
            self.pending = Pending::Priority {
                player: next,
                legal: Box::new(self.compute_legal(next)),
            };
            self.awaiting_answer = true;
            true
        }
    }

    /// Applies as-it-enters-the-battlefield modifiers to permanents that
    /// entered since the last scan (CR 614.1c/d; taplands, shocklands).
    /// Returns whether a modifier wrote to the board, which the caller reads
    /// twice over: the legal lists are recomputed from it, and the machine
    /// goes round one more pass. The second is the load-bearing one — this
    /// runs *after* the projection two steps up, so a counter placed here is
    /// invisible to the state-based actions two steps down until they have
    /// been refreshed once more.
    #[allow(clippy::too_many_lines)] // the entry-modifier table is naturally flat
    pub(crate) fn apply_enter_modifiers(&mut self) -> bool {
        use baylee_cards_dsl::EnterModifier;
        // `from` travels with the arrival because one modifier reads it:
        // the X a `{X}{X}` body enters with belongs to the spell that
        // became this permanent (CR 107.3m), so it exists on the way in
        // off the stack and nowhere else.
        let events: Vec<(ObjectId, PlayerId, Zone)> = self
            .state
            .journal
            .entries()
            .get(self.entry_scan_seq as usize..)
            .unwrap_or_default()
            .iter()
            .filter_map(|e| match &e.event {
                GameEvent::ZoneChanged {
                    object,
                    from,
                    to: Zone::Battlefield,
                    ..
                } => Some((*object, *from)),
                _ => None,
            })
            .filter_map(|(id, from)| self.state.object(id).map(|o| (id, o.controller, from)))
            .collect();
        self.entry_scan_seq = self.state.journal.last_seq();
        let mut changed = false;
        for (id, controller, from_zone) in events {
            // CR 107.3m in one place, before anything reads it. The rule
            // gives an entering permanent's own abilities the X announced
            // for *the spell that became it*, and gives the permanent
            // itself an X of 0 — so the field means "the spell's number, or
            // nothing", and the one moment that is decidable is here, where
            // the arrival still knows which zone it came from. A Walking
            // Ballista that died at X = 1 and was reanimated, or one blinked
            // back from exile, was announced by nobody and comes down as the
            // 0/0 it prints (CR 107.3g: a card anywhere but the stack has an
            // X of 0); `move_object` deliberately carries the spell-shaped
            // fields through a zone change, so without this the old number
            // would still be sitting there.
            //
            // Normalised rather than guarded at each reader, because there
            // are now two of them and they are not alike: `WithCounters`
            // below is a replacement effect and could ask `from_zone`
            // itself, while an enters-the-battlefield *triggered* ability
            // is put on the stack by `collect_triggers` — a later step of
            // this same pass, with no arrival in its hands to ask.
            if from_zone != Zone::Stack
                && let Some(obj) = self.state.object_mut(id)
                && obj.x_value != 0
            {
                obj.x_value = 0;
            }
            // Daybound's first static ability (CR 702.145b): if it is
            // night, a permanent represented by a double-faced card
            // *enters* transformed. It is done here rather than left to
            // the fixpoint's later step because "enters transformed" means
            // the back face is what entered — the front face's own
            // enter-the-battlefield triggers were never on the board, and
            // `collect_triggers` runs after this in the same pass.
            if self.state.day_night == Some(DayNight::Night)
                && self
                    .state
                    .object(id)
                    .is_some_and(|o| o.face_index == 0 && o.zone == Zone::Battlefield)
                && let Some(def) = self
                    .state
                    .object(id)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))
                && def.faces.len() >= 2
                && def
                    .keywords_for_face(0)
                    .contains(baylee_cards_dsl::KeywordSet::DAYBOUND)
            {
                self.state.transform(id, def, 1);
                changed = true;
            }
            // Echo (CR 702.30): register the pay-or-sacrifice choice at
            // the controller's next upkeep.
            if let Some(cost) = self.state.object(id).and_then(|o| {
                o.abilities(&self.lookup).iter().find_map(|a| match a {
                    baylee_cards_dsl::AbilityDef::Echo { cost } => Some(*cost),
                    _ => None,
                })
            }) {
                self.state.delayed.push(crate::state::DelayedTrigger {
                    controller,
                    when: crate::state::DelayedWhen::NextUpkeep,
                    action: crate::state::DelayedAction::PayCostOrSacrifice { cost, card: id },
                });
            }
            // A Saga takes a lore counter as it enters (CR 714.3a), and
            // "As [this permanent] enters …" is named as a replacement
            // effect in CR 614.1c — so the counter goes through the door
            // that applies the multiplying replacements, exactly as a
            // planeswalker's starting loyalty does forty lines below. CR
            // 614.16 says nothing about Sagas; what it gives is the shape,
            // and this engine had already read it that way once.
            //
            // Which chapters that triggers is then a window and not a
            // number — see [`Self::queue_saga_chapters`]. Under a Doubling
            // Season two counters land at once and chapters I *and* II are
            // owed; this site matched `chapter: 1` and would have run the
            // first one only.
            if self.is_saga(id)
                && let Some(old) = self
                    .state
                    .object(id)
                    .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::Lore))
            {
                let ts = self.state.next_timestamp();
                crate::replacement::put_counters(
                    &mut self.state,
                    id,
                    baylee_cards_dsl::CounterKind::Lore,
                    1,
                );
                let new = self
                    .state
                    .object(id)
                    .map_or(old, |o| o.counters.get(baylee_cards_dsl::CounterKind::Lore));
                if let Some(obj) = self.state.object_mut(id) {
                    obj.timestamp = ts;
                }
                if self.queue_saga_chapters(id, controller, old, new, ts) {
                    changed = true;
                }
            }
            // Planeswalkers enter with their printed loyalty counters
            // (CR 306.5b).
            if let Some(loyalty) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .and_then(|def| {
                    let face = &def.faces[0];
                    if face
                        .types
                        .contains(baylee_core::types::TypeSet::PLANESWALKER)
                    {
                        face.loyalty
                    } else {
                        None
                    }
                })
            {
                // Starting loyalty is counters put on the permanent as it
                // enters, so the counter-placement replacements apply
                // (CR 614.16): Doubling Season doubles it.
                crate::replacement::put_counters(
                    &mut self.state,
                    id,
                    baylee_cards_dsl::CounterKind::Loyalty,
                    loyalty,
                );
                changed = true;
            }
            // Clone-on-enter, for every door that is not a permanent spell.
            //
            // A spell asks in `finalize_spell`, before the move, which is
            // what CR 614.12a requires — so by the time that arrival reaches
            // this scan the question has been answered and asking again would
            // ask it twice. `from_zone` is what separates the two and is
            // already in hand: a permanent spell resolves off the **stack**,
            // a token is created `from: OutsideGame`, and everything else
            // comes out of a graveyard, a library, exile or a hand.
            if from_zone != Zone::Stack && self.check_copy_on_enter(id) {
                return true;
            }
            let Some((card, face_index)) = self
                .state
                .object(id)
                .and_then(|o| Some((o.card?, o.face_index)))
            else {
                continue;
            };
            let Some(def) = self.lookup.card(card.index) else {
                continue;
            };
            // The permanent's *own* face, not the front one. A modal
            // double-faced card enters as its back face (Glasspool Shore is a
            // land that enters tapped), and reading `faces[0]` there asked the
            // creature half whether the land comes in tapped.
            let Some(face) = def.faces.get(face_index as usize) else {
                continue;
            };
            // A modifier that *asks* is applied last, whatever order the
            // card prints it in. Publishing a `Pending` returns from this
            // scan, and `entry_scan_seq` has already moved past this
            // arrival — so anything left in the loop behind the question
            // would never be applied at all. Uncharted Haven is
            // `ChooseColor` then `Tapped` and was entering untapped; the
            // same hole had been under `ChooseSubtype` and
            // `TappedOrPayLife` since they were written, invisible only
            // because no card in the pool prints another modifier after
            // one of them.
            //
            // The *first* question wins, and a card with two would lose the
            // second. No printed card asks twice as it enters, and the day
            // one does this is a second pass rather than a second field.
            let mut asked: Option<&EnterModifier> = None;
            for modifier in face.enter_modifiers {
                match modifier {
                    EnterModifier::Tapped => {
                        self.state.set_tapped(id, true);
                        changed = true;
                    }
                    EnterModifier::TappedUnless(filter) => {
                        if !self.controls_at_least(filter, controller, id, 1) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    EnterModifier::TappedUnlessCount { filter, at_least } => {
                        if !self.controls_at_least(filter, controller, id, usize::from(*at_least)) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    // The same count, the other way round: a fast land comes
                    // down tapped once the board is *past* its bound, which is
                    // the sentence a slow land's arm would answer backwards.
                    EnterModifier::TappedUnlessAtMost { filter, at_most } => {
                        if self.controls_count(filter, controller, id) > usize::from(*at_most) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    // Both of these count *players*, and neither re-derives
                    // which ones count. `eval::players` is the one place that
                    // knows a teammate is not an opponent and that a player
                    // who has lost is out of the game — a hand-rolled
                    // `self.state.players.len() - 1` here would have been
                    // right in a duel and wrong at every table these lands
                    // are actually printed for.
                    EnterModifier::TappedUnlessOpponents { at_least } => {
                        let opponents = eval::players(PlayerRel::Opponent, &self.state, controller)
                            .map_or(0, |seats| seats.len());
                        if opponents < usize::from(*at_least) {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    EnterModifier::TappedUnlessSomeoneAtOrBelow { life } => {
                        // "A player", so the controller is on the list too.
                        let low = eval::players(PlayerRel::EachPlayer, &self.state, controller)
                            .unwrap_or_default()
                            .into_iter()
                            .any(|seat| {
                                self.state
                                    .players
                                    .get(seat.get() as usize)
                                    .is_some_and(|p| p.life <= *life)
                            });
                        if !low {
                            self.state.set_tapped(id, true);
                            changed = true;
                        }
                    }
                    // CR 614.1c: "this enters with N counters on it" is a
                    // replacement effect, so it goes through the door that
                    // lets a counter doubler have its say — a Vivid land
                    // under a Doubling Season brings four charge counters,
                    // not two (CR 614.16).
                    EnterModifier::WithCounters { kind, amount } => {
                        // CR 107.3m: a replacement effect on a permanent that
                        // refers to X uses the X chosen for the spell that
                        // became that object as it resolved. `x_value` is
                        // that announced number and asks nothing further
                        // here, because the top of this loop has already put
                        // it back to 0 for an arrival that came from
                        // anywhere but the stack.
                        let x = Some(self.state.object(id).map_or(0, |o| o.x_value));
                        let n = crate::eval::amount(amount, &self.state, controller, id, x);
                        if n > 0 {
                            let n = u16::try_from(n).unwrap_or(u16::MAX);
                            crate::replacement::put_counters(&mut self.state, id, *kind, n);
                            changed = true;
                        }
                    }
                    EnterModifier::Prepared => {
                        if let Some(obj) = self.state.object_mut(id)
                            && !obj.riders.contains(&crate::object::Rider::Prepared)
                        {
                            obj.riders.push(crate::object::Rider::Prepared);
                        }
                    }
                    EnterModifier::ChooseSubtype
                    | EnterModifier::ChooseColor
                    | EnterModifier::ChooseColorExcept(_)
                    | EnterModifier::TappedOrPayLife(_)
                    | EnterModifier::TappedUnlessReveal(_) => {
                        asked.get_or_insert(modifier);
                    }
                }
            }
            match asked {
                None => {}
                Some(EnterModifier::ChooseSubtype) => {
                    self.pending_plan = Some(PlanKind::ChooseSubtype { object: id });
                    self.pending = Pending::ChooseSubtype {
                        player: controller,
                        options: (0..=349).map(baylee_core::ids::SubtypeId::new).collect(),
                    };
                    self.awaiting_answer = true;
                    return true; // one choice at a time
                }
                Some(m @ (EnterModifier::ChooseColor | EnterModifier::ChooseColorExcept(_))) => {
                    // Colorless is not a colour (CR 105.1), so it is not
                    // among the options even though `ManaColor` carries it:
                    // "choose a color" is one of the five.
                    let except = match m {
                        EnterModifier::ChooseColorExcept(c) => Some(*c),
                        _ => None,
                    };
                    let options: Vec<_> = baylee_cards_dsl::ALL_MANA_COLORS
                        .iter()
                        .copied()
                        .filter(|c| Some(*c) != except)
                        .collect();
                    self.pending_plan = Some(PlanKind::ChooseColor { object: id });
                    self.pending = Pending::ChooseColor {
                        player: controller,
                        options,
                    };
                    self.awaiting_answer = true;
                    return true; // one choice at a time
                }
                Some(EnterModifier::TappedOrPayLife(amount)) => {
                    let amount = *amount;
                    if self.state.can_pay_life(controller, i32::from(amount)) {
                        let source = self
                            .state
                            .object(id)
                            .and_then(|o| o.card)
                            .map(|c| AbilityRef::new(c.index, AbilityRef::ENTERS));
                        self.pending_plan = Some(PlanKind::EntryTap { object: id, amount });
                        self.pending = Pending::YesNo {
                            player: controller,
                            prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
                            source,
                        };
                        self.awaiting_answer = true;
                        return true; // one choice at a time
                    }
                    // Unpayable → tapped without a choice, and the scan goes
                    // on: nothing was asked, so nothing was interrupted.
                    self.state.set_tapped(id, true);
                    changed = true;
                }
                Some(EnterModifier::TappedUnlessReveal(filter)) => {
                    // The one clause in this family read against a hidden
                    // zone. Every `TappedUnless…` sibling asks
                    // `controls_at_least`, which walks the battlefield; a
                    // Faerie held in hand is on nobody's battlefield and the
                    // question would always answer no.
                    let filter: &Filter = filter;
                    let options: Vec<ObjectId> = self
                        .state
                        .zones
                        .list(ZoneLocation::Hand(controller))
                        .iter()
                        .filter(|card| {
                            self.state.object(**card).is_some_and(|o| {
                                eval::matches(filter, &self.state, o, controller, id)
                            })
                        })
                        .copied()
                        .collect();
                    if options.is_empty() {
                        // Nothing to show → tapped, and no question asked.
                        // The same branch an unpayable `TappedOrPayLife`
                        // takes, and for the same reason: a prompt whose
                        // only legal answer is "no" is not a choice, and
                        // offering it would hand the opponent the
                        // information that the hand is empty of Faeries.
                        self.state.set_tapped(id, true);
                        changed = true;
                    } else {
                        self.pending_plan = Some(PlanKind::EntryReveal { object: id });
                        self.pending = Pending::ChooseCards {
                            player: controller,
                            options,
                            // Naming nothing is how the offer is declined —
                            // the card prints "you may", and a `min` of one
                            // would make the reveal compulsory.
                            min: 0,
                            max: 1,
                            prompt: ChoicePrompt::RevealOrEnterTapped,
                        };
                        self.awaiting_answer = true;
                        return true; // one choice at a time
                    }
                }
                Some(_) => unreachable!("only the asking modifiers are recorded"),
            }
        }
        changed
    }

    /// Whether `controller` controls `at_least` permanents matching `filter`,
    /// not counting `entering` itself.
    ///
    /// Both enters-tapped-unless clauses ask this one question, so they share
    /// the answer: a checkland is the count with `at_least: 1`, and two
    /// conditions written twice would drift the first time one of them
    /// learned something.
    ///
    /// The entering permanent is left out because its own clause is a
    /// replacement applied on the way in, and ten cards in the pool can see
    /// the difference. A slow land prints "two or more **other** lands" and
    /// `landgen` reads that as `And(&[ControlledByYou, LAND])` — which a slow
    /// land matches — so this line is where the word "other" is spoken for
    /// Deserted Beach and its nine siblings. Count the entering land and
    /// every one of them comes in untapped a land early.
    ///
    /// It used to say the opposite: that nothing in the pool could see it,
    /// every enters-tapped-unless clause naming something the land is not,
    /// with Mystic Sanctuary as the one cycle that could and saying "other"
    /// itself. Mystic Sanctuary is a generated stub carrying no
    /// enter-modifier at all, so it was standing in for the ten cards that
    /// really do this — while
    /// `card_tests::a_slow_land_counts_the_other_lands_and_never_itself` had
    /// been playing one all along and asserting the opposite.
    ///
    /// A card *could* say it for itself: `Filter::Another` is read by `eval`
    /// as `obj.id != this`, and the `this` passed below is the entering
    /// permanent. None of the ten uses it. That is a division of labour
    /// rather than an oversight, and
    /// `card_tests::the_only_lands_that_would_count_themselves_are_the_slow_ones`
    /// is the fence around it — the list is asked of `eval::matches`, so a
    /// new cycle on either side of the split is a build failure.
    fn controls_at_least(
        &self,
        filter: &baylee_cards_dsl::Filter,
        controller: PlayerId,
        entering: ObjectId,
        at_least: usize,
    ) -> bool {
        self.controls_count(filter, controller, entering) >= at_least
    }

    /// The count both bounds read, and the one place the entering permanent
    /// is skipped.
    ///
    /// Two sentences ask about the same number from opposite ends — a slow
    /// land wants at least two other lands and a fast land wants at most two
    /// — so the comparison belongs to the modifier and the counting does not.
    /// Written twice, the word "other" would have had two places to go
    /// missing.
    fn controls_count(
        &self,
        filter: &baylee_cards_dsl::Filter,
        controller: PlayerId,
        entering: ObjectId,
    ) -> usize {
        // A phased-out Swamp does not exist for a checkland (CR 702.26b),
        // and a phased-out land does not count against a fastland (#209).
        self.state
            .battlefield_seen()
            .filter(|other| {
                *other != entering
                    && self.state.object(*other).is_some_and(|o| {
                        eval::matches(filter, &self.state, o, controller, entering)
                    })
            })
            .count()
    }

    /// Checks a newly entered permanent for a clone-on-enter clause and
    /// presents the copy choice when valid targets exist. Returns `true`
    /// when a pending choice was produced.
    /// The clone choice's two halves that do not depend on when it is asked:
    /// which ability is on the object, and what the battlefield offers it.
    ///
    /// Split out because the question is asked from **two** places now and
    /// the difference between them is only the timing — a shared body is what
    /// stops the two from drifting into offering different lists.
    fn copy_on_enter_question(&self, id: ObjectId) -> Option<(Vec<ObjectId>, PlayerId)> {
        let (spec, _) = self.copy_on_enter(id)?;
        let controller = self.state.object(id)?.controller;
        Some((
            eval::target_options(&spec, &self.state, controller, id),
            controller,
        ))
    }

    /// The clone ability on `id`: what it may copy, and what the copy
    /// changes. One reader for the offer above and for the agent's
    /// explanation of it (`decision_context`), so the two cannot name
    /// different abilities.
    pub(super) fn copy_on_enter(
        &self,
        id: ObjectId,
    ) -> Option<(TargetSpec, &'static [baylee_cards_dsl::CopyMod])> {
        self.state
            .object(id)?
            .abilities(&self.lookup)
            .iter()
            .find_map(|a| match a {
                AbilityDef::CopyOnEnter { target, mods }
                | AbilityDef::CopyOnEnterUntilEot { target, mods } => Some((*target, *mods)),
                _ => None,
            })
    }

    /// Publishes the clone choice and says whether it was published.
    ///
    /// `before_entry` is the rule, not a convenience. CR 614.12a: *"If a
    /// replacement effect that modifies how a permanent enters the
    /// battlefield requires a choice, that choice is made before the
    /// permanent enters the battlefield."* The answer carries it back so the
    /// handler knows whether it still owes the move.
    fn offer_copy_on_enter(
        &mut self,
        id: ObjectId,
        options: Vec<ObjectId>,
        controller: PlayerId,
        before_entry: bool,
    ) -> bool {
        if options.is_empty() {
            return false; // optional: simply doesn't copy
        }
        self.pending_plan = Some(PlanKind::CopyOnEnter {
            object: id,
            before_entry,
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
        true
    }

    /// The clone choice for a permanent **spell**, asked while it is still on
    /// the stack.
    ///
    /// This is CR 614.12a done rather than worked around. The permanent is
    /// not on the battlefield when the list is built, so it cannot be in it —
    /// which is also Glasspool Mimic's own ruling ("You may choose only a
    /// creature that's already on the battlefield") falling out of the timing
    /// instead of being spelled out by name. The `options.retain` that used
    /// to carry that ruling is gone from this path, and the test that
    /// replaced it asks where the Mimic *is* while the question stands, not
    /// whether it is in the list: a list it cannot be in is the stronger
    /// claim.
    ///
    /// The other half of the ruling — a creature entering at the same time is
    /// not a legal choice either — needs nothing here for the same reason it
    /// needed nothing before: permanents arrive one at a time.
    pub(crate) fn ask_copy_before_entry(&mut self, spell: ObjectId) -> bool {
        let Some((options, controller)) = self.copy_on_enter_question(spell) else {
            return false;
        };
        self.offer_copy_on_enter(spell, options, controller, true)
    }

    /// The same choice for a permanent that arrived by some **other** door.
    ///
    /// Reanimation, a search that puts a card onto the battlefield, a token
    /// copy of a clone — none of those go through `finalize_spell`, and this
    /// scan is the only place that sees them. They are still asked one step
    /// after the arrival, so the by-name exclusion below is still doing the
    /// work the timing does on the spell path, and it is still wrong in the
    /// same way the rules say it is: measured against the pool, every card
    /// carrying `CopyOnEnter` is a permanent spell, so the residue is a card
    /// put onto the battlefield by *another* card's effect.
    ///
    /// The Mimic is the one that reaches it: its errata'd filter is "a
    /// creature you control", and the permanent asking the question is one.
    /// Answering with itself made it a copy of a 0/0 Shapeshifter Rogue,
    /// which is the printed face and dies to the state-based action that
    /// follows. Cursed Mirror cannot: it asks as an artifact and its filter
    /// wants a creature.
    pub(crate) fn check_copy_on_enter(&mut self, id: ObjectId) -> bool {
        let Some((mut options, controller)) = self.copy_on_enter_question(id) else {
            return false;
        };
        options.retain(|&o| o != id);
        self.offer_copy_on_enter(id, options, controller, false)
    }

    /// Applies the clone-on-enter choice: the permanent's copiable base is
    /// replaced by the target's base, with the card's modifications. For
    /// `CopyOnEnterUntilEot` (Cursed Mirror), that half is a layer-1
    /// continuous effect with `UntilEndOfTurn` duration instead.
    ///
    /// Only that half differs. Both branches write the copied abilities onto
    /// the object, because nothing about an ability is layer-projected, and
    /// the temporary branch flags the write so [`Self::cleanup_step`] knows
    /// to take it back.
    ///
    /// One thing neither branch does on its own: a permanent with a
    /// *printed* static ability that becomes a copy keeps that static
    /// registered, because `sync_static_effects` registered it at step 0a of
    /// the pass this runs in at 0b, and only a departure un-registers one.
    ///
    /// That was for a long time an accident with a card standing on it.
    /// **Sakashima of a Thousand Faces** prints exactly such a static — "the
    /// legend rule doesn't apply to permanents you control" — and says
    /// "…except it has Sakashima's other abilities", which no [`CopyMod`]
    /// could express. The clause that could not be said and the effect that
    /// was never un-registered cancelled out, and the card was right for a
    /// reason that had nothing to do with what it prints.
    ///
    /// [`CopyMod::KeepOtherAbilities`] says it now, and
    /// [`Self::keep_own_statics`] pays it — so the outcome is the same and
    /// arrives by rule. The accident is gone from the spell door outright:
    /// CR 614.12a moved that choice in front of the permanent's arrival, so
    /// there is no 0a pass between the two for a static to be registered in.
    /// It survives only at the doors that copy *after* arrival —
    /// reanimation, a search to the battlefield, a token copy — where it is
    /// still reachable by a card that does **not** carry the mod, and
    /// `combo_tests::no_card_becomes_a_copy_carrying_a_printed_static_unnoticed`
    /// is what holds the pool to none.
    ///
    /// What is kept is the first half of CR 707.9a and not the second: the
    /// statics apply, and they do not join the copy's *copiable* values, so
    /// a second clone copying this one does not get them. The reason is a
    /// type — see [`CopyMod::KeepOtherAbilities`], which carries it.
    ///
    /// [`CopyMod`]: baylee_cards_dsl::CopyMod
    /// [`CopyMod::KeepOtherAbilities`]: baylee_cards_dsl::CopyMod::KeepOtherAbilities
    #[allow(clippy::too_many_lines)]
    pub(crate) fn apply_copy_choice(&mut self, id: ObjectId, target: ObjectId) {
        // The copier's own printed list is read *here* and not where it is
        // used, because both branches below overwrite `own_abilities` with
        // the copied one — after which `abilities` answers with the
        // target's text and the copier's own is no longer reachable from
        // the object at all.
        let (mods, until_eot, own_printed): (
            Vec<baylee_cards_dsl::CopyMod>,
            bool,
            &'static [AbilityDef],
        ) = {
            let Some(obj) = self.state.object(id) else {
                return;
            };
            let own = obj.abilities(&self.lookup);
            let (mods, until_eot) = own
                .iter()
                .find_map(|a| match a {
                    AbilityDef::CopyOnEnter { mods, .. } => Some((mods.to_vec(), false)),
                    AbilityDef::CopyOnEnterUntilEot { mods, .. } => Some((mods.to_vec(), true)),
                    _ => None,
                })
                .unwrap_or_default();
            (mods, until_eot, own)
        };
        let keeps_its_own = mods
            .iter()
            .any(|m| matches!(m, baylee_cards_dsl::CopyMod::KeepOtherAbilities));
        if until_eot {
            // Temporary copy: layer-1 effect + mods as their own effects.
            let controller = self
                .state
                .object(id)
                .map_or(PlayerId::new(0), |o| o.controller);
            let ts = self.state.next_timestamp();
            self.state
                .effects
                .register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller,
                    layer: baylee_cards_dsl::Layer::Copy,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter: crate::effects::EffectFilter::object(&self.state, id),
                    modifier: baylee_cards_dsl::Modifier::BecomeCopyOf(target),
                });
            // Abilities are copiable values too (CR 707.2), and the effect
            // above cannot carry them: `Modifier::BecomeCopyOf` assigns the
            // target's `characteristics()`, and abilities are not among them
            // — `Characteristics` has no field for a rules text, so nothing
            // about an ability is layer-projected. A Mirror that became a
            // Llanowar Elf was a 1/1 Elf Druid that still tapped for {R}.
            //
            // Read through `abilities` rather than off the target's card, for
            // the reason the permanent branch below does: a target that is
            // itself a copy answers with what it has become, which is what a
            // copy of it takes.
            let copied = self
                .state
                .object(target)
                .map(|o| o.ability_list(&self.lookup));
            if let Some(copied) = copied
                && let Some(obj) = self.state.object_mut(id)
            {
                obj.take_abilities(copied);
                // Unlike every other writer of that field. This copy ends
                // with the turn, and the field it writes is the one half of
                // the copy that cannot expire on its own.
                obj.own_abilities_until_eot = true;
            }
            if keeps_its_own {
                self.keep_own_statics(id, own_printed);
            }
            for m in mods {
                let (layer, modifier) = match m {
                    baylee_cards_dsl::CopyMod::AddKeyword(k) => (
                        baylee_cards_dsl::Layer::Ability,
                        baylee_cards_dsl::Modifier::AddKeyword(k),
                    ),
                    baylee_cards_dsl::CopyMod::AddType(t) => (
                        baylee_cards_dsl::Layer::Type,
                        baylee_cards_dsl::Modifier::AddType(t),
                    ),
                    baylee_cards_dsl::CopyMod::RemoveType(t) => (
                        baylee_cards_dsl::Layer::Type,
                        baylee_cards_dsl::Modifier::RemoveType(t),
                    ),
                    baylee_cards_dsl::CopyMod::AddSubtype(s) => (
                        baylee_cards_dsl::Layer::Type,
                        baylee_cards_dsl::Modifier::AddSubtype(s),
                    ),
                    // CR 707.9a: the ability the clause names, on the layer
                    // its modifier derives (CR 613.1f for a grant), ending
                    // with the turn like the rest of this copy.
                    baylee_cards_dsl::CopyMod::Grant(modifier) => (modifier.layer(), *modifier),
                    baylee_cards_dsl::CopyMod::AddCounter(kind, n) => {
                        // "…except it enters with an additional counter on
                        // it" is a replacement effect (CR 614.1c), and a
                        // counter-doubling replacement applies to what
                        // another replacement effect places (CR 614.16) —
                        // the same reading that already sends a
                        // planeswalker's starting loyalty through this
                        // door. It carries the journal entry and the
                        // invalidation too, the latter being what a copy
                        // arriving with counters needs (CR 613.4c), since
                        // nothing in the effect table moved to say so.
                        crate::replacement::put_counters(&mut self.state, id, kind, n);
                        continue;
                    }
                    // Two different reasons for one empty arm. There is no
                    // `Modifier` that takes a supertype away; and keeping
                    // the copier's own abilities is not a modification of
                    // what was copied at all but an addition beside it
                    // (CR 707.9a), already paid above and before this loop,
                    // while the copier's own list is still reachable.
                    baylee_cards_dsl::CopyMod::RemoveSupertype(_)
                    | baylee_cards_dsl::CopyMod::KeepOtherAbilities => continue,
                };
                let ts = self.state.next_timestamp();
                self.state
                    .effects
                    .register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(id),
                        controller,
                        layer,
                        timestamp: ts,
                        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                        filter: crate::effects::EffectFilter::object(&self.state, id),
                        modifier,
                    });
            }
            return;
        }
        // The target's copiable values rather than its `base`, because the
        // two disagree exactly when the target is *itself* a temporary copy:
        // a Cursed Mirror that became a Llanowar Elf has an artifact's base
        // and an Elf's copiable values, and the Mimic's own ruling says a
        // Mimic copying it "enters the battlefield as whatever the chosen
        // creature copied". The abilities beside it already read through
        // `abilities`, which follows `own_abilities` and so has always
        // answered with what the target became — the two halves of one
        // object were being read from two different places.
        let Some(target_base) = crate::layers::copiable_values(&self.state, target) else {
            return;
        };
        let Some(target_abilities) = self
            .state
            .object(target)
            .map(|o| o.ability_list(&self.lookup))
        else {
            return;
        };
        {
            let obj = self.state.object_mut(id).expect("copy target exists");
            // Kept so the copy can stop being one. A copy lasts as long as
            // the object does (CR 707.2a), and the object ends at the next
            // zone change (CR 400.7), which is where this is spent.
            if obj.original_base.is_none() {
                obj.original_base = Some(obj.base.clone());
            }
            obj.base = target_base;
            // Abilities are copiable values too (CR 707.2), and `base` holds
            // only characteristics — a copy that took the base alone arrived
            // with the right name and P/T and no rules text at all.
            obj.take_abilities(target_abilities);
        }
        if keeps_its_own {
            self.keep_own_statics(id, own_printed);
        }
        for m in mods {
            let obj = self.state.object_mut(id).expect("copy target exists");
            match m {
                baylee_cards_dsl::CopyMod::AddType(t) => {
                    let b = obj.base_mut();
                    b.types = b.types.union(t);
                }
                baylee_cards_dsl::CopyMod::RemoveType(t) => {
                    let b = obj.base_mut();
                    b.types = b.types.difference(t);
                }
                baylee_cards_dsl::CopyMod::RemoveSupertype(s) => {
                    let b = obj.base_mut();
                    b.supertypes = b.supertypes.difference(s);
                }
                baylee_cards_dsl::CopyMod::AddSubtype(s) => {
                    obj.base_mut().subtypes.insert(s);
                }
                baylee_cards_dsl::CopyMod::AddKeyword(k) => {
                    let b = obj.base_mut();
                    b.keywords = b.keywords.union(k);
                }
                baylee_cards_dsl::CopyMod::AddCounter(kind, n) => {
                    // The same door as the temporary branch above, for the
                    // same reason (CR 614.1c, CR 614.16). This is the arm
                    // a card in the pool actually reaches: Spark Double
                    // enters with one +1/+1 counter and one loyalty
                    // counter, and under a Doubling Season it enters with
                    // two of whichever it can hold.
                    crate::replacement::put_counters(&mut self.state, id, kind, n);
                }
                // Paid before this loop, for the reason the temporary
                // branch's twin gives.
                baylee_cards_dsl::CopyMod::KeepOtherAbilities => {}
                // CR 707.9a: "…except it has '…'". Registered the way the
                // copier's kept statics are (`keep_own_statics`) — its
                // timestamp, for as long as it stays on the battlefield —
                // because the copy has already taken away every ability the
                // card printed beside this clause (CR 707.2).
                baylee_cards_dsl::CopyMod::Grant(modifier) => {
                    let (controller, timestamp) = (obj.controller, obj.timestamp);
                    let filter = crate::effects::EffectFilter::object(&self.state, id);
                    self.state
                        .effects
                        .register(crate::effects::ContinuousEffect {
                            id: baylee_core::ids::EffectId::new(0),
                            source: Some(id),
                            controller,
                            layer: modifier.layer(),
                            timestamp,
                            duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                            filter,
                            modifier: *modifier,
                        });
                }
            }
        }
        self.state.invalidate_projections();
    }

    /// CR 707.9a: the copier keeps its own printed statics beside what it
    /// copied — "except it has Sakashima's other abilities".
    ///
    /// What it writes is the effect [`Self::sync_static_effects`] would have
    /// built, field for field, at the one moment the copier's own list is
    /// still reachable: once `own_abilities` holds the copied text, the
    /// printed one is gone from the object. So `has_source_ability` keeps
    /// that scan from registering a second copy of any of these, and the
    /// departure sweep at the top of it takes them away again — this
    /// function adds a moment, not a mechanism.
    ///
    /// The copy ability itself is excluded by the filter rather than by a
    /// test for it: it is not an [`AbilityDef::Static`], so "other" falls
    /// out of "static" for every card that can reach this. What does *not*
    /// fall out is a triggered or activated other ability, which has no
    /// continuous effect to live in and would be lost in silence —
    /// `combo_tests::every_copy_that_keeps_its_own_abilities_keeps_only_statics`
    /// is the bound that stops one arriving unnoticed.
    fn keep_own_statics(&mut self, id: ObjectId, printed: &'static [AbilityDef]) {
        let Some(obj) = self.state.object(id) else {
            return;
        };
        let (controller, timestamp) = (obj.controller, obj.timestamp);
        let mut to_register = Vec::new();
        for ability in printed {
            let AbilityDef::Static(sa) = ability else {
                continue;
            };
            if self.state.effects.has_source_ability(id, sa.modifier) {
                continue;
            }
            to_register.push(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(id),
                controller,
                layer: sa.layer,
                timestamp,
                duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                modifier: sa.modifier,
            });
        }
        for fx in to_register {
            self.state.effects.register(fx);
        }
    }

    /// Keeps the effect table in sync with the battlefield: registers
    /// static abilities of permanents, drops effects whose source left.
    pub(crate) fn sync_static_effects(&mut self) {
        use baylee_cards_dsl::Duration;
        // Drop effects whose source left the battlefield (structural
        // anthem removal).
        let gone: Vec<ObjectId> = self
            .state
            .effects
            .iter()
            .filter_map(|fx| fx.source)
            .filter(|s| {
                self.state
                    .object(*s)
                    .is_none_or(|o| o.zone != Zone::Battlefield)
            })
            .collect();
        self.state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::WhileSourceOnBattlefield)
                && fx.source.is_some_and(|s| gone.contains(&s))
        });
        // Collect statics of permanents not yet registered (then apply,
        // so the borrow of `state` ends before mutation).
        //
        // This scan and the replacement-rule scan at the bottom of the
        // function both read `GameObject::abilities` rather than the card
        // behind the permanent. It is the third place a permanent is asked
        // what it can do — the offer and the trigger scan had both already
        // been taught the question — and it is the one nobody notices,
        // because a static ability is something the machine registers
        // rather than something a player is offered and refused.
        //
        // Two kinds of copy were failing here, and only one of them for
        // the obvious reason. A token copy has no card at all, so the
        // whole permanent was skipped. A Glasspool Mimic *has* a card and
        // it is the wrong one: `check_copy_on_enter` writes the copied
        // list into `own_abilities` (CR 707.2), and reading the printed
        // face instead registered the Mimic's own statics, of which it has
        // none.
        //
        // A Mimic reanimated or searched onto the battlefield is registered
        // on the pass after the one it arrived on, because
        // `check_copy_on_enter` runs inside `apply_enter_modifiers`, one
        // step *after* this one: the pass it arrives on scans it before it
        // is a copy of anything, and the next pass picks it up — the
        // question below being whether the effect is already registered and
        // not whether the permanent has been looked at.
        //
        // A Mimic that was *cast* no longer reaches that window at all.
        // CR 614.12a puts the choice in front of the arrival, so
        // `finalize_spell` asks while the card is still on the stack and the
        // permanent enters already a copy — there is no pass on which this
        // scan sees it as itself. That is the half of the ordering the spell
        // door stopped needing, and the reason a permanent's own printed
        // statics have to be kept deliberately now
        // ([`Self::keep_own_statics`]) rather than by being registered
        // before the copy caught up.
        //
        // For the doors that are left, nothing happens in between, which is
        // the part worth knowing and is not luck. `check_copy_on_enter`
        // asks the controller which creature to copy, so it sets a pending
        // and the machine returns on `awaiting_answer` two lines later; the
        // pass that applies the answer begins again at step 0 and reaches
        // this scan and the one at the bottom before `collect_triggers` at
        // step 3. So a Mirror that
        // entered as a Katara *does* multiply the trigger its own arrival
        // caused, and no player is ever offered priority on a board where a
        // copy is missing half its rules text.
        //
        // A token copy is asked nothing and could not be saved that way:
        // `settle_copied_rules_text` hands it its list at step 0, ahead of
        // every scan in the pass, which is why that function runs where it
        // does.
        let ids: Vec<ObjectId> = self.state.zones.list(ZoneLocation::Battlefield).clone();
        let mut to_register = Vec::new();
        for id in ids {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            for ability in obj.abilities(&self.lookup) {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                if self.state.effects.has_source_ability(id, sa.modifier) {
                    continue;
                }
                to_register.push(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller: obj.controller,
                    layer: sa.layer,
                    timestamp: obj.timestamp,
                    duration: Duration::WhileSourceOnBattlefield,
                    filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                    modifier: sa.modifier,
                });
            }
        }
        for fx in to_register {
            self.state.effects.register(fx);
        }
        // Sync replacement rules (drop rules of departed sources, register
        // new ones).
        let gone_rules: Vec<ObjectId> = self
            .state
            .replacement_rules
            .iter()
            .map(|r| r.source)
            .filter(|s| {
                self.state
                    .object(*s)
                    .is_none_or(|o| o.zone != Zone::Battlefield)
            })
            .collect();
        self.state
            .replacement_rules
            .retain(|r| !gone_rules.contains(&r.source));
        let mut rules_to_add = Vec::new();
        for id in self.state.zones.list(ZoneLocation::Battlefield).clone() {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            for ability in obj.abilities(&self.lookup) {
                let AbilityDef::Replacement(rule) = ability else {
                    continue;
                };
                if self
                    .state
                    .replacement_rules
                    .iter()
                    .any(|r| r.source == id && r.rule == *rule)
                {
                    continue;
                }
                rules_to_add.push(crate::state::ReplacementEntry {
                    source: id,
                    controller: obj.controller,
                    rule: *rule,
                });
            }
        }
        self.state.replacement_rules.extend(rules_to_add);
    }

    fn alive_players(&self) -> Vec<PlayerId> {
        self.state
            .players
            .iter()
            .filter(|p| !p.has_lost())
            .map(|p| p.id)
            .collect()
    }

    /// Ends the game with `result`: the last question anybody is asked.
    pub(crate) fn end_game(&mut self, result: GameResult) {
        self.pending = Pending::GameOver(result);
        self.awaiting_answer = true;
        self.state.journal.record(GameEvent::GameWon {
            winner: result.winner,
        });
    }

    /// Whether `seat` is one of the `passes` players who have passed in
    /// succession ahead of `holder` in this priority round. Those are the
    /// players still in the game just before `holder` in turn order, because
    /// every pass hands priority to [`Engine::next_alive_after`] the passer.
    pub(crate) fn passed_before(&self, seat: PlayerId, holder: PlayerId) -> bool {
        let n = self.state.players.len() as u8;
        let mut at = holder.get();
        let mut counted = 0;
        while counted < self.passes {
            at = (at + n - 1) % n;
            if at == holder.get() {
                return false;
            }
            if self.state.players[usize::from(at)].has_lost() {
                continue;
            }
            if at == seat.get() {
                return true;
            }
            counted += 1;
        }
        false
    }

    /// Whether the active player has left the game. Their turn then goes on
    /// to its end without an active player (CR 800.4j): nobody takes their
    /// turn-based actions, and a priority they would receive goes to the
    /// next player in turn order.
    pub(crate) fn active_has_left(&self) -> bool {
        self.state.players[usize::from(self.state.turn.active.get())].has_lost()
    }

    pub(crate) fn next_alive_after(&self, player: PlayerId) -> PlayerId {
        let n = self.state.players.len() as u8;
        let start = player.get();
        for offset in 1..=n {
            let candidate = PlayerId::new((start + offset) % n);
            if !self.state.players[candidate.get() as usize].has_lost() {
                return candidate;
            }
        }
        player
    }

    pub(crate) fn game_result(&self) -> Option<GameResult> {
        // An agreed draw ends the game with players still alive, so it has
        // to be checked before the last-player-standing count (CR 104.4i).
        if self.agreed_draw {
            return Some(GameResult {
                winner: None,
                reason: EndReason::Draw,
            });
        }
        // The game is decided by *sides*, not by heads: a seat with no team
        // is a side of one, so a game with no teams in it counts exactly the
        // players it counted before. A team wins the moment nobody from
        // another side is left standing, however many of its own members
        // died getting there (CR 104.2c).
        let mut sides: Vec<Side> = Vec::new();
        for player in self.alive_players() {
            let side = self.state.side_of(player);
            if !sides.contains(&side) {
                sides.push(side);
            }
        }
        match sides.as_slice() {
            [] => Some(GameResult {
                winner: None,
                reason: EndReason::Draw,
            }),
            [Side::Solo(player)] => Some(GameResult {
                winner: Some(Victor::Player(*player)),
                reason: EndReason::LastPlayerStanding,
            }),
            [Side::Team(team)] => Some(GameResult {
                winner: Some(Victor::Team(*team)),
                reason: EndReason::LastTeamStanding,
            }),
            _ => None,
        }
    }

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
    fn trigger_abilities(
        &self,
        t: &crate::trigger::PendingTrigger,
    ) -> &'static [baylee_cards_dsl::AbilityDef] {
        t.abilities.map_or_else(
            || {
                self.state
                    .object(t.source)
                    .map_or(&[][..], |o| o.abilities(&self.lookup))
            },
            |list| list.abilities,
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
        if let Some(abilities) = t.abilities {
            self.activating_abilities = Some((t.source, abilities));
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
        let objects = eval::target_options(&req.spec, &self.state, t.controller, t.source).len();
        let players = eval::target_player_options(&self.state, &req.spec, t.controller).len();
        objects + players >= req.min as usize
    }

    /// Remember event-time abilities before a state-based action removes them.
    fn queue_new_triggers(&mut self) {
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
        self.trigger_queue.extend(found);
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
            return;
        }
        self.queue_new_triggers();
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
                && self
                    .state
                    .ability_fires
                    .contains_key(&(t.source, t.ability_index))
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
                    // stack." Nothing to ask and nothing to resolve.
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
                        self.state
                            .ability_fires
                            .insert((t.source, t.ability_index), 1);
                    }
                    self.hand_over_trigger_abilities(&t);
                    self.push_ability_to_stack(t.controller, t.source, t.ability_index, targets);
                    self.set_top_mode(t.chosen_mode);
                    if let Some(event_object) = t.event_object {
                        let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                        if let Some(top) = top
                            && let Some(obj) = self.state.object_mut(top)
                        {
                            obj.event_object = Some(event_object);
                        }
                    }
                    continue;
                }
                let options = eval::target_options(&req.spec, &self.state, t.controller, t.source);
                // A trigger may point at a player as readily as a spell does
                // ("it deals 1 damage to target opponent"), and "any target"
                // offers both lists at once (CR 115.4). The choice is one
                // choice, so the counts add up.
                let player_options =
                    eval::target_player_options(&self.state, &req.spec, t.controller);
                let offered = options.len() + player_options.len();
                if offered < req.min as usize {
                    // No legal target: the trigger is removed from the stack
                    // entirely (CR 603.3d).
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
                if offered > 0 {
                    self.pending_plan = Some(PlanKind::Trigger {
                        source: t.source,
                        ability_index: t.ability_index,
                        mode: t.chosen_mode,
                    });
                    let max = req.max.min(offered as u8);
                    self.pending = Pending::ChooseTargets {
                        player: t.controller,
                        options,
                        player_options,
                        min: req.min,
                        max,
                        reason: TargetPrompt::Targets,
                    };
                    self.awaiting_answer = true;
                    return;
                }
            }
            self.trigger_queue.pop_front();
            if t.once_per_turn {
                self.state
                    .ability_fires
                    .insert((t.source, t.ability_index), 1);
            }
            // Synthetic triggers with a target requirement (granted
            // triggered abilities): ask for the target first.
            if t.synthetic_effects.is_some()
                && let Some(spec) = t.synthetic_target
            {
                let options = eval::target_options(&spec, &self.state, t.controller, t.source);
                if options.is_empty() {
                    // No legal target, so the trigger is removed (CR 603.3d)
                    // — and it is *already* removed: the pop above took this
                    // queue entry off before the synthetic target was asked
                    // about. Popping again here took the trigger queued
                    // behind it as well, unread and unresolved.
                    continue;
                }
                let plan_t = t.clone();
                self.pending_plan = Some(PlanKind::SyntheticTriggerTarget { trigger: plan_t });
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
                    // The event object is usually carried *twice*, and the
                    // two halves are read by different things. As the
                    // implicit target it is what prowess pumps and what ward
                    // counters; as `event_object` it is what
                    // `TargetSpec::EventObject` resolves through
                    // (`resolve::zones::spec_object`). Only the non-synthetic
                    // branch below wrote the second one, so a synthetic effect
                    // list naming the event object read `None` and did
                    // nothing — undying and persist put a trigger on the stack
                    // that resolved into silence.
                    obj.event_object = t.event_object;
                    obj
                });
                self.synthetic_fx.insert(id, synthetic);
                self.state
                    .zones
                    .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
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
                // Carry the event object onto the fresh stack object.
                if let Some(event_object) = t.event_object {
                    let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                    if let Some(top) = top
                        && let Some(obj) = self.state.object_mut(top)
                    {
                        obj.event_object = Some(event_object);
                    }
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
    fn intervening_if_failed(&self, on_stack: ObjectId) -> bool {
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
        let abilities = obj.own_abilities.unwrap_or_else(|| {
            self.state
                .object(loc.source)
                .map_or(&[][..], |o| o.abilities(&self.lookup))
        });
        let condition = match abilities.get(loc.index as usize) {
            Some(
                AbilityDef::Triggered { condition, .. }
                | AbilityDef::ModalTriggered { condition, .. },
            ) => *condition,
            _ => None,
        };
        !crate::eval::intervening_if(&self.state, condition, obj.controller, loc.source)
    }

    /// What the object on top of the stack is allowed to target.
    ///
    /// One arm list for a question that is asked twice — CR 608.2b's
    /// re-check below, and `Resolution::targeted`, which is what tells
    /// `Filter::This` apart from itself. It was written out once before and
    /// the second copy would have been the third: `AbilityDef::Activated`
    /// and `AbilityDef::ActivatedConditional` are twins that six readers
    /// across this workspace have already matched one of and not the other.
    ///
    /// A spell carries its requirement on the object instead of in the card:
    /// the cast wizard writes `target_req` there so a copy can be retargeted
    /// without a lookup (CR 707.10c), and this is a second reader with the
    /// same reason — by resolution the mode, the face and the copy status
    /// have all been settled and the object is where they were settled.
    fn stack_target_req(&self, on_stack: ObjectId) -> Option<TargetReq> {
        let obj = self.state.object(on_stack)?;
        if obj.kind != ObjectKind::AbilityOnStack {
            return obj.target_req;
        }
        let loc = obj.ability?;
        if loc.index == AbilityRef::SYNTHETIC {
            // A synthetic trigger prints nothing to read a requirement off,
            // so it carries one on its object when it has one: a granted
            // triggered ability's target, or a reflexive one's. Prowess and
            // ward carry none and answer `None`, as before. Answering `None`
            // for all of them meant CR 608.2b never re-checked a synthetic
            // target. Eden's reflexive ability returned a card that had been
            // exiled in response, from exile to hand.
            return obj.target_req;
        }
        let abilities = obj.own_abilities.unwrap_or_else(|| {
            self.state
                .object(loc.source)
                .map_or(&[][..], |o| o.abilities(&self.lookup))
        });
        match abilities.get(loc.index as usize)? {
            AbilityDef::Activated { targets, .. }
            | AbilityDef::ActivatedConditional { targets, .. }
            | AbilityDef::Loyalty { targets, .. }
            | AbilityDef::Triggered { targets, .. }
            | AbilityDef::SagaChapter { targets, .. } => *targets,
            AbilityDef::ModalTriggered { modes, .. } => {
                // The same `expect` the resolution path below makes, and for
                // the reason written there: the mode is announced as the
                // ability goes on the stack (CR 603.3c), so one that reached
                // resolution without it came off a push site that forgot to
                // carry it. Falling back to the first mode is how a card
                // resolves the wrong half of itself in silence.
                let idx = obj
                    .mode_index
                    .expect("a modal trigger on the stack has its mode")
                    as usize;
                modes.get(idx).and_then(|m| m.targets)
            }
            _ => None,
        }
    }

    /// What the top of the stack may target with its **second** instance of
    /// the word "target", read from the same places [`Self::stack_target_req`]
    /// reads the first: the spell's own object, or the ability's definition.
    ///
    /// Two arms and not the whole list, because only two shapes can say it —
    /// [`AbilityDef::Spell`] through the object and the activated twins here.
    /// Both twins, for the reason `stack_target_req` gives.
    fn stack_second_target_req(&self, on_stack: ObjectId) -> Option<TargetReq> {
        let obj = self.state.object(on_stack)?;
        if obj.kind != ObjectKind::AbilityOnStack {
            return obj.second_target_req();
        }
        let loc = obj.ability?;
        if loc.index == AbilityRef::SYNTHETIC {
            return None;
        }
        let abilities = obj.own_abilities.unwrap_or_else(|| {
            self.state
                .object(loc.source)
                .map_or(&[][..], |o| o.abilities(&self.lookup))
        });
        match abilities.get(loc.index as usize)? {
            AbilityDef::Activated { second_targets, .. }
            | AbilityDef::ActivatedConditional { second_targets, .. } => *second_targets,
            _ => None,
        }
    }

    /// CR 608.2b, asked of the top of the stack as it begins to resolve.
    ///
    /// > If the spell or ability specifies targets, it checks whether the
    /// > targets are still legal. […] If all its targets, for every instance
    /// > of the word "target," are now illegal, the spell or ability doesn't
    /// > resolve.
    ///
    /// Each instance is re-checked by [`Self::instance_legality`], which
    /// says how; this is where the instances are put back together.
    ///
    /// **What this cannot see.** `GameObject::targets` holds a bare
    /// `ObjectId`, and a creature blinked in response really is back on the
    /// battlefield — so it enumerates as legal although CR 400.7 makes it a
    /// new object. No zone test can catch that; only the `(ObjectId,
    /// version)` pair `object.rs`'s own header prescribes can, which is #117
    /// and is the same defect one layer over.
    fn target_legality(&self, on_stack: ObjectId) -> TargetLegality {
        let Some(obj) = self.state.object(on_stack) else {
            return TargetLegality::NotAsked;
        };
        // "For every instance of the word 'target'": each instance is asked
        // on its own, against its own requirement, and the spell fizzles
        // only when every instance that chose something has lost all of it.
        // A fight whose second creature was bounced still resolves — the
        // first target is legal — and does nothing to either creature,
        // which is CR 701.14b's business in the resolver and not this one's.
        let first = self
            .stack_target_req(on_stack)
            .and_then(|req| self.instance_legality(obj, req, &obj.targets, true));
        let second = self
            .stack_second_target_req(on_stack)
            .and_then(|req| self.instance_legality(obj, req, obj.second_targets(), false));
        if first.is_none() && second.is_none() {
            return TargetLegality::NotAsked;
        }
        let lost = |kept: &Option<(SmallVec<[ObjectId; 2]>, SeatSet)>| {
            kept.as_ref()
                .is_none_or(|(objects, players)| objects.is_empty() && players.is_empty())
        };
        if lost(&first) && lost(&second) {
            return TargetLegality::AllIllegal;
        }
        // An instance that was not asked keeps what it holds.
        let (objects, players) = first.unwrap_or_else(|| (obj.targets.clone(), obj.target_players));
        let second = second.map_or_else(
            || SmallVec::from_slice(obj.second_targets()),
            |(objects, _)| objects.into_iter().collect(),
        );
        TargetLegality::Kept {
            objects,
            players,
            second,
        }
    }

    /// One instance of the word "target", re-checked: what it chose that is
    /// still legal, or `None` when there is nothing of it to ask.
    ///
    /// The question is asked with the very enumeration that offered the
    /// targets in the first place — `eval::stack_target_options`, which is
    /// `eval::target_options` and `eval::target_player_options` with the same
    /// `(you, this)` the cast wizard and `ability_has_a_target` pass. One
    /// predicate read from both ends: an offer and a re-check that disagreed
    /// would be a target the engine let a player choose and then refused to
    /// resolve at. A change of targets (CR 115.7) asks it too.
    ///
    /// `players` is whether this instance is the one whose players ride in
    /// `target_players` — the first, since a second instance is objects only
    /// in every shape that can print one.
    fn instance_legality(
        &self,
        obj: &crate::object::GameObject,
        req: TargetReq,
        chosen: &[ObjectId],
        players: bool,
    ) -> Option<(SmallVec<[ObjectId; 2]>, SeatSet)> {
        // Two specs name no *chosen* target. `EventObject` is the object the
        // trigger fired on, and `Player(rel)` derives its players from a
        // relation at resolution — `resolve::players_of` reads neither list
        // for it. Both enumerate empty by construction, so asking them this
        // question would read every target they have as illegal.
        if matches!(req.spec, TargetSpec::EventObject | TargetSpec::Player(_)) {
            return None;
        }
        // The player half exists only for the three specs that can name one;
        // `eval::targeted_players` says why the bound matters.
        let chosen_players = if players {
            eval::targeted_players(obj, &req.spec)
        } else {
            SeatSet::new()
        };
        // CR 608.2b is about targets that were chosen. A requirement with a
        // minimum of zero, taken with nothing pointed at, has none to lose —
        // and an empty list satisfies "all of them are illegal" vacuously,
        // which would fizzle every untargeted half of the pool.
        if chosen.is_empty() && chosen_players.is_empty() {
            return None;
        }
        let (legal_objects, legal_players) =
            eval::stack_target_options(&self.state, obj, &req.spec);
        let objects: SmallVec<[ObjectId; 2]> = chosen
            .iter()
            .copied()
            .filter(|id| legal_objects.contains(id))
            .collect();
        let mut kept = SeatSet::new();
        for player in chosen_players.iter() {
            if legal_players.contains(&player) {
                kept.insert(player);
            }
        }
        Some((objects, kept))
    }

    /// CR 608.2b's removal: off the stack, without having resolved.
    ///
    /// **Not [`Self::finalize_spell`]**, which is the path a spell that *did*
    /// resolve takes and owes two riders this one does not. Rebound exiles a
    /// spell "as it resolves" (CR 702.88) and an Adventure likewise
    /// (CR 715.3d); a spell that never resolved has done neither, and
    /// borrowing that function would have given a fizzled Ephemerate its
    /// rebound. Flashback is the rider that *does* apply, because CR 702.34a
    /// exiles the card "any time it would leave the stack" rather than on
    /// resolution.
    fn leave_stack_without_resolving(&mut self, top: ObjectId) {
        let Some(obj) = self.state.object(top) else {
            return;
        };
        if obj.kind == ObjectKind::AbilityOnStack {
            // An ability ceases to exist rather than going anywhere
            // (CR 608.2n) — the same removal CR 603.4's arm above makes.
            self.state.zones.remove(top, ZoneLocation::Stack);
            let _ = self.state.arena.remove(top);
            return;
        }
        let owner = obj.owner;
        let flashback = obj.riders.contains(&crate::object::Rider::Flashback);
        if let Some(obj) = self.state.object_mut(top) {
            obj.kind = ObjectKind::Card;
        }
        let destination = if flashback {
            ZoneLocation::Exile(owner)
        } else {
            ZoneLocation::Graveyard(owner)
        };
        // `Cause::Spell` and not `Cause::Effect`: nothing's effect moved this
        // card. It is the same clause's other half — CR 608.2n puts a
        // resolved spell in its owner's graveyard and `finalize_spell` calls
        // that `Cause::Spell` — arriving by the door one sentence earlier.
        let _ = self
            .state
            .move_object(top, destination, ZonePosition::Top, Cause::Spell);
    }

    #[allow(clippy::too_many_lines)] // resolution dispatch is a flat router; extraction would obscure it
    pub(crate) fn resolve_stack_top(&mut self) {
        let Some(&top) = self.state.zones.list(ZoneLocation::Stack).last() else {
            return;
        };
        // CR 603.4, asked before anything records that this resolved: an
        // ability whose intervening-`if` clause is no longer true "is removed
        // from the stack and does nothing". Not a counter and not a
        // resolution — a journal that said `StackObjectResolved` here and
        // then took the ability away would be describing a different rule to
        // everything that reads it.
        if self.intervening_if_failed(top) {
            self.state
                .journal
                .record(GameEvent::StackObjectDidNotResolve { object: top });
            // As when an ability is countered: an ability on the stack
            // ceases to exist rather than going anywhere (CR 608.2n).
            self.state.zones.remove(top, ZoneLocation::Stack);
            let _ = self.state.arena.remove(top);
            return;
        }
        // CR 608.2b, asked in the same place and for the same reason: a
        // spell or ability all of whose targets have become illegal does not
        // resolve. Before the spell/ability split below, because an Aura is
        // a targeted *permanent* spell and a check inside either branch
        // would miss one of them.
        match self.target_legality(top) {
            TargetLegality::AllIllegal => {
                self.state
                    .journal
                    .record(GameEvent::StackObjectDidNotResolve { object: top });
                self.leave_stack_without_resolving(top);
                return;
            }
            // "…won't do anything to an illegal target" (CR 608.2b): the
            // rest of it still happens. Narrowed once, here, rather than in
            // each of the four `Resolution`s built below — all four read
            // `obj.targets.clone()`, so the one write is what keeps them
            // from disagreeing.
            //
            // Safe to narrow because a `TargetReq` carries **one** spec: the
            // positions in `targets` are a set and not a tuple, and nothing
            // in this crate reads one by index. A second instance of the
            // word "target" is the one place that would not hold, which is
            // why it is a list of its own and is narrowed on its own line:
            // a fight whose first creature became illegal must not find its
            // second creature standing in the first one's place.
            TargetLegality::Kept {
                objects,
                players,
                second,
            } => {
                if let Some(obj) = self.state.object_mut(top) {
                    if obj.targets.len() != objects.len() {
                        obj.targets = objects;
                    }
                    if obj.second_targets().len() != second.len() {
                        let req = obj.second_target_req();
                        obj.set_second(second, req);
                    }
                    if obj.target_players != players {
                        obj.target_players = players;
                        // `chosen_player` is the same choice written twice
                        // and `resolve::players_of` reads *it* for
                        // `PlayerRel::Chosen`. Left behind, a player who
                        // gained hexproof in response would still be dealt
                        // to by the half of the spell that reads the scalar.
                        obj.chosen_player = obj.chosen_player.filter(|p| players.contains(*p));
                    }
                }
            }
            TargetLegality::NotAsked => {}
        }
        self.state
            .journal
            .record(GameEvent::StackObjectResolved { object: top });
        let kind = self.state.object(top).map(|o| o.kind);
        if kind == Some(ObjectKind::AbilityOnStack) {
            let obj = self.state.object(top).expect("stack object exists");
            let loc = obj.ability.expect("ability object has a location");
            // The list `loc.index` points into, captured when the ability was
            // put on the stack — a card face, an emblem's stored list, a
            // token's definition or a copy's, all the same case by then. It
            // is read from the ability object rather than from the source
            // because the two have been separate objects since it was put
            // there (CR 113.7a): the source may have died, changed face, or
            // stopped being a copy in the meantime.
            let abilities = obj.own_abilities.unwrap_or_else(|| {
                self.state
                    .object(loc.source)
                    .map_or(&[][..], |o| o.abilities(&self.lookup))
            });
            let effects = if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
                // Synthetic keyword trigger (prowess, ward): effects live
                // in the side map, resolved below.
                &[][..]
            } else {
                match abilities.get(loc.index as usize) {
                    // `ActivatedConditional` belongs here beside `Activated`:
                    // the condition is a restriction on *activating* it —
                    // CR 602.5, "a player can't begin to activate an ability
                    // that's prohibited from being activated" — checked once
                    // in `start_activation` and spent there. What reaches the
                    // stack is an ordinary ability,
                    // and leaving it out of this arm meant every conditional
                    // ability that uses the stack panicked the engine as it
                    // resolved — Wizard Class could be levelled and not
                    // survive it. The `targeted` match just below had it all
                    // along, which is why nothing else noticed.
                    Some(
                        AbilityDef::Activated { effects, .. }
                        | AbilityDef::ActivatedConditional { effects, .. }
                        | AbilityDef::Triggered { effects, .. }
                        | AbilityDef::Loyalty { effects, .. }
                        | AbilityDef::SagaChapter { effects, .. },
                    ) => *effects,
                    Some(AbilityDef::ModalTriggered { modes, .. }) => {
                        // `expect` and not "mode 0 if nobody said": the mode
                        // is announced as the ability is put on the stack
                        // (CR 603.3c), so an ability that reached resolution
                        // without one came off a push site that forgot to
                        // carry it — and falling back to the first mode is
                        // how a card resolves the wrong half of itself in
                        // silence, which is the fault entry 34 is about.
                        let idx = obj
                            .mode_index
                            .expect("a modal trigger on the stack has its mode")
                            as usize;
                        modes
                            .get(idx)
                            .map(|m| m.effects)
                            .expect("modal trigger mode exists")
                    }
                    _ => panic!(
                        "ability object references non-resolvable ability: source {:?} index {}",
                        loc.source, loc.index
                    ),
                }
            };
            // Whether this ability said "target" at all, which is what tells
            // `Filter::This` apart from itself — see `Resolution::targeted`.
            //
            // The arm list this used to spell out is `stack_target_req`,
            // which CR 608.2b's check above already needs: two copies of a
            // match over `Activated` and `ActivatedConditional` is two
            // chances to add a variant to one of them, and this workspace
            // has found six readers matching one twin and not the other.
            let targeted = self.stack_target_req(top).is_some();
            if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC {
                // Synthetic keyword trigger (prowess & co.): effects live in
                // the side map instead of the card definition.
                let synthetic = self
                    .synthetic_fx
                    .remove(&top)
                    .expect("synthetic trigger has effects");
                let mut res = Resolution {
                    source: loc.source,
                    on_stack: top,
                    controller: obj.controller,
                    effects: resolve::flatten(synthetic),
                    pc: 0,
                    targets: obj.targets.clone(),
                    second_targets: SmallVec::from_slice(obj.second_targets()),
                    x: None,
                    chosen_player: obj.chosen_player,
                    target_players: obj.target_players,
                    event_object: obj.event_object,
                    // Whether it said "target", which a synthetic trigger
                    // with a requirement did: see `stack_target_req`.
                    targeted,
                    awaiting: None,
                    mana_ability: false,
                    countered_source: None,
                    target_lki: None,
                };
                match resolve::run(&mut self.state, &mut res) {
                    resolve::Flow::Complete => self.finish_resolution(&res),
                    resolve::Flow::Wait(pending) => {
                        self.resolution = Some(res);
                        self.pending = pending;
                        self.awaiting_answer = true;
                    }
                }
                return;
            }
            let mut res = Resolution {
                source: loc.source,
                on_stack: top,
                controller: obj.controller,
                effects: resolve::flatten(effects),
                pc: 0,
                targets: obj.targets.clone(),
                second_targets: SmallVec::from_slice(obj.second_targets()),
                // Zero on every ability the pool prints today, and read
                // rather than assumed because an activation with a counter-X
                // cost writes one here (`push_ability_to_stack`). `Some(0)`
                // and `None` are the same number to `eval::amount`.
                x: Some(obj.x_value),
                chosen_player: obj.chosen_player,
                target_players: obj.target_players,
                event_object: obj.event_object,
                targeted,
                awaiting: None,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
            };
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
            return;
        }
        // A spell resolves (plain spells and modal spells alike — the
        // chosen mode is stored on the spell object).
        let spell_fx = self
            .state
            .object(top)
            .and_then(|o| {
                let face = o.face_index as usize;
                o.card
                    .and_then(|c| self.lookup.card(c.index))
                    .map(|def| def.abilities_for_face(face))
            })
            .and_then(|abilities| {
                abilities.iter().find_map(|a| match a {
                    AbilityDef::Spell {
                        effects, targets, ..
                    } if !effects.is_empty() => Some((*effects, targets.is_some())),
                    _ => None,
                })
            })
            .or_else(|| {
                let mode_index = self.state.object(top)?.mode_index?;
                let face = self.state.object(top)?.face_index as usize;
                let def = self
                    .state
                    .object(top)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))?;
                def.abilities_for_face(face).iter().find_map(|a| match a {
                    AbilityDef::ModalSpell { modes } => modes
                        .get(mode_index as usize)
                        .map(|m| (m.effects, m.targets.is_some())),
                    _ => None,
                })
            });
        if let Some((fx, targeted)) = spell_fx {
            let obj = self.state.object(top).expect("stack object exists");
            let mut res = Resolution {
                source: top,
                on_stack: top,
                controller: obj.controller,
                effects: resolve::flatten(fx),
                pc: 0,
                targets: obj.targets.clone(),
                second_targets: SmallVec::from_slice(obj.second_targets()),
                x: Some(obj.x_value),
                chosen_player: obj.chosen_player,
                target_players: obj.target_players,
                event_object: None,
                targeted,
                awaiting: None,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
            };
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
        } else {
            self.finalize_spell(top);
        }
    }

    /// Applies a face switch queued by a resolution effect (transforms).
    pub(crate) fn apply_pending_face_changes(&mut self) {
        let pending: Vec<(ObjectId, u8)> = self
            .state
            .arena
            .iter()
            .filter_map(|(id, o)| o.pending_face_change.map(|f| (id, f)))
            .collect();
        for (id, face) in pending {
            if let Some(obj) = self.state.object_mut(id) {
                obj.pending_face_change = None;
            }
            if let Some(def) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            {
                self.state.transform(id, def, face as usize);
            }
        }
    }

    /// Hands every token copy the printed rules text it was created owing
    /// (CR 707.2), from [`GameState::pending_copied_faces`].
    ///
    /// The same division of labour as [`Self::day_night_statics`] below, for
    /// the same reason: a face's abilities are behind the card registry,
    /// `resolve` has no lookup for it and cannot be given one — the rules
    /// kernel does not depend on `baylee-cards` — so the copy is created
    /// naming the face it copied and is finished here, at the nearest place
    /// that holds a lookup.
    ///
    /// It runs first in the pass rather than beside the daybound checks
    /// because what follows in the same pass is what asks a permanent what
    /// it can do: [`Self::sync_static_effects`] on the very next line, then
    /// the trigger scan and the offer, all three reading
    /// [`GameObject::abilities`], and a copy still answering an empty list
    /// would have its text a whole priority window late.
    ///
    /// `own_abilities` wins where both are set: a copy of a copy was handed
    /// the list it is copying, and that list is the copiable one — the card
    /// underneath it is not what it is a copy of.
    fn settle_copied_rules_text(&mut self) {
        if self.state.pending_copied_faces.is_empty() {
            return;
        }
        for (id, card, face) in std::mem::take(&mut self.state.pending_copied_faces) {
            let printed = self
                .lookup
                .card(card)
                .map(|def| crate::object::AbilityList {
                    abilities: def.abilities_for_face(face as usize),
                    printed: crate::object::PrintedFace::new(card, face),
                });
            if let Some(obj) = self.state.object_mut(id)
                && obj.own_abilities.is_none()
                && let Some(printed) = printed
            {
                obj.take_abilities(printed);
            }
        }
    }

    /// Daybound and nightbound's continuous checks (CR 702.145c–g).
    ///
    /// They are not state-based actions — CR 702.145c and f say so in as
    /// many words — so they do not belong in [`sba::run`], and they need
    /// the card definition behind a permanent to know how many faces it
    /// has, which `sba::run` has no lookup for. They run as their own step
    /// of the machine's fixpoint, after the state-based actions have
    /// settled and before triggers are collected, so that a permanent that
    /// turns over does so before anything asks what triggered.
    ///
    /// Returns whether anything changed, which sends the fixpoint round
    /// again.
    ///
    /// The order inside is the order the rules fall in: the two that hand
    /// a game with *neither* designation one (d, then g) come before the
    /// two that read the designation (c and f). Otherwise a lone daybound
    /// creature entering a fresh game would wait a whole iteration for the
    /// day it is about to cause.
    fn day_night_statics(&mut self) -> bool {
        use baylee_cards_dsl::KeywordSet as K;
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield);
        // The common case by a wide margin: no daybound card at the table,
        // so the whole step is one scan of the battlefield and out.
        let mut any_daybound = false;
        let mut any_nightbound = false;
        for &id in battlefield {
            let Some(kw) = self.state.object(id).map(|o| o.characteristics().keywords) else {
                continue;
            };
            any_daybound |= kw.contains(K::DAYBOUND);
            any_nightbound |= kw.contains(K::NIGHTBOUND);
        }
        if !any_daybound && !any_nightbound {
            return false;
        }
        // CR 702.145d, then g. The nightbound clause is the conditional
        // one: it makes it night only when no daybound permanent is on the
        // battlefield at all, which is why both flags are collected before
        // either is acted on.
        if self.state.day_night.is_none() {
            if any_daybound {
                self.state.become_day();
                return true;
            }
            self.state.become_night();
            return true;
        }
        // CR 702.145c and f: front face up with daybound at night, or back
        // face up with nightbound by day, turns over. "Immediately", and by
        // its controller — but the transform is the whole of it, so there
        // is nobody to ask.
        let night = self.state.day_night == Some(DayNight::Night);
        let turning: Vec<ObjectId> = self
            .state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|&id| {
                let Some(obj) = self.state.object(id) else {
                    return false;
                };
                let kw = obj.characteristics().keywords;
                (night && obj.face_index == 0 && kw.contains(K::DAYBOUND))
                    || (!night && obj.face_index == 1 && kw.contains(K::NIGHTBOUND))
            })
            .collect();
        let mut changed = false;
        for id in turning {
            // CR 701.27c: only a permanent represented by a transforming
            // double-faced card can transform. A token has no card, and a
            // clone of a werewolf carries the copied keyword over a card
            // with one face — turning either one over would rebuild its
            // base from a face that is not there and wipe the copy. Both
            // are skipped *without* reporting a change, or the fixpoint
            // would find work to do forever.
            let Some(def) = self
                .state
                .object(id)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
            else {
                continue;
            };
            if def.faces.len() < 2 {
                continue;
            }
            changed |= self.state.transform(id, def, usize::from(night));
        }
        changed
    }

    pub(crate) fn finish_resolution(&mut self, res: &Resolution) {
        // The reflexive triggers this resolution created (CR 603.12) join
        // the queue as it ends, and from there take the ordinary path.
        // `queue_new_triggers` sorts them with that pass's other triggers
        // (CR 603.3b), and `collect_triggers` stacks them, asking for a
        // target or dropping one with none (CR 603.3d). It happens here,
        // the first thing every completed stack resolution passes, and
        // not in `queue_new_triggers`. Step 0b of `run_machine` can publish
        // a question (an as-enters choice) before that runs, and the list
        // must be empty whenever a question is out.
        self.trigger_queue.extend(self.state.reflexive.drain(..));
        // A mana ability never went on the stack (CR 605.3b), and its
        // `on_stack` is the source permanent itself. Falling through here
        // treated that permanent as a resolving spell: `finalize_spell`
        // untapped the land and "moved" it to the battlefield it was
        // already on, so every colour-choice source — Badlands, City of
        // Brass, Command Tower, Harabaz Druid — untapped itself and made
        // unbounded mana. Nothing to finalize; the activating player keeps
        // priority (CR 605.3a).
        if res.mana_ability {
            self.after_action(res.controller);
            return;
        }
        if self
            .state
            .object(res.on_stack)
            .is_some_and(|o| o.kind == ObjectKind::AbilityOnStack)
        {
            // Abilities on the stack simply cease to exist (CR 608.2n).
            //
            // CR 714.4's sacrifice used to be spelled out here, on the way
            // past: a resolving chapter carried its own Saga to the
            // graveyard. It is a state-based action and now runs as one, in
            // [`Self::finished_sagas`] — which is what a chapter ability
            // that *never resolves* needs. Countering the last chapter left
            // the Saga on the battlefield for the rest of the game.
            self.state.zones.remove(res.on_stack, ZoneLocation::Stack);
            let _ = self.state.arena.remove(res.on_stack);
        } else {
            self.finalize_spell(res.on_stack);
        }
        self.apply_pending_face_changes();
    }

    /// CR 707.10: a copy of a permanent spell stops being a copy of a spell
    /// and becomes a **token** permanent as it resolves — which is what
    /// Storm of Saruman's own reminder text says.
    ///
    /// Being a token is exactly "carries no card" here (`Filter::IsToken`),
    /// so the card has to go, and the rules text it would have answered with
    /// moves into `own_abilities` first: that is the slot a card-less object
    /// keeps its abilities in, and the one `move_object` leaves alone for an
    /// object with no card.
    ///
    /// The `SpellCopy` rider goes with it. It said "cease to exist off the
    /// stack" (CR 704.5e); from here CR 704.5d says the same thing about the
    /// token, and leaving both on would have the cleanup pass answer for one
    /// object twice.
    fn a_copy_becomes_a_token(&mut self, spell: ObjectId) {
        let is_copy = self
            .state
            .object(spell)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::SpellCopy));
        if !is_copy {
            return;
        }
        let printed = self
            .state
            .object(spell)
            .map(|o| o.ability_list(&self.lookup));
        if let Some(obj) = self.state.object_mut(spell) {
            if obj.own_abilities.is_none()
                && let Some(printed) = printed
            {
                obj.take_abilities(printed);
            }
            obj.card = None;
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::SpellCopy));
        }
    }

    // Adventure, the permanent door with the copy question in front of it,
    // rebound, flashback and the graveyard — five endings for one spell, and
    // each carries the rule that picks it.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn finalize_spell(&mut self, spell: ObjectId) {
        let (is_permanent, owner) = {
            let Some(obj) = self.state.object(spell) else {
                return;
            };
            // CR 608.2m lets a spell that leaves the stack part-way through
            // finish resolving; CR 608.2n then puts *the spell* into its
            // owner's graveyard — and a card its own effect has already
            // moved is not one. This asked neither question and moved
            // whatever id it was handed, so `Effect::ExileSource` exiled the
            // card and the finalisation fetched it straight back out:
            // Teferi's Protection, Temporal Mastery and Spirit Water Revival
            // all had the one clause the DSL can express undone one step
            // later. A permanent spell is unaffected — it is still on the
            // stack when this runs, which is the whole reason this line can
            // be a single zone test rather than a per-effect flag.
            if obj.zone != crate::zone::Zone::Stack {
                return;
            }
            (obj.characteristics().types.is_permanent(), obj.owner)
        };
        // Adventure (CR 715): an Adventure spell resolves to exile; the
        // front face may then be cast from exile.
        let adventure_def = {
            let face = self
                .state
                .object(spell)
                .map_or(0, |o| o.face_index as usize);
            self.state
                .object(spell)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .filter(|def| def.faces.get(face).is_some_and(|f| f.adventure))
        };
        if let Some(def) = adventure_def {
            // The card is exiled *on an adventure*, and an adventurer card
            // has its normal characteristics in every zone but the stack: it
            // is the creature that sits in exile, not the instant that just
            // resolved. The face was left where the cast put it, so the
            // exiled card kept Swift Spiral's name, its instant type and its
            // `{1}{W}`. Everything downstream reads the object: `can_cast`
            // probed `{1}{W}` and asked the timing of an *instant*, and the
            // wizard's back-face loop priced the same face, so the adventure
            // was castable out of its own exile for two white mana, at
            // instant speed, every turn, for as long as the card sat there.
            self.state.switch_face(spell, def, 0);
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Card;
                obj.riders.push(crate::object::Rider::Adventure);
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            return;
        }
        if is_permanent {
            self.a_copy_becomes_a_token(spell);
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Permanent;
            }
            self.state.set_tapped(spell, false);
            // CR 614.12a: a choice a replacement effect needs is made
            // *before* the permanent enters. Publishing a `Pending` returns
            // from here and the move is owed to the answer — see
            // `PlanKind::CopyOnEnter::before_entry`, which is where it is
            // paid.
            if self.ask_copy_before_entry(spell) {
                return;
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Spell,
            );
        } else {
            // Rebound (CR 702.88): cast from hand → exile with a rebound
            // rider and a delayed re-cast at the next upkeep.
            let rebound = self.state.object(spell).is_some_and(|o| {
                o.cast_from_hand
                    && o.characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::REBOUND)
            });
            if rebound {
                if let Some(obj) = self.state.object_mut(spell) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::Rebound);
                }
                let _ = self.state.move_object(
                    spell,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                self.state.delayed.push(crate::state::DelayedTrigger {
                    controller: owner,
                    when: crate::state::DelayedWhen::NextUpkeep,
                    action: crate::state::DelayedAction::CastFromExileWithoutPaying {
                        card: spell,
                        version: self.state.object(spell).map_or(0, |o| o.version),
                    },
                });
                return;
            }
            // Flashback (CR 702.34): exile instead of the graveyard.
            let flashback = self
                .state
                .object(spell)
                .is_some_and(|o| o.riders.contains(&crate::object::Rider::Flashback));
            if flashback {
                if let Some(obj) = self.state.object_mut(spell) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = self.state.move_object(
                    spell,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                return;
            }
            if let Some(obj) = self.state.object_mut(spell) {
                obj.kind = ObjectKind::Card;
            }
            let _ = self.state.move_object(
                spell,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Spell,
            );
        }
    }

    // ------------------------------------------------------------ combat

    pub(crate) fn begin_turn(&mut self, first_turn: bool) {
        // CR 502.2 asks the *previous* turn how many spells its active
        // player cast, and this turn's untap step is the first place that
        // can ask — by which time `per_turn.reset()` below has zeroed the
        // count and the swap under it has moved `turn.active` on. So the
        // pair is taken here, at the one instant both are still true.
        self.state.previous_turn = (!first_turn).then(|| {
            let active = self.state.turn.active;
            crate::state::PreviousTurn {
                active,
                spells_cast: self.state.per_turn.spells_cast[active.get() as usize],
            }
        });
        if !first_turn {
            // Extra turns (CR 500.7) preempt the normal successor.
            let next = self
                .state
                .extra_turns
                .pop_front()
                .unwrap_or_else(|| self.next_alive_after(self.state.turn.active));
            self.state.turn.active = next;
            self.state.turn.number += 1;
        }
        // Only the seat whose turn this is: summoning sickness is measured
        // against *their* most recent turn (CR 302.6), so an opponent's
        // creature stays asleep while this turn runs.
        let stamp = self.state.timestamp;
        let seat = self.state.turn.active.get() as usize;
        self.state.players[seat].turn_start_timestamp = stamp;
        // "Until your next turn" effects end as their controller's turn
        // begins (Elspeth's flying, Teferi's sorcery-flash).
        let new_active = self.state.turn.active;
        self.state.effects.remove_where(|fx| {
            matches!(fx.duration, baylee_cards_dsl::Duration::UntilYourNextTurn)
                && fx.controller == new_active
        });
        self.state.per_turn.reset();
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
            let fire = matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::NextUpkeep
            ) && self.state.delayed[i].controller == active;
            if fire {
                let trigger = self.state.delayed.remove(i);
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
    fn synthetic_source_name(&mut self, source: ObjectId) -> NameRef {
        if source == ObjectId::NO_SOURCE {
            return self.state.names.intern("Monarch");
        }
        self.state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name)
    }

    /// Pushes a synthetic trigger (prowess, ward, granted abilities) with
    /// explicitly chosen targets onto the stack.
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
            // Same as the sibling site: the chosen targets are one handle and
            // the event object is another.
            obj.event_object = t.event_object;
            // What the targets were chosen against. CR 608.2b re-checks
            // them against it at resolution, as it does a spell's.
            obj.target_req = t.synthetic_target.map(TargetReq::one);
            obj
        });
        self.synthetic_fx.insert(id, synthetic);
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
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
    fn is_saga(&self, id: ObjectId) -> bool {
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
                        && o.abilities(&self.lookup)
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
                    o.abilities(&self.lookup).get(index as usize),
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
    /// Returns whether anything was sacrificed, which sends the fixpoint
    /// round again — a Saga leaving the battlefield is exactly the kind of
    /// thing the state-based actions above want another look at.
    ///
    /// [`Object::abilities`]: crate::object::GameObject::abilities
    fn finished_sagas(&mut self) -> bool {
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield).clone();
        let mut changed = false;
        for id in battlefield {
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
            let Some(owner) = self.state.object(id).map(|o| o.owner) else {
                continue;
            };
            if let Some(obj) = self.state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = self.state.move_object(
                id,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            );
            changed = true;
        }
        changed
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
    fn queue_saga_chapters(
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
        for id in self.state.zones.list(ZoneLocation::Battlefield).clone() {
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
        let mut i = 0;
        while i < self.state.delayed.len() {
            if matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::NextEndStep
            ) {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue
                    .push_back((trigger.controller, trigger.action));
            } else {
                i += 1;
            }
        }
    }

    /// Processes one queued delayed action; returns `true` when a pending
    /// choice was produced.
    pub(crate) fn process_delayed(&mut self) -> bool {
        let Some((controller, action)) = self.delayed_queue.pop_front() else {
            return false;
        };
        // This is where a delayed trigger that has come due would be put on
        // the stack, and one controlled by a player who has left the game
        // isn't (CR 800.4d): Swift Spiral's creature stays in exile once
        // its caster has gone. The controller is the one who controlled the
        // spell or ability that made it (CR 603.7d, 603.7e).
        if self.state.has_left(controller) {
            return false;
        }
        match action {
            crate::state::DelayedAction::CastFromExileWithoutPaying { card, version } => {
                let Some(object) = self
                    .state
                    .object(card)
                    .filter(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
                else {
                    return false;
                };
                let owner = object.owner;
                let _ = self.start_free_cast(owner, card);
                self.awaiting_answer
            }
            crate::state::DelayedAction::ReturnToBattlefield { card } => {
                if self
                    .state
                    .object(card)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
                {
                    // End-step blink returns (Eerie Interlude, Swift
                    // Spiral): under the OWNER's control.
                    let owner = self.state.object(card).map(|o| o.owner);
                    if let Some(obj) = self.state.object_mut(card)
                        && let Some(owner) = owner
                    {
                        obj.set_controller(owner);
                    }
                    let _ = self.state.move_object(
                        card,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    );
                }
                false
            }
            // Mana Drain's "add": its controller's pool, whose first main
            // phase it waits for.
            crate::state::DelayedAction::AddMana { color, amount } => {
                self.state.players[controller.get() as usize]
                    .mana_pool
                    .add(color, amount);
                self.state.journal.record(GameEvent::ManaProduced {
                    player: controller,
                    color,
                    amount,
                    source: None,
                });
                false
            }
            crate::state::DelayedAction::PayCostOrSacrifice { cost, card } => {
                self.demand_echo(cost, card)
            }
            crate::state::DelayedAction::PayCostOrLose { cost } => self.demand_pact(cost),
        }
    }

    /// Echo come due (CR 702.30a): "sacrifice it unless you pay [cost]",
    /// asked of the active player once the upkeep's priority window has
    /// closed (see `upkeep_payments`). Returns `true` when a question was
    /// put.
    fn demand_echo(&mut self, cost: baylee_core::mana::ManaCost, card: ObjectId) -> bool {
        // It resolves after a priority window now, so the permanent
        // may already be gone — and only a permanent on the
        // battlefield can be sacrificed (CR 701.21a).
        if !self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
        {
            return false;
        }
        let active = self.state.turn.active;
        let can_pay =
            mana_pay::can_pay(&self.state.players[active.get() as usize].mana_pool, &cost);
        if !can_pay {
            // Echo with an empty pool: sacrifice immediately.
            let owner = self.state.object(card).map_or(active, |o| o.owner);
            if let Some(obj) = self.state.object_mut(card) {
                obj.kind = ObjectKind::Card;
            }
            let _ = self.state.move_object(
                card,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            );
            return false;
        }
        let source = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .map(|c| AbilityRef::new(c.index, AbilityRef::UPKEEP_COST));
        self.pending_plan = Some(PlanKind::DelayedPaySacrifice { cost, card });
        self.pending = Pending::YesNo {
            player: active,
            prompt: YesNoPrompt::Generic,
            source,
        };
        self.awaiting_answer = true;
        true
    }

    /// A pact's "pay [cost]; if you don't, you lose the game", demanded like
    /// echo and at the same moment. Returns `true` when a question was put.
    fn demand_pact(&mut self, cost: baylee_core::mana::ManaCost) -> bool {
        let active = self.state.turn.active;
        // Asked only now, after the upkeep's priority window (see
        // `upkeep_payments`): the player has had the chance to make
        // the mana, which this engine pays from the pool. A pool that
        // still cannot cover it is a payment that is not made, and
        // "if you don't, you lose the game" is all that is left.
        let can_pay =
            mana_pay::can_pay(&self.state.players[active.get() as usize].mana_pool, &cost);
        if !can_pay {
            let _ = sba::lose_by_effect(&mut self.state, active);
            return false;
        }
        self.pending_plan = Some(PlanKind::DelayedPay { cost });
        self.pending = Pending::YesNo {
            player: active,
            prompt: YesNoPrompt::Generic,
            // A pact's "pay or lose" must never be automatable:
            // a standing "no" here is a standing loss.
            source: None,
        };
        self.awaiting_answer = true;
        true
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
                // Nobody draws for an active player who has left (CR 800.4j).
                if !skip && !self.active_has_left() {
                    let active = self.state.turn.active;
                    self.state.draw_cards(active, 1);
                }
                (Phase::Beginning, Step::Draw)
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
                if self.state.combat.attackers.is_empty() {
                    (Phase::Combat, Step::CombatEnd)
                } else {
                    (Phase::Combat, Step::DeclareBlockers)
                }
            }
            (_, Step::DeclareBlockers) => {
                // Deal combat damage on entering the damage step(s).
                if self.any_first_or_double_striker() {
                    combat::deal_combat_damage(&mut self.state, true);
                    (Phase::Combat, Step::CombatDamageFirst)
                } else {
                    combat::deal_combat_damage(&mut self.state, false);
                    (Phase::Combat, Step::CombatDamage)
                }
            }
            (_, Step::CombatDamageFirst) => {
                combat::deal_combat_damage(&mut self.state, false);
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
            (_, Step::End) => (Phase::Ending, Step::Cleanup),
            (_, Step::Cleanup) => (Phase::Beginning, Step::Untap),
            _ => unreachable!("invalid phase/step combination"),
        };
        self.state.turn.phase = next_phase;
        self.state.turn.step = next_step;
        self.state.journal.record(GameEvent::StepChanged {
            phase: next_phase,
            step: next_step,
        });
    }

    pub(crate) fn any_first_or_double_striker(&self) -> bool {
        use baylee_cards_dsl::KeywordSet as K;
        self.state
            .combat
            .attackers
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
        self.state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                let Some(obj) = self.state.object(*id) else {
                    return false;
                };
                obj.controller == active
                    && obj.status.contains(Status::TAPPED)
                    && !self.keeps_tapped(*id)
                    && self.state.effects.iter().any(|fx| {
                        matches!(fx.modifier, baylee_cards_dsl::Modifier::MayChooseNotToUntap)
                            && crate::effects::applies_to(&self.state, fx, obj)
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
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield).clone();
        // Phasing: phased-out permanents the active player controls phase
        // back in at the untap step (CR 702.26a).
        for id in &battlefield {
            let phased = self
                .state
                .object(*id)
                .is_some_and(|o| o.controller == active && o.status.contains(Status::PHASED_OUT));
            if phased {
                if let Some(obj) = self.state.object_mut(*id) {
                    obj.status.remove(Status::PHASED_OUT);
                }
                self.state.journal.record(GameEvent::PhaseChanged {
                    object: *id,
                    phased_out: false,
                });
            }
        }
        self.check_day_night();
        // CR 502.3, the third turn-based action: "the active player
        // determines which permanents they control will untap. Then they
        // untap them all simultaneously."
        let optional = self.untap_optional();
        if optional.is_empty() {
            self.finish_untap_step(&[]);
            return false;
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
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield).clone();
        for id in battlefield {
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
        // in `end_of_combat` and `UntilEndOfTurn` in `cleanup_step` — and it
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

    pub(crate) fn cleanup_step(&mut self) -> bool {
        // Clear damage (CR 514.2), expire "until end of turn" effects
        // (CR 514.2), and check hand size.
        let mut reverted: Vec<ObjectId> = Vec::new();
        for obj in self.state.arena.iter_mut_all() {
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
        }
        self.state
            .effects
            .remove_where(|fx| matches!(fx.duration, baylee_cards_dsl::Duration::UntilEndOfTurn));
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
        let active = self.state.turn.active;
        // Reliquary Tower & co.: no maximum hand size for this player.
        let no_max = self.state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::NoMaxHandSize)
                && fx.controller == active
        });
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
        self.end_cleanup();
        false
    }

    pub(crate) fn end_cleanup(&mut self) {
        self.state.combat = crate::combat::CombatState::default();
        self.state.board_state_changed();
        self.combat_declared = CombatDeclared::None;
        self.begin_turn(false);
    }
}
