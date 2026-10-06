use super::{
    AbilityDef, AbilityLoc, CardLookup, Cause, Cleanup, CombatDeclared, EndReason, Engine,
    GameEvent, GameObject, GameResult, NameRef, ObjectId, ObjectKind, Pending, Phase, PlanKind,
    PlayerId, Resolution, SmallVec, Status, Step, Zone, ZoneLocation, ZonePosition, combat, eval,
    resolve, sba, trigger,
};
use crate::choice::{
    CastModeDesc, CastModeKind, ChoicePrompt, PlayerAction, PriorityHold, SeatAutomation,
    TargetPrompt, YesNoPrompt,
};
use crate::event::PolicyAnswer;
use crate::state::Side;
use crate::turn::DayNight;
use crate::win::Victor;
use baylee_cards_dsl::{Effect, Filter, PlayerRel, SpellMode, TargetReq, TargetSpec};
use baylee_core::ids::{AbilityRef, SeatSet};
use baylee_core::preset::LoopPolicy;

mod delayed;
mod entering;
mod stack;
mod statics;
mod steps;
mod triggers;
mod turn;

/// What a permanent that has just entered still has to ask as it enters,
/// queued in [`Engine::entry_questions`] until every co-arrival's other
/// replacements are on the board.
#[derive(Clone, Copy, Debug)]
pub(crate) enum EntryAsk {
    /// Which permanent it enters as a copy of (a clone put onto the
    /// battlefield by another card's effect; `check_copy_on_enter`).
    Copy,
    /// The first of its own modifiers that asks.
    Modifier(&'static baylee_cards_dsl::EnterModifier),
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
        // What the seat's per-ability policy answered, when it was the
        // policy that answered (#234): its ability, the stack object asked
        // about, and the answer.
        type ByPolicy = Option<(AbilityRef, Option<ObjectId>, PolicyAnswer)>;
        if self.decision_actor() != self.pending.asked() {
            return false;
        }
        let answer: Option<(PlayerId, PlayerAction, ByPolicy)> = match &self.pending {
            Pending::Priority { player, legal } => {
                let settings = self.automation(*player);
                let top = self.state.zones.list(ZoneLocation::Stack).last().copied();
                let top_ability = top.and_then(|id| self.state.object(id)).and_then(|obj| {
                    let loc = obj.ability?;
                    self.state
                        .printed_ability_list(obj.id)
                        .and_then(|list| list.entry(loc.index as usize))
                        .and_then(|entry| entry.provenance.ability_ref())
                        .or_else(|| loc.card.map(|card| AbilityRef::new(card, loc.index)))
                });
                let by_hold = match settings.hold {
                    PriorityHold::PassWhenNothingToDo => legal.nothing_but_passing(),
                    // Every other variant either withholds the decision or
                    // does not, and `suppresses` is the one place that says
                    // which — the same answer the view hands the client, so
                    // an indicator cannot disagree with the engine. The
                    // expiry pass above already cleared any hold whose
                    // condition is met, so an active one still means "keep
                    // going".
                    other => other.suppresses(),
                };
                let by_policy = top_ability.filter(|ability| settings.yields_to(*ability));
                let pass = !settings.priority_paused && (by_hold || by_policy.is_some());
                // The policy is reported only where it made the difference:
                // a pass the hold would have made anyway is the hold's, and
                // the seat already sees its hold.
                let report = by_policy
                    .filter(|_| !by_hold)
                    .map(|ability| (ability, top, PolicyAnswer::Passed));
                pass.then_some((*player, PlayerAction::PassPriority, report))
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
            } if prompt.automatable() => {
                let asking = self.resolution.as_ref().map(|r| r.on_stack);
                self.automation(*player)
                    .standing_answer(*ability)
                    .map(|standing| {
                        let told = if standing.as_bool() {
                            PolicyAnswer::Yes
                        } else {
                            PolicyAnswer::No
                        };
                        (
                            *player,
                            PlayerAction::YesNo(standing.as_bool()),
                            Some((*ability, asking, told)),
                        )
                    })
            }
            _ => None,
        };
        let Some((player, action, report)) = answer else {
            return false;
        };
        self.awaiting_answer = false;
        // An automated answer goes through the ordinary action path, so it
        // is validated and journaled exactly like a hand-played one — a
        // replay cannot tell the difference, which is the point.
        let accepted = self.apply_inner(player, action).is_ok();
        // Recorded once accepted, so the journal never claims an answer the
        // game refused.
        if accepted && let Some((ability, object, answer)) = report {
            self.state.journal.record(GameEvent::AutoAnswered {
                player,
                ability,
                object,
                answer,
            });
        }
        accepted
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
            if self.state.numeric_failure.is_some() {
                return;
            }
            if self.open_land_mana_window() {
                continue;
            }
            self.open_variable_mana_window();
            self.state.capture_rule_references(&self.lookup);
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

    fn advance_combat_damage(&mut self) -> bool {
        let Some(mut work) = self.combat_damage.take() else {
            return false;
        };
        if let Some(pending) = work.advance(&mut self.state) {
            self.combat_damage = Some(work);
            self.pending = pending;
            self.awaiting_answer = true;
            return true;
        }
        self.finish_combat_damage(&work);
        false
    }

    /// Payment opportunities update layers and mana triggers without priority.
    /// Ordinary triggers and SBAs wait (CR 117.2e, 117.5, 605.4a).
    fn advance_payment_window(&mut self) -> bool {
        self.queue_new_triggers();
        if self.resolve_triggered_mana_abilities() {
            return true;
        }
        if self.state.journal.last_seq() != self.trigger_scan_seq {
            return false;
        }
        let Some(window) = &self.mana_window else {
            return false;
        };
        if matches!(window.suspended, super::PaymentContinuation::LandMana(_)) {
            return self.advance_land_mana();
        }
        let player = window.player;
        self.regrant_priority = None;
        self.pending = Pending::Priority {
            player,
            legal: Box::new(self.compute_legal(player)),
        };
        self.awaiting_answer = true;
        true
    }

    #[allow(clippy::too_many_lines)] // Ordered rules machine: payment, SBA, triggers and priority must remain visible together.
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
        while !self.advance_combat_damage() {
            if self.state.numeric_failure.is_some() {
                return;
            }
            if let Some(pending) = crate::graveyard_order::pending(&mut self.state) {
                self.pending = pending;
                self.awaiting_answer = true;
                return;
            }
            let signature = watch.wants_sample().then(|| self.state.loop_signature());
            if let Some(period) = watch.step(signature)
                && self.on_loop_detected(period)
            {
                return;
            }
            self.prune_player_control();
            // 0. A token copy's rules text (CR 707.2), before anything asks
            //    what a permanent can do.
            self.settle_copied_rules_text();
            // 0a. Continuous effects: sync statics with the battlefield and
            //    refresh characteristic caches (generation compare).
            // `sync_static_effects` refreshes the projection it reads, so
            // whether the effect set moved is asked before it as well.
            let stale = self.state.characteristics_generation != self.state.effects.generation;
            self.sync_static_effects();
            let moved =
                stale || self.state.characteristics_generation != self.state.effects.generation;
            self.state.refresh_characteristics();
            if self.state.award_enduring_stories() {
                continue;
            }
            // CR 702.131d: continuous effects are reapplied after a player
            // gets the city's blessing, before anything else is asked.
            if self.state.award_citys_blessings() {
                continue;
            }
            // What a player who has left still controls is exiled as the
            // last effect giving it to somebody else ends (CR 800.4c). Not a
            // state-based action, so before them and before the game-over
            // check: the first point after any effect can have ended. Only
            // an effect ending can hand an object back to a departed player,
            // and every one of those moves the generation.
            if moved
                && self
                    .state
                    .players
                    .iter()
                    .any(crate::state::Player::has_lost)
                && !crate::sba::exile_what_the_departed_control(&mut self.state).is_empty()
            {
                continue;
            }
            // 0b. As-it-enters modifiers (taplands, shockland choices).
            let wrote = self.apply_enter_modifiers();
            if self.awaiting_answer {
                // The question is published from here, and the step that
                // asks it may already have moved the board behind 0a: a
                // daybound permanent entering at night entered transformed
                // (CR 702.145b), and `GameState::turn_over` took away the
                // statics of the face going down and left the new face's
                // to the next scan; a Room took its door, and an
                // earlier arrival its counters. So the board is settled
                // first, as `Engine::new` and `settle_mulligans` settle the
                // one the opening hands are kept beside. Nothing else the
                // pass owes runs before the answer, and that is right: the
                // rules make an as-it-enters choice before the permanent
                // enters (CR 614.12a), so no state-based action or trigger
                // may look at the board first.
                self.sync_static_effects();
                self.state.refresh_characteristics();
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
            // 0c. An Aura's return finishes once the card it returned has
            //     entered: its as-it-enters choices made (a Clone's copy) and
            //     its statics registered at 0a, so the attachment asks the
            //     creature it actually became, and before any state-based
            //     action or trigger looks at the board.
            if !self.state.reanimation_finishes.is_empty() {
                for finish in std::mem::take(&mut self.state.reanimation_finishes) {
                    crate::aura_bindings::finish_reanimation(&mut self.state, finish);
                }
                continue;
            }
            if self.mana_window.is_some() {
                if self.advance_payment_window() {
                    return;
                }
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
            let outcome = self.run_state_based_actions();
            if outcome.changed
                || outcome.legend_choice.is_some()
                || outcome.commander_zone.is_some()
            {
                self.cleanup_check_acted();
            }
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
            // A miracle whose spell could not choose its targets could only
            // be declined, so it is not offered (CR 601.2c, 601.2).
            if !self.miracle_targets_available(player, card) {
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
            Step::Cleanup => match self.cleanup {
                Cleanup::Due => self.cleanup_step(),
                // The check put nothing on the stack and performed nothing
                // (`cleanup_check_acted`), so no player gets priority and
                // the step ends (CR 704.3, last sentence).
                Cleanup::Checking if self.state.zones.stack_is_empty() => self.end_cleanup(),
                // A turn was skipped (CR 614.10): the one after it would
                // begin now, and is offered to its own skips first.
                Cleanup::Ended { after } => self.begin_next_turn(after),
                // A triggered ability is on the stack, or a state-based
                // action was performed: the active player gets priority, and
                // the step is a priority step from here (CR 514.3a).
                Cleanup::Checking | Cleanup::Open => {
                    self.cleanup = Cleanup::Open;
                    self.priority_round()
                }
            },
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
                self.pending = self.attack_question(attacker);
                self.awaiting_answer = true;
                true
            }
            Step::DeclareBlockers if self.combat_declared != CombatDeclared::Blockers => {
                let after = match self.combat_declared {
                    CombatDeclared::BlockersBy(seat) => Some(seat),
                    _ => None,
                };
                let defending = match (self.next_defending_player(after), after) {
                    (Some(seat), _) => seat,
                    // With no attackers there is nobody to ask, so the seat
                    // order stands and the declaration is an empty one.
                    (None, None) => self.next_alive_after(self.state.turn.active),
                    // `declare_blockers` asks the next one itself, so this
                    // is a table whose last defending player left between.
                    (None, Some(_)) => {
                        self.combat_declared = CombatDeclared::Blockers;
                        return false;
                    }
                };
                self.ask_blockers(defending);
                true
            }
            _ => self.priority_round(),
        }
    }

    /// The next defending player to declare blockers after `after` (from
    /// the first when `None`), or `None` when every one has.
    ///
    /// "If more than one player is being attacked, controls a planeswalker
    /// that's being attacked, or protects a battle that's being attacked,
    /// each defending player in APNAP order declares blockers as the
    /// declare blockers step begins. … The first defending player declares
    /// all their blocks, then the second defending player, and so on"
    /// (CR 802.4); APNAP order is the active player, then "the remaining
    /// nonactive players in turn order" (CR 101.4). A player nothing
    /// attacks has no creature it could block with (802.4a) and is not
    /// asked.
    ///
    /// Where `after` sits is read off the seats, not off the players still
    /// in the game: a defending player who declared and then left the game
    /// (CR 800.4a) still stands between those who declared before them and
    /// those who have yet to, and is not a sign that everyone has.
    pub(crate) fn next_defending_player(&self, after: Option<PlayerId>) -> Option<PlayerId> {
        let n = u8::try_from(self.state.players.len()).unwrap_or(u8::MAX);
        let active = self.state.turn.active.get();
        let attacked = |seat: PlayerId| {
            self.state
                .combat
                .attackers()
                .iter()
                .any(|a| combat::blocking_player(&self.state, a.defending) == Some(seat))
        };
        let from = after.map_or(1, |seat| (seat.get() + n - active) % n + 1);
        (from..n)
            .map(|offset| PlayerId::new((active + offset) % n))
            .filter(|seat| !self.state.players[usize::from(seat.get())].has_lost())
            .find(|seat| attacked(*seat))
    }

    /// Asks `defending` for its blocks (CR 509.1): each creature it may
    /// block with and what it may block, how many attackers each may block
    /// (509.1a), one declaration obeying the requirements (509.1c) and the
    /// counts the declaration as a whole is held to (509.1b).
    pub(crate) fn ask_blockers(&mut self, defending: PlayerId) {
        let active = self.state.turn.active;
        let blockers = combat::block_options(&self.state, defending);
        let rules = combat::BlockRules::new(&self.state);
        let capacity = blockers
            .iter()
            .filter_map(|o| {
                let most = rules.capacity(o.blocker);
                (most != Some(1)).then(|| crate::choice::BlockCapacity {
                    blocker: o.blocker,
                    most: most.map(|n| u8::try_from(n).unwrap_or(u8::MAX)),
                })
            })
            .collect();
        let obeying = rules.obeying(&blockers);
        // What `obeying` and an answer are counted by (CR 509.1c), so a
        // client can hold its declaration to it before sending it.
        let demands = if rules.has_requirements() {
            blockers
                .iter()
                .flat_map(|o| o.attackers.iter().map(move |a| (o.blocker, *a)))
                .filter_map(|(blocker, attacker)| {
                    let count = rules.demands(blocker, attacker);
                    (count > 0).then(|| crate::choice::BlockDemand {
                        blocker,
                        attacker,
                        count: u32::try_from(count).unwrap_or(u32::MAX),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        // The counts the declaration as a whole is held to (CR 509.1b),
        // for the attackers somebody may block.
        let bounds = self
            .state
            .combat
            .attackers()
            .iter()
            .map(|a| a.creature)
            .filter(|a| blockers.iter().any(|o| o.attackers.contains(a)))
            .filter_map(|a| combat::block_bound(&self.state, a))
            .collect();
        self.pending = Pending::ChooseBlockers {
            player: defending,
            attacker: active,
            blockers,
            capacity,
            obeying,
            demands,
            bounds,
        };
        self.awaiting_answer = true;
    }

    /// The declare-attackers question for `attacker` (CR 508.1): what may
    /// attack and what it may attack, what must attack (CR 508.1d), and
    /// which creatures the restrictions on the pair hold to part of the
    /// defenders (CR 508.1c).
    fn attack_question(&self, attacker: PlayerId) -> Pending {
        let defenders = combat::defender_options(&self.state, attacker);
        let rules = combat::AttackRules::new(&self.state);
        let mut attackers = Vec::new();
        let mut required = Vec::new();
        let mut limits = Vec::new();
        for id in self.state.battlefield_seen() {
            if !combat::can_attack(&self.state, attacker, id) {
                continue;
            }
            // A creature every restriction on the pair shuts out
            // attacks nothing, and a requirement asks nothing of
            // it (CR 508.1d counts only what can be obeyed).
            let allowed = rules.defenders_for(id, &defenders);
            if allowed.is_empty() {
                continue;
            }
            if allowed.len() < defenders.len() {
                limits.push(crate::choice::AttackLimit {
                    creature: id,
                    defenders: allowed,
                });
            }
            if rules.must_attack(id) {
                required.push(id);
            }
            attackers.push(id);
        }
        Pending::ChooseAttackers {
            player: attacker,
            attackers,
            defenders,
            required,
            limits,
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
                // (CR 702.94a). The ability triggered by revealing the card
                // allows its cast when it resolves, after a priority window.
                // The cast can now open its own mana-only payment window;
                // floating mana before this offer is no longer required.
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
                // A division of combat damage banding hands to a player is
                // asked before the damage step deals it (CR 702.22j–k);
                // its last answer ends this step (`answer_share`).
                if self.ask_combat_division() {
                    return true;
                }
                self.advance_step();
                // The draw step's draw may have been offered to a skip
                // (`offer_draw_skip`): answered before anything else.
                if self.awaiting_answer {
                    return true;
                }
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
}

/// Ends every "for as long as you control [the source]" whose controller no
/// longer controls its source (CR 611.2b). The source leaving is handled as
/// it moves (`GameState::move_object`); this is the other half, a change of
/// control, which moves nothing.
fn end_control_durations(state: &mut crate::state::GameState) {
    use baylee_cards_dsl::Duration;
    let lost: Vec<(ObjectId, PlayerId)> = state
        .effects
        .iter()
        .filter(|fx| matches!(fx.duration, Duration::WhileYouControlSource))
        .filter_map(|fx| {
            let source = fx.source?;
            // A phased-out source is treated as though it does not exist
            // (CR 702.26b): nobody controls it, and a "for as long as" that
            // tracks it ends as it phases out (CR 702.26f).
            let held = state.object(source).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == fx.controller
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
            });
            (!held).then_some((source, fx.controller))
        })
        .collect();
    if !lost.is_empty() {
        state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::WhileYouControlSource)
                && fx
                    .source
                    .is_some_and(|s| lost.contains(&(s, fx.controller)))
        });
    }
}

/// Drops every effect for as long as the game lasts that names one object
/// which has since moved: it is a new object the effect never named
/// (CR 400.7), and a control change would otherwise sit in the hashed table
/// for the rest of the game.
/// The static abilities of emblems not yet registered. "Abilities of emblems
/// function in the command zone" (CR 114.4), and an emblem never leaves it,
/// so each is registered once and lasts the game (Wrenn and Realmbreaker's
/// "You may play lands and cast permanent spells from your graveyard").
fn emblem_statics(state: &crate::state::GameState) -> Vec<crate::effects::ContinuousEffect> {
    let mut found = Vec::new();
    for seat in 0..state.players.len() {
        let zone = ZoneLocation::Command(PlayerId::new(seat as u8));
        for &id in state.zones.list(zone) {
            let Some(obj) = state.object(id) else {
                continue;
            };
            if obj.kind != crate::object::ObjectKind::Emblem {
                continue;
            }
            let Some(abilities) = obj.own_abilities.as_ref() else {
                continue;
            };
            for ability in crate::copiable_abilities::AbilityDefs::from(abilities) {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                if state.effects.has_source_ability(id, sa.modifier) {
                    continue;
                }
                found.push(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller: obj.controller,
                    origin: crate::effects::EffectOrigin::Static,
                    layer: sa.layer,
                    timestamp: obj.timestamp,
                    duration: baylee_cards_dsl::Duration::Indefinitely,
                    filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                    modifier: sa.modifier,
                });
            }
        }
    }
    found
}

fn forget_effects_on_moved_objects(state: &mut crate::state::GameState) {
    use baylee_cards_dsl::Duration;
    let stale: Vec<(ObjectId, u32)> = state
        .effects
        .iter()
        .filter(|fx| matches!(fx.duration, Duration::Indefinitely))
        .filter_map(|fx| match fx.filter {
            crate::effects::EffectFilter::ObjectIs(id, version) => state
                .object(id)
                .is_none_or(|o| o.version != version)
                .then_some((id, version)),
            crate::effects::EffectFilter::Dsl(_) => None,
        })
        .collect();
    if !stale.is_empty() {
        state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::Indefinitely)
                && matches!(fx.filter, crate::effects::EffectFilter::ObjectIs(id, version)
                    if stale.contains(&(id, version)))
        });
    }
}

/// Whether a static ability's effect in `layer` goes on applying after the
/// ability is gone (CR 613.6): the characteristic-changing layers before 6.
/// Layer 3 is where rules effects are parked (`Modifier::layer`), and those
/// are the ability itself, so they go with it.
fn outlives_its_ability(layer: baylee_cards_dsl::Layer) -> bool {
    use baylee_cards_dsl::Layer;
    matches!(
        layer,
        Layer::Copy | Layer::Control | Layer::Type | Layer::Color
    )
}

/// Whether a land's own static ability keeps its effect in `layer` once an
/// effect has set the land's subtype to a basic land type (CR 305.7).
///
/// That effect applies in layer 4, so only the land's effects in the layers
/// before it were applied first (CR 613.6). One in layer 4 itself goes: the
/// land-type effect decides whether it exists, so it depends on that effect
/// and waits for it (CR 613.8a) — Urborg, Tomb of Yawgmoth set to a Swamp
/// makes no other land a Swamp.
fn outlives_its_rules_text(layer: baylee_cards_dsl::Layer) -> bool {
    use baylee_cards_dsl::Layer;
    matches!(layer, Layer::Copy | Layer::Control)
}

/// What a synthetic trigger's targets were chosen against, for the
/// resolution-time re-check (CR 608.2b): one object, or up to one per
/// opponent for `ObjectOfEachOpponent` — the count the questions asked, and
/// written as the printed cards write it (`TargetReq::up_to(spec,
/// u8::MAX)`), so a second opponent's answer is not trimmed as a surplus.
fn synthetic_target_req(spec: TargetSpec) -> TargetReq {
    match spec {
        TargetSpec::ObjectOfEachOpponent(_) => TargetReq::up_to(spec, u8::MAX),
        _ => TargetReq::one(spec),
    }
}

/// Dash's delayed triggered ability (CR 702.109a): "return the permanent this
/// spell becomes to its owner's hand at the beginning of the next end step".
/// It asks as it resolves whether its source is still that permanent
/// (`Condition::DashCostPaid`), because a permanent that has left the
/// battlefield since is a new object (CR 400.7) and a card in a graveyard is
/// not returned by it.
static DASH_RETURN: [Effect; 1] = [Effect::IfCondition {
    condition: baylee_cards_dsl::Condition::DashCostPaid,
    then: &[Effect::ReturnToHand {
        target: TargetSpec::EventObject,
    }],
    otherwise: &[],
}];
