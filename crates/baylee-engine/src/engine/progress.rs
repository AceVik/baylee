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

/// One `Modifier::UntapAtMost` binding the active player's untap step.
struct UntapLimit {
    /// What it counts.
    of: &'static Filter,
    /// The effect's controller, the filter's "you".
    you: PlayerId,
    /// The effect's source, the filter's "this".
    this: Option<ObjectId>,
    /// How many of them may untap.
    count: u8,
}

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

    /// Applies as-it-enters-the-battlefield modifiers to permanents that
    /// entered since the last scan (CR 614.1c/d; taplands, shocklands).
    ///
    /// Permanents that enter together are one event, and each one's own
    /// replacements modify how it enters (CR 614.12). So every arrival's
    /// replacements that need nobody's answer are applied first: entering
    /// tapped, counters, starting loyalty, a Saga's lore counter, daybound's
    /// back face, a Room's door. The ones that ask are then asked one at a
    /// time, from [`Engine::entry_questions`], by seat in APNAP order and in
    /// the order the permanents entered (CR 101.4). A question used to return
    /// from the middle of the scan with the cursor already past every
    /// arrival, so every co-arrival after the one that asked got none of its
    /// replacements: a Urza's Saga fetched beside Steam Vents had no lore
    /// counter, and a planeswalker entering behind a shockland had no
    /// loyalty and died.
    ///
    /// Returns whether a modifier wrote to the board, which the caller reads
    /// twice over: the legal lists are recomputed from it, and the machine
    /// goes round one more pass. The second is the load-bearing one — this
    /// runs *after* the projection two steps up, so a counter placed here is
    /// invisible to the state-based actions two steps down until they have
    /// been refreshed once more.
    pub(crate) fn apply_enter_modifiers(&mut self) -> bool {
        let mut changed = false;
        // A question an earlier scan owes comes first. Nothing enters while
        // one is out, so the queue holds the rest of that batch's questions,
        // and the arrivals below wait for it to empty.
        if self.ask_owed_entry_question(&mut changed) {
            return true;
        }
        changed |= self.scan_arrivals();
        if self.ask_owed_entry_question(&mut changed) {
            return true;
        }
        changed
    }

    /// The half of [`Self::apply_enter_modifiers`] that asks nobody: every
    /// arrival since the last scan gets the replacements that need no answer,
    /// and the ones that ask are queued on [`Engine::entry_questions`],
    /// APNAP, for the caller to ask. Returns whether it wrote to the board.
    ///
    /// [`Engine::new`] runs it over the starting battlefield, where nobody
    /// may be asked anything yet: every seat is deciding its mulligan.
    #[allow(clippy::too_many_lines)] // the entry-modifier table is naturally flat
    pub(crate) fn scan_arrivals(&mut self) -> bool {
        use baylee_cards_dsl::EnterModifier;
        let mut changed = false;
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
        let mut asks: Vec<(ObjectId, PlayerId, EntryAsk)> = Vec::new();
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
            //
            // It enters with its back face up (CR 712.14a); nothing turns
            // over, so it is `turn_over` and not `transform`: no
            // `Transformed` entry for a "transforms into" trigger or the
            // log to read, and no new timestamp over the one it entered
            // with (CR 613.7d). `transform` gave it both.
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
                self.state.turn_over(id, def, 1);
                changed = true;
            }
            // A Room is given the unlocked designation of the half it was
            // cast as as it enters, and neither when it was not cast
            // (CR 709.5d). Here, like "enters transformed", so what the
            // trigger scan later in this pass finds is the half that entered.
            // The unlock is journalled because CR 709.5h triggers on the
            // designation however it was given.
            if let Some((def, half)) = self
                .state
                .object(id)
                .filter(|o| o.zone == Zone::Battlefield && !o.doors.is_room())
                .and_then(|o| Some((self.lookup.card(o.card?.index)?, o.face_index)))
                .filter(|(def, _)| def.has_shared_type_line())
            {
                let cast = from_zone == Zone::Stack && half < 2;
                self.state
                    .set_doors(id, def, if cast { 1 << half } else { 0 });
                if cast {
                    self.state
                        .journal
                        .record(GameEvent::DoorUnlocked { object: id, half });
                }
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
                    action: crate::state::DelayedAction::PayCostOrSacrifice {
                        cost,
                        card: id,
                        version: self.state.object(id).map_or(0, |o| o.version),
                    },
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
            if self.put_starting_loyalty(id) {
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
            //
            // Asked after the scan like every other question, and it still
            // stands in for this permanent's own modifiers, as it always has.
            if from_zone != Zone::Stack
                && self
                    .copy_on_enter_question(id)
                    .is_some_and(|(options, _)| options.iter().any(|&o| o != id))
            {
                asks.push((id, controller, EntryAsk::Copy));
                continue;
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
            // card prints it in: it is queued, and asked once every
            // arrival's other modifiers are on the board. Uncharted Haven is
            // `ChooseColor` then `Tapped` and once entered untapped, when a
            // question returned from the middle of this loop; the same hole
            // had been under `ChooseSubtype` and `TappedOrPayLife` since
            // they were written.
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
                    EnterModifier::LoseLifeEqualToLife => {
                        let life = self
                            .state
                            .players
                            .get(controller.get() as usize)
                            .map_or(0, |p| p.life);
                        if life > 0 {
                            self.state
                                .change_life(controller, -life, crate::event::Cause::Effect);
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
                    | EnterModifier::ChooseBasicLandType
                    | EnterModifier::ChooseCardName
                    | EnterModifier::ChooseColor
                    | EnterModifier::ChooseOpponent
                    | EnterModifier::ChooseColorExcept(_)
                    | EnterModifier::TappedOrPayLife(_)
                    | EnterModifier::TappedUnlessReveal(_) => {
                        asked.get_or_insert(modifier);
                    }
                }
            }
            if let Some(modifier) = asked {
                asks.push((id, controller, EntryAsk::Modifier(modifier)));
            }
        }
        // CR 101.4: players choosing at the same time choose in APNAP
        // order. One player's questions keep the order the permanents
        // entered in (the sort is stable); CR 101.4c would let that player
        // choose the order, which is not offered.
        let active = self.state.turn.active.get();
        let seats = self.state.players.len() as u8;
        asks.sort_by_key(|(_, controller, _)| (controller.get() + seats - active) % seats);
        self.entry_questions.extend(asks);
        changed
    }

    /// Asks the next as-it-enters question [`Engine::entry_questions`]
    /// holds, and says whether one is out. One whose permanent has left the
    /// battlefield in the meantime is dropped. One that has no question to
    /// ask any more is settled without one: a shockland its controller can
    /// no longer pay for, or a reveal with nothing to reveal, enters tapped,
    /// and `changed` says the board was written.
    fn ask_owed_entry_question(&mut self, changed: &mut bool) -> bool {
        while let Some((id, controller, ask)) = self.entry_questions.pop_front() {
            if self
                .state
                .object(id)
                .is_none_or(|o| o.zone != Zone::Battlefield)
            {
                continue;
            }
            let asked = match ask {
                EntryAsk::Copy => self.check_copy_on_enter(id),
                EntryAsk::Modifier(modifier) => {
                    self.ask_entry_modifier(id, controller, modifier, changed)
                }
            };
            if asked {
                return true;
            }
        }
        false
    }

    /// The entry choice uses opponent relationships, never targeting restrictions.
    fn ask_enter_opponent(&mut self, id: ObjectId, controller: PlayerId) -> bool {
        let options =
            eval::players(PlayerRel::Opponent, &self.state, controller).unwrap_or_default();
        if options.is_empty() {
            return false;
        }
        self.pending_plan = Some(PlanKind::ChooseOpponent { object: id });
        self.pending = Pending::ChoosePlayer {
            player: controller,
            options,
        };
        self.awaiting_answer = true;
        true
    }

    /// Asks one permanent's as-it-enters question, or settles it without
    /// one when there is nothing to choose; says whether a question is out.
    fn ask_entry_modifier(
        &mut self,
        id: ObjectId,
        controller: PlayerId,
        modifier: &'static baylee_cards_dsl::EnterModifier,
        changed: &mut bool,
    ) -> bool {
        use baylee_cards_dsl::EnterModifier;
        match modifier {
            EnterModifier::ChooseOpponent => self.ask_enter_opponent(id, controller),
            EnterModifier::ChooseSubtype | EnterModifier::ChooseBasicLandType => {
                use baylee_core::generated::subtypes::land::{
                    FOREST, ISLAND, MOUNTAIN, PLAINS, SWAMP,
                };
                // A basic land type is one of the five (CR 205.3i), in the
                // order the rule names them, and nothing else.
                let options = if matches!(modifier, EnterModifier::ChooseBasicLandType) {
                    vec![PLAINS, ISLAND, SWAMP, MOUNTAIN, FOREST]
                } else {
                    (0..=349).map(baylee_core::ids::SubtypeId::new).collect()
                };
                self.pending_plan = Some(PlanKind::ChooseSubtype { object: id });
                self.pending = Pending::ChooseSubtype {
                    player: controller,
                    options,
                };
                self.awaiting_answer = true;
                true
            }
            EnterModifier::ChooseCardName => {
                self.pending_plan = Some(PlanKind::ChooseCardName { object: id });
                self.pending = Pending::ChooseCardName { player: controller };
                self.awaiting_answer = true;
                true
            }
            m @ (EnterModifier::ChooseColor | EnterModifier::ChooseColorExcept(_)) => {
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
                true
            }
            EnterModifier::TappedOrPayLife(amount) => {
                let amount = *amount;
                // Asked when its turn comes, so a second shockland in the
                // same batch is asked against the life the first one left
                // (CR 614.12b).
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
                    return true;
                }
                // Unpayable → tapped without a choice: nothing was asked.
                self.state.set_tapped(id, true);
                *changed = true;
                false
            }
            EnterModifier::TappedUnlessReveal(filter) => {
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
                        self.state
                            .object(**card)
                            .is_some_and(|o| eval::matches(filter, &self.state, o, controller, id))
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
                    *changed = true;
                    return false;
                }
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
                    total: None,
                };
                self.awaiting_answer = true;
                true
            }
            _ => unreachable!("only the asking modifiers are queued"),
        }
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

    /// CR 306.5b: a planeswalker enters with as many loyalty counters as its
    /// printed loyalty. Returns whether it put any.
    ///
    /// "Printed" is read off the entering permanent's copiable values and not
    /// off its card. CR 614.12 decides which replacement effects apply to a
    /// permanent entering from the permanent as it would exist on the
    /// battlefield, counting replacement effects that already modified how it
    /// enters, and a copy is one (CR 614.1c "enters as"). Loyalty is a
    /// copiable value (CR 707.2), counters are not. So a Spark Double that
    /// enters as a copy of Karn, the Great Creator is a planeswalker with
    /// Karn's printed 5, whatever Karn has now, and takes 5 here beside the
    /// one its own text adds. Read off the card, it was a 0/0 creature with no
    /// loyalty and entered with that one alone.
    ///
    /// Starting loyalty is counters put on the permanent as it enters, so the
    /// counter-placement replacements apply (CR 614.16): Doubling Season
    /// doubles it.
    pub(crate) fn put_starting_loyalty(&mut self, id: ObjectId) -> bool {
        let Some(loyalty) = crate::layers::copiable_values(&self.state, id).and_then(|values| {
            values
                .types
                .contains(baylee_core::types::TypeSet::PLANESWALKER)
                .then_some(values.loyalty)
                .flatten()
        }) else {
            return false;
        };
        crate::replacement::put_counters(
            &mut self.state,
            id,
            baylee_cards_dsl::CounterKind::Loyalty,
            loyalty,
        );
        true
    }

    /// Apply an entry copy through the same snapshot transaction as a
    /// resolving copy ability (CR 707.2, 707.9).
    pub(crate) fn apply_copy_choice(&mut self, id: ObjectId, target: ObjectId) {
        #[cfg(test)]
        crate::ability_log::copied_on_entry(&self.state, &self.lookup, id);
        let Some(own) = self.state.printed_ability_list(id) else {
            return;
        };
        let Some((index, mods, until_eot)) =
            own.abilities
                .iter()
                .enumerate()
                .find_map(|(index, ability)| match ability {
                    AbilityDef::CopyOnEnter { mods, .. } => Some((index, *mods, false)),
                    AbilityDef::CopyOnEnterUntilEot { mods, .. } => Some((index, *mods, true)),
                    _ => None,
                })
        else {
            return;
        };
        let text = own
            .base_text(index)
            .then(crate::eval::live_context(&self.state, id).text);
        crate::copiable_abilities::apply_copy(
            &mut self.state,
            crate::copiable_abilities::CopyApplication {
                source: id,
                target,
                mods,
                copy_index: u32::try_from(index).expect("ability index fits"),
                until_eot,
                resolving: None,
                text,
            },
        );
    }

    /// Keeps the effect table in sync with the battlefield: registers
    /// static abilities of permanents, drops effects whose source left.
    #[allow(clippy::too_many_lines)] // Phasing, lapsed rules and provenance-aware registration form one sync pass.
    pub(crate) fn sync_static_effects(&mut self) {
        use baylee_cards_dsl::Duration;
        // A phased-out permanent's statics apply to nothing (CR 702.26b):
        // set aside while it is phased out, back as they were once it has
        // phased in (CR 702.26d). First, so the departure sweep below also
        // drops what a source that left while phased out had set aside.
        // phasing: this walk is the one looking for phased-out permanents.
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield);
        let phased_out: Vec<ObjectId> = battlefield
            .iter()
            .copied()
            .filter(|&id| {
                self.state
                    .object(id)
                    .is_some_and(|o| o.status.contains(crate::object::Status::PHASED_OUT))
            })
            .collect();
        self.state
            .effects
            .follow_phasing(|source| phased_out.contains(&source));
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
        // A blink can leave the same arena object on the battlefield with
        // a new rules identity. Its former printed/copied static must end
        // before the replacement registration affects the projection.
        let stale: Vec<_> = self
            .state
            .effects
            .iter()
            .filter_map(|effect| {
                if effect.origin != crate::effects::EffectOrigin::Static
                    || effect.duration != Duration::WhileSourceOnBattlefield
                {
                    return None;
                }
                let (_, origin) = self
                    .state
                    .effect_text_overrides
                    .iter()
                    .find(|(id, _)| *id == effect.id)?;
                let reference = match *origin {
                    crate::text_changes::TextOrigin::Live(source)
                    | crate::text_changes::TextOrigin::Ability { source, .. } => source,
                    crate::text_changes::TextOrigin::Frozen(_) => return None,
                };
                // A gained static may instead quote a different grantor: that
                // identity cannot be compared with the recipient's incarnation.
                (effect.source == Some(reference.object)
                    && self.state.source_identity(reference.object) != Some(reference))
                .then_some(effect.id)
            })
            .collect();
        self.state
            .effects
            .remove_where(|effect| stale.contains(&effect.id));
        end_control_durations(&mut self.state);
        forget_effects_on_moved_objects(&mut self.state);
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
        // statics are composed into the copiable ability list before entry.
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
        //
        // A static with a condition on its source (a station symbol's,
        // CR 721.2a) exists only while the condition holds: its effect is
        // registered on the pass that finds it true and removed on the pass
        // that finds it false. Both move the effect generation, which is the
        // projection's cache key, so no filter has to read the source.
        //
        // A permanent that has lost all its abilities (CR 613.1f) keeps only
        // what its statics do in layers 1, 2, 4 and 5: those effects began
        // before layer 6 took the abilities away and go on applying (CR
        // 613.6). The rest — layer 6 on, and every rules effect parked in
        // layer 3 — is gone with the ability, so it lapses like a static
        // whose condition failed. A modifier spanning multiple layers
        // (AnimateNoncreatureArtifact) is registered at its first layer,
        // so its later parts survive with it (CR 613.6). Separate statics
        // remain separate effects and do not share that continuation.
        // That reads the projection, so it is made current first: the effect
        // that took the abilities may have been registered a moment ago.
        self.state.refresh_characteristics();
        // Not a phased-out permanent: its statics wait parked for it, and a
        // condition on it is not asked while it does not exist.
        let ids: Vec<ObjectId> = self.state.battlefield_view();
        let mut to_register = Vec::new();
        let mut lapsed = Vec::new();
        for id in ids {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            let lost = obj.characteristics().abilities_lost.is_some();
            let text_lost = obj.characteristics().rules_text_lost;
            let list = obj.printed_ability_list(&self.lookup);
            for (index, ability) in list.abilities.iter().enumerate() {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                let identity = baylee_core::ids::DamageSourceRef {
                    object: id,
                    version: obj.version,
                };
                let origin = crate::text_changes::TextOrigin::Ability {
                    source: identity,
                    index: index as u32,
                    base: list.base_text(index),
                };
                let context = crate::text_changes::RuleContext {
                    source: id,
                    text: origin.resolve(&self.state.text_changes),
                };
                let registered = self
                    .state
                    .effects
                    .iter()
                    .find(|fx| {
                        fx.source == Some(id)
                            && fx.origin == crate::effects::EffectOrigin::Static
                            && fx.modifier == sa.modifier
                            && self
                                .state
                                .effect_text_overrides
                                .iter()
                                .find(|(effect, _)| *effect == fx.id)
                                .is_none_or(|(_, text)| *text == origin)
                    })
                    .map(|fx| fx.id);
                let gone_with_the_ability = (lost && !outlives_its_ability(sa.layer))
                    || (text_lost && !outlives_its_rules_text(sa.layer));
                if gone_with_the_ability
                    || sa.condition.is_some_and(|condition| {
                        !crate::eval::condition_holds_with_context(
                            &self.state,
                            obj.controller,
                            context,
                            condition,
                        )
                    })
                {
                    if let Some(effect) = registered {
                        lapsed.push(effect);
                    }
                    continue;
                }
                if let Some(effect) = registered {
                    // Adopt legacy registrations without an origin sidecar.
                    if !self
                        .state
                        .effect_text_overrides
                        .iter()
                        .any(|(id, _)| *id == effect)
                    {
                        to_register.push((None, effect, Some(origin)));
                    }
                    continue;
                }
                to_register.push((
                    Some(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(id),
                        controller: obj.controller,
                        origin: crate::effects::EffectOrigin::Static,
                        layer: sa.layer,
                        timestamp: obj.timestamp,
                        duration: Duration::WhileSourceOnBattlefield,
                        filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                        modifier: sa.modifier,
                    }),
                    baylee_core::ids::EffectId::new(0),
                    Some(origin),
                ));
            }
            for (offset, ability) in list.runtime_statics().iter().enumerate() {
                let layer = ability.modifier.layer();
                let origin = crate::text_changes::TextOrigin::Ability {
                    source: baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: obj.version,
                    },
                    index: u32::try_from(list.abilities.len() + offset)
                        .expect("ability index fits u32"),
                    base: ability.base_text,
                };
                let registered = self
                    .state
                    .effects
                    .iter()
                    .find(|fx| {
                        fx.source == Some(id)
                            && fx.origin == crate::effects::EffectOrigin::Static
                            && fx.modifier == ability.modifier
                            && self
                                .state
                                .effect_text_overrides
                                .iter()
                                .any(|(effect, text)| *effect == fx.id && *text == origin)
                    })
                    .map(|fx| fx.id);
                if (lost && !outlives_its_ability(layer))
                    || (text_lost && !outlives_its_rules_text(layer))
                {
                    if let Some(effect) = registered {
                        lapsed.push(effect);
                    }
                    continue;
                }
                if registered.is_none() {
                    to_register.push((
                        Some(crate::effects::ContinuousEffect {
                            id: baylee_core::ids::EffectId::new(0),
                            source: Some(id),
                            controller: obj.controller,
                            origin: crate::effects::EffectOrigin::Static,
                            layer,
                            timestamp: obj.timestamp,
                            duration: Duration::WhileSourceOnBattlefield,
                            filter: crate::effects::EffectFilter::object(&self.state, id),
                            modifier: ability.modifier,
                        }),
                        baylee_core::ids::EffectId::new(0),
                        Some(origin),
                    ));
                }
            }
        }
        self.state
            .effects
            .remove_where(|effect| lapsed.contains(&effect.id));
        to_register.extend(
            emblem_statics(&self.state)
                .into_iter()
                .map(|effect| (Some(effect), baylee_core::ids::EffectId::new(0), None)),
        );
        for (effect, existing, origin) in to_register {
            let id = effect.map_or(existing, |effect| self.state.effects.register(effect));
            if let Some(origin) = origin {
                self.state.effect_text_overrides.push((id, origin));
            }
        }
        crate::effects::sync_granted_statics(&mut self.state);
        self.sync_replacement_rules();
        #[cfg(test)]
        crate::ability_log::note_sources(&self.state, &self.lookup);
    }

    /// Drops the replacement rules of sources that left the battlefield,
    /// phased out (CR 702.26b) or lost their abilities (CR 613.1f) and
    /// registers the new ones. A permanent that phases in is scanned again
    /// like one that arrived, so its rules come back from its abilities.
    fn sync_replacement_rules(&mut self) {
        let gone_rules: Vec<ObjectId> = self
            .state
            .replacement_rules
            .iter()
            .map(|r| r.source)
            .filter(|s| {
                self.state.object(*s).is_none_or(|o| {
                    o.zone != Zone::Battlefield
                        || o.status.contains(crate::object::Status::PHASED_OUT)
                        || o.characteristics().abilities_lost.is_some()
                        || o.characteristics().rules_text_lost
                })
            })
            .collect();
        self.state
            .replacement_rules
            .retain(|r| !gone_rules.contains(&r.source));
        let mut rules_to_add = Vec::new();
        for id in self.state.battlefield_view() {
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
    pub(super) fn trigger_abilities(
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
            if let Some((player, amount)) = trigger.event_damage {
                object
                    .riders
                    .push(crate::object::Rider::EventAmount(amount));
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
    fn resolve_triggered_mana_abilities(&mut self) -> bool {
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
                    let first = super::PerOpponent {
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
                    let first = super::PerOpponent {
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
        if obj.target_req.is_some() || obj.kind != ObjectKind::AbilityOnStack {
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
        crate::object::ability_target_req(abilities, loc.index, obj.mode_index)
    }

    /// The damage a triggered ability on the stack divides as its controller
    /// chooses ([`baylee_cards_dsl::Effect::DealDamageDivided`]), read off
    /// the list it was put on the stack with, as its second target is.
    pub(super) fn stack_divided_amount(&self, on_stack: ObjectId) -> Option<u32> {
        let obj = self.state.object(on_stack)?;
        let loc = obj.ability?;
        if obj.kind != ObjectKind::AbilityOnStack || loc.index == AbilityRef::SYNTHETIC {
            return None;
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
        let AbilityDef::Triggered { effects, .. } = abilities.get(loc.index as usize)? else {
            return None;
        };
        effects.iter().find_map(|effect| match effect {
            baylee_cards_dsl::Effect::DealDamageDivided { amount } => Some(*amount),
            _ => None,
        })
    }

    /// What the top of the stack may target with its **second** instance of
    /// the word "target", read from the same places [`Self::stack_target_req`]
    /// reads the first: the spell's own object, or the ability's definition.
    ///
    /// Two arms and not the whole list, because only two shapes can say it —
    /// [`AbilityDef::Spell`] through the object and the activated twins here.
    /// Both twins, for the reason `stack_target_req` gives.
    pub(super) fn stack_second_target_req(&self, on_stack: ObjectId) -> Option<TargetReq> {
        let obj = self.state.object(on_stack)?;
        // A requirement written on the object wins: the cast wizard writes a
        // spell's, and a trigger's is written bound to the player its first
        // instance named (`Engine::ask_trigger_second_target`), which is the
        // question its re-check has to ask again.
        if obj.kind != ObjectKind::AbilityOnStack || obj.second_target_req().is_some() {
            return obj.second_target_req();
        }
        let loc = obj.ability?;
        if loc.index == AbilityRef::SYNTHETIC {
            return None;
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
        match abilities.get(loc.index as usize)? {
            AbilityDef::Activated { second_targets, .. }
            | AbilityDef::ActivatedConditional { second_targets, .. }
            | AbilityDef::Loyalty { second_targets, .. }
            | AbilityDef::Triggered { second_targets, .. } => *second_targets,
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
    /// CR 400.7: each announced slot names an incarnation, not the arena
    /// handle of a card which may since have left and returned.
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
            .enumerate()
            .filter_map(|(index, &id)| {
                let announced = self.state.recorded_target_reference(
                    obj.id,
                    !players,
                    u32::try_from(index).expect("target slot"),
                );
                (legal_objects.contains(&id)
                    && announced.is_some()
                    && announced == self.state.source_identity(id))
                .then_some(id)
            })
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
            self.state.remember_damage_source(top);
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
            self.state.remember_damage_source(top);
            let _ = self.state.arena.remove(top);
            return;
        }
        // CR 608.2b, asked in the same place and for the same reason: a
        // spell or ability all of whose targets have become illegal does not
        // resolve. Before the spell/ability split below, because an Aura is
        // a targeted *permanent* spell and a check inside either branch
        // would miss one of them.
        let mut resolution_object = self.state.object(top).expect("stack object exists").clone();
        match self.target_legality(top) {
            TargetLegality::AllIllegal => {
                self.state
                    .journal
                    .record(GameEvent::StackObjectDidNotResolve { object: top });
                self.leave_stack_without_resolving(top);
                return;
            }
            // "…won't do anything to an illegal target" (CR 608.2b): the
            // rest of it still happens. Narrow a local resolution image,
            // preserving the original announcement for copies and history.
            // Every resolution constructor below reads this same image.
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
                let obj = &mut resolution_object;
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
            TargetLegality::NotAsked => {}
        }
        self.begin_controlled_resolution(top);
        self.state
            .journal
            .record(GameEvent::StackObjectResolved { object: top });
        let kind = self.state.object(top).map(|o| o.kind);
        if kind == Some(ObjectKind::AbilityOnStack) {
            // "The second time this ability has resolved this turn": counted
            // here, as it begins to resolve, so the resolution asking is one
            // of those it counts. A synthetic keyword trigger has no index of
            // its own to count under.
            if let Some(loc) = self.state.object(top).and_then(|o| o.ability)
                && loc.index != baylee_core::ids::AbilityRef::SYNTHETIC
            {
                let version = self.state.object(loc.source).map_or(0, |o| o.version);
                self.state
                    .per_turn
                    .note_resolution(loc.source, version, loc.index);
            }
            let obj = &resolution_object;
            let loc = obj.ability.expect("ability object has a location");
            // The list `loc.index` points into, captured when the ability was
            // put on the stack — a card face, an emblem's stored list, a
            // token's definition or a copy's, all the same case by then. It
            // is read from the ability object rather than from the source
            // because the two have been separate objects since it was put
            // there (CR 113.7a): the source may have died, changed face, or
            // stopped being a copy in the meantime.
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
                    subject: crate::resolve::SubjectContext::default(),
                    text: crate::text_changes::TextChangeMap::IDENTITY,
                    event_mana: None,
                    retarget_left: None,
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
                subject: crate::resolve::SubjectContext::default(),
                text: crate::text_changes::TextChangeMap::IDENTITY,
                event_mana: None,
                retarget_left: None,
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
                    } if !effects.is_empty() => {
                        Some((resolve::flatten(effects), targets.is_some(), None))
                    }
                    _ => None,
                })
            })
            .or_else(|| self.modal_program(top));
        if let Some((program, targeted, retarget_left)) = spell_fx {
            let obj = &resolution_object;
            let mut res = Resolution {
                source: top,
                on_stack: top,
                controller: obj.controller,
                effects: program,
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
                subject: crate::resolve::SubjectContext::default(),
                text: crate::text_changes::TextChangeMap::IDENTITY,
                retarget_left,
                event_mana: None,
            };
            #[cfg(test)]
            crate::ability_log::resolving(&self.state, &self.lookup, top);
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => self.finish_resolution(&res),
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                }
            }
        } else {
            #[cfg(test)]
            crate::ability_log::resolved(&self.state, &self.lookup, top);
            self.finalize_spell(top);
        }
    }

    /// What a modal spell on the stack does: its chosen mode's effects, or,
    /// for a spell cast with several (`GameObject::modes`), every chosen
    /// mode's, in the order they are printed and not the order they were
    /// picked in (CR 608.2c).
    ///
    /// Beside the program, whether it targets, and where the second of its
    /// modes that says "target" begins: that mode's targets were chosen as
    /// the spell's second instance of the word, and from there on they are
    /// the ones "target" means (`Resolution::retarget_left`).
    fn modal_program(&self, top: ObjectId) -> Option<(Vec<Effect>, bool, Option<usize>)> {
        let obj = self.state.object(top)?;
        let def = obj.card.and_then(|c| self.lookup.card(c.index))?;
        let modes = def
            .abilities_for_face(obj.face_index as usize)
            .iter()
            .find_map(|a| match a {
                AbilityDef::ModalSpell { modes, .. } => Some(*modes),
                _ => None,
            })?;
        if obj.modes != 0 {
            let mut program = Vec::new();
            let mut targeting = 0_usize;
            let mut second = None;
            for (_, mode) in crate::casting::chosen_modes(modes, obj.modes) {
                if mode.targets.is_some() {
                    if targeting == 1 {
                        second = Some(program.len());
                    }
                    targeting += 1;
                }
                program.extend(resolve::flatten(mode.effects));
            }
            let left = second.map(|at| program.len() - at);
            return Some((program, targeting > 0, left));
        }
        let mode = modes.get(usize::from(obj.mode_index?))?;
        Some((resolve::flatten(mode.effects), mode.targets.is_some(), None))
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
                    token: None,
                    abilities: def.abilities_for_face(face as usize).into(),
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
        // The common case by a wide margin: no daybound card at the table,
        // so the whole step is one scan of the battlefield and out. A
        // phased-out permanent is not at the table (CR 702.26b).
        let mut any_daybound = false;
        let mut any_nightbound = false;
        for id in self.state.battlefield_seen() {
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
            .battlefield_seen()
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
        #[cfg(test)]
        if res.mana_ability {
            crate::ability_log::mana_finished(&self.lookup, res.source);
        } else {
            crate::ability_log::resolved(&self.state, &self.lookup, res.on_stack);
        }
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
        // What this resolution left for the engine to do as it ends
        // (cascade's cast, CR 702.85a), ahead of everything already queued.
        let mut i = self.state.delayed.len();
        while i > 0 {
            i -= 1;
            if matches!(
                self.state.delayed[i].when,
                crate::state::DelayedWhen::AsResolutionEnds
            ) {
                let trigger = self.state.delayed.remove(i);
                self.delayed_queue
                    .push_front((trigger.controller, trigger.action));
            }
        }
        // A card this resolution discovered is offered once it is over
        // (CR 701.57a), through the queue step 3b of `run_machine` drains
        // before anybody receives priority. CR 608.2g casts it *during* the
        // resolution; the difference is only in what the resolution's own
        // triggers see, which step 3 stacks first.
        for (player, card, version) in self.state.discovered.drain(..) {
            self.delayed_queue.push_back((
                player,
                crate::state::DelayedAction::CastDiscovered { card, version },
            ));
        }
        // A copy of a synthetic ability (CR 707.10) takes the original's
        // effects, which live here and not on the object the resolver made.
        for (original, copy) in std::mem::take(&mut self.state.synthetic_copies) {
            if let Some(&effects) = self.synthetic_fx.get(&original) {
                self.synthetic_fx.insert(copy, effects);
            }
        }
        // A mana ability never went on the stack (CR 605.3b), and its
        // `on_stack` is the source permanent itself. Falling through here
        // treated that permanent as a resolving spell: `finalize_spell`
        // untapped the land and "moved" it to the battlefield it was
        // already on, so every colour-choice source — Badlands, City of
        // Brass, Command Tower, Harabaz Druid — untapped itself and made
        // unbounded mana. Nothing to finalize; the activating player keeps
        // priority (CR 605.3a).
        if res.mana_ability {
            // A triggered mana ability may belong to another player (Mana
            // Flare). Its choice returns to the original mana activator.
            self.after_action(res.event_mana.map_or(res.controller, |event| event.player));
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
            self.state.remember_damage_source(res.on_stack);
            let _ = self.state.arena.remove(res.on_stack);
        } else {
            self.finalize_spell(res.on_stack);
        }
        self.apply_pending_face_changes();
        self.end_controlled_resolution(res.on_stack);
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
            .map(|o| o.printed_ability_list(&self.lookup));
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
            self.state.begin_permanent_resolution(spell);
            // Dash (CR 702.109a): "return the permanent this spell becomes to
            // its owner's hand at the beginning of the next end step" — a
            // delayed triggered ability that uses the stack (CR 603.7), and
            // asks as it resolves whether it is still that permanent.
            if let Some(obj) = self.state.object(spell)
                && obj.riders.contains(&crate::object::Rider::Dashed)
            {
                let controller = obj.controller;
                self.state
                    .delay_for_resolved_permanent(spell, controller, &DASH_RETURN);
            }
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
            let spells = &self.state.per_turn.spells_cast;
            crate::state::PreviousTurn {
                active,
                spells_cast: spells[active.get() as usize],
                spells_by_all: spells.iter().sum(),
                most_by_one: spells.iter().copied().max().unwrap_or(0),
            }
        });
        if !first_turn {
            // Turn order goes on from the last turn that ended or was
            // skipped (CR 614.10).
            let after = match self.cleanup {
                Cleanup::Ended { after } => after,
                _ => self.state.turn.active,
            };
            self.cleanup = Cleanup::Due;
            // Extra turns (CR 500.7) preempt the normal successor.
            let next = self
                .state
                .extra_turns
                .pop_front()
                .unwrap_or_else(|| self.next_alive_after(after));
            self.state.turn.active = next;
            self.state.turn.number += 1;
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

    /// Asks `player` about the first draw-step skip they control and have
    /// not declined for this draw (Island Sanctuary,
    /// `ReplacementRule::MaySkipDrawStepDraw`). The draw is made by the
    /// answer, so a `true` here means nothing has been drawn yet.
    pub(crate) fn offer_draw_skip(&mut self, player: PlayerId, declined: Vec<ObjectId>) -> bool {
        use baylee_cards_dsl::{AbilityDef, ReplacementRule};
        let state = &self.state;
        let Some(source) = state
            .replacement_rules
            .iter()
            .filter(|r| r.rule == ReplacementRule::MaySkipDrawStepDraw && r.controller == player)
            .map(|r| r.source)
            .filter(|s| !declined.contains(s))
            .find(|s| {
                state.object(*s).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && o.controller == player
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
                    AbilityDef::Replacement(ReplacementRule::MaySkipDrawStepDraw)
                )
            })?;
            list.entry(index)?.provenance.ability_ref()
        });
        self.pending_plan = Some(PlanKind::SkipDraw { source, declined });
        self.pending = Pending::YesNo {
            player,
            prompt: YesNoPrompt::MayDo,
            source: ability,
        };
        self.awaiting_answer = true;
        true
    }

    /// "If you do, until your next turn, you can't be attacked except by
    /// creatures with flying and/or islandwalk." Created by the skip and not
    /// by the permanent, so it holds once the source has left (the card's
    /// ruling); `Duration::UntilYourNextTurn` ends it as `player`'s next
    /// turn begins.
    pub(crate) fn restrict_attacks_after_skipped_draw(
        &mut self,
        player: PlayerId,
        source: ObjectId,
    ) {
        static FLYING_OR_ISLANDWALK: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::Or(&[
            baylee_cards_dsl::Filter::HasKeyword(baylee_cards_dsl::KeywordSet::FLYING),
            baylee_cards_dsl::Filter::HasKeyword(baylee_cards_dsl::KeywordSet::ISLANDWALK),
        ]);
        let modifier = baylee_cards_dsl::Modifier::CantBeAttackedExceptBy {
            who: baylee_cards_dsl::PlayerRel::You,
            by: &FLYING_OR_ISLANDWALK,
        };
        let timestamp = self.state.next_timestamp();
        self.state
            .effects
            .register(crate::effects::ContinuousEffect {
                // `register` assigns the real one.
                id: baylee_core::ids::EffectId::new(0),
                source: Some(source),
                controller: player,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: baylee_cards_dsl::Duration::UntilYourNextTurn,
                filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
                modifier,
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
    fn synthetic_source_name(&mut self, source: ObjectId) -> NameRef {
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
    fn run_state_based_actions(&mut self) -> sba::SbaOutcome {
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
                    text: crate::text_changes::TextChangeMap::IDENTITY,
                    source_version: None,
                    event_object_identity: None,
                    counter_source_version: None,
                    event_mana: None,
                    event_mana_value: None,
                    event_departure: None,
                    event_damage: None,
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

    /// Cascade's cast (CR 702.85a), as its resolution ends: the exiled
    /// card is cast without paying its mana cost, or put on the bottom of
    /// its owner's library when it cannot be. Returns whether a question is
    /// out.
    fn cast_free_or_bottom(&mut self, controller: PlayerId, card: ObjectId, version: u32) -> bool {
        let in_exile = |state: &crate::state::GameState| {
            state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
                .map(|o| o.owner)
        };
        if in_exile(&self.state).is_none() {
            return false;
        }
        // A spell nothing can be pointed at cannot be cast
        // (CR 601.2c); asked first, so a refusal leaves no wizard
        // behind to unwind.
        let cast =
            crate::casting::face_has_a_legal_target(&self.state, &self.lookup, controller, card, 0)
                && self.start_permitted_free_cast(controller, card).is_ok();
        if !cast && let Some(owner) = in_exile(&self.state) {
            // "…that weren't cast on the bottom of your library."
            let _ = self.state.move_object(
                card,
                ZoneLocation::Library(owner),
                ZonePosition::Bottom,
                Cause::Effect,
            );
        }
        self.awaiting_answer
    }

    /// `Effect::MayCastTarget`'s cast, said yes to: a CR 605.3a payment
    /// window for the card's mana cost, where the caster makes the mana
    /// the cast will be paid with (CR 608.2g lets them). Nothing opens for
    /// a card that has left the zone it was targeted in (CR 400.7) or that
    /// its caster may not begin to cast at all (CR 601.3). Returns whether
    /// a question is out.
    fn open_cast_payment(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        version: u32,
        then_no_more_spells: bool,
    ) -> bool {
        let Some(cost) = self
            .state
            .object(card)
            .filter(|o| o.version == version)
            .map(|o| o.characteristics().mana_cost)
        else {
            return false;
        };
        if !crate::casting::may_begin_casting(&self.state, player) {
            return false;
        }
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        self.mana_window = Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::Cast {
                card,
                version,
                cost,
                then_no_more_spells,
                opened: Box::new(self.window_start(player)),
            },
        });
        self.pending = Pending::Priority {
            player,
            legal: Box::new(legal),
        };
        self.awaiting_answer = true;
        true
    }

    /// Offers a discovered card's cast to the player who discovered it
    /// (CR 701.57a), if it is still the card in exile that was found
    /// (`version`) — or puts it into the hand without asking when it cannot
    /// be cast. Returns `true` when the question is out.
    fn offer_discovered(&mut self, player: PlayerId, card: ObjectId, version: u32) -> bool {
        if !self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
        {
            return false;
        }
        if !self.free_cast_possible(player, card) {
            self.discovered_to_hand(card);
            return false;
        }
        self.pending_plan = Some(PlanKind::Discovered { card });
        self.pending = Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::Discover { card },
            source: None,
        };
        self.awaiting_answer = true;
        true
    }

    /// A discovered card that is not cast goes to its owner's hand
    /// (CR 701.57a), if it is still in exile.
    ///
    /// A priority grant already out is asked again: this runs after a free
    /// cast the wizard refused, which resumes the game on its way out, and
    /// the card joins a hand that grant's legal actions did not see.
    pub(crate) fn discovered_to_hand(&mut self, card: ObjectId) {
        let Some(owner) = self
            .state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Exile)
            .map(|o| o.owner)
        else {
            return;
        };
        let _ = self.state.move_object(
            card,
            ZoneLocation::Hand(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
        if let Pending::Priority { player, .. } = self.pending {
            self.pending = Pending::Priority {
                player,
                legal: Box::new(self.compute_legal(player)),
            };
        }
    }

    /// A delayed "transform it": only the permanent it was made for, on the
    /// battlefield as the same object and still showing the same face
    /// (CR 400.7), turns over.
    fn transform_if_unchanged(&mut self, card: ObjectId, version: u32, face: u8) {
        let still_there = self.state.object(card).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield && o.version == version && o.face_index == face
        });
        if still_there
            && let Some(def) = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
        {
            self.state
                .transform(card, def, 1 - usize::from(face.min(1)));
        }
    }

    /// Processes one queued delayed action; returns `true` when a pending
    /// choice was produced.
    #[allow(clippy::too_many_lines)] // one arm per delayed action; splitting hides the list
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
            crate::state::DelayedAction::CastFreeOrBottom { card, version } => {
                self.cast_free_or_bottom(controller, card, version)
            }
            crate::state::DelayedAction::CastPaying {
                card,
                version,
                then_no_more_spells,
            } => self.open_cast_payment(controller, card, version, then_no_more_spells),
            crate::state::DelayedAction::Transform {
                card,
                version,
                face,
            } => {
                self.transform_if_unchanged(card, version, face);
                false
            }
            crate::state::DelayedAction::Sacrifice { card, version } => {
                let owner = self.state.object(card).and_then(|o| {
                    (o.zone == crate::zone::Zone::Battlefield
                        && o.version == version
                        && o.controller == controller)
                        .then_some(o.owner)
                });
                if let Some(owner) = owner {
                    let _ = self.state.move_object(
                        card,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        crate::event::Cause::Effect,
                    );
                }
                false
            }
            crate::state::DelayedAction::CastDiscovered { card, version } => {
                self.offer_discovered(controller, card, version)
            }
            crate::state::DelayedAction::ReturnToBattlefield { card, version } => {
                if self
                    .state
                    .object(card)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile && o.version == version)
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
                if !self.state.players[controller.get() as usize]
                    .mana_pool
                    .try_add(color, u32::from(amount))
                {
                    self.state.numeric_failure = Some("delayed mana exceeds u32 per color");
                    return false;
                }
                self.state.journal.record(GameEvent::ManaProduced {
                    player: controller,
                    color,
                    amount,
                    source: None,
                });
                false
            }
            crate::state::DelayedAction::PayCostOrSacrifice {
                cost,
                card,
                version,
            } => {
                if self
                    .state
                    .object(card)
                    .is_some_and(|o| o.version == version)
                {
                    self.demand_echo(cost, card)
                } else {
                    false
                }
            }
            crate::state::DelayedAction::PayCostOrLose { cost } => self.demand_pact(cost),
            // A delayed triggered ability that uses the stack, come due at a
            // step: it joins the trigger queue and is put on the stack from
            // there, as any trigger is. Earthbend's watch is read off the
            // journal (`trigger::watch_triggers`) and never reaches this
            // queue; no step-timed one exists yet.
            crate::state::DelayedAction::LinkedCounterCleanup {
                source,
                version,
                effects,
            } => {
                self.queue_delayed_trigger(
                    controller,
                    source,
                    effects,
                    None,
                    Some(version),
                    crate::text_changes::TextChangeMap::IDENTITY,
                );
                if let Some(trigger) = self.trigger_queue.back_mut() {
                    trigger.counter_source_version = Some(version);
                }
                false
            }
            crate::state::DelayedAction::Trigger {
                source,
                source_version,
                effects,
                text,
            } => {
                self.queue_delayed_trigger(
                    controller,
                    source,
                    effects,
                    None,
                    Some(source_version),
                    text,
                );
                false
            }
            // "That creature" while it is still that object (CR 603.7c):
            // one that has left the battlefield, even to come back, is a
            // new object and the trigger is about nothing (CR 400.7).
            crate::state::DelayedAction::TriggerAbout {
                source,
                source_version,
                effects,
                text,
                object,
                version,
            } => {
                let still = self
                    .state
                    .object(object)
                    .is_some_and(|o| o.version == version);
                self.queue_delayed_trigger(
                    controller,
                    source,
                    effects,
                    still.then_some(object),
                    Some(source_version),
                    text,
                );
                false
            }
        }
    }

    /// A delayed triggered ability come due at a step joins the trigger
    /// queue, and is put on the stack from there (CR 603.7).
    fn queue_delayed_trigger(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        effects: &'static [baylee_cards_dsl::Effect],
        event_object: Option<ObjectId>,
        source_version: Option<u32>,
        text: crate::text_changes::TextChangeMap,
    ) {
        self.trigger_queue
            .push_back(crate::trigger::PendingTrigger {
                text,
                source_version,
                event_object_identity: None,
                counter_source_version: None,
                event_damage: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                source,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                abilities: None,
                controller,
                timestamp: self.state.object(source).map_or(0, |o| o.timestamp),
                event_object,
                implicit_target: None,
                synthetic_effects: Some(effects),
                once_per_turn: false,
                synthetic_target: None,
                chosen_mode: None,
            });
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
        let can_pay = super::casting::affordable(
            &self.state,
            active,
            &self.state.players[active.get() as usize].mana_pool,
            &cost,
        );
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
        // This is "pay", not "you may pay": available mana must be spent.
        // An empty pool is not a refusal. CR 605.3a allows mana abilities
        // while an effect asks for payment, including this delayed debt.
        if super::casting::pay_mana(&mut self.state, active, &cost) {
            return false;
        }
        self.pending_plan = Some(PlanKind::DelayedPay { cost });
        self.pending = Pending::YesNo {
            player: active,
            prompt: YesNoPrompt::PayPact { cost },
            // A pact's "pay or lose" must never be automatable:
            // a standing "no" here is a standing loss.
            source: None,
        };
        self.awaiting_answer = true;
        true
    }

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

    pub(super) fn finish_combat_damage(&mut self, work: &crate::damage::DamageWork) {
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
                        let active = self.state.turn.active;
                        if !self.offer_draw_skip(active, Vec::new()) {
                            self.state.draw_cards(active, 1);
                        }
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
