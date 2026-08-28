//! The engine driver: the complete turn/priority state machine.
//!
//! Public contract (see `docs/engine-internals.md`): the game only advances
//! through [`Engine::apply`] answering [`Engine::pending`]. Everything else
//! — SBAs, stack resolution, turn-based actions — is automatic.

use crate::casting::{self, CastFailure};
use crate::choice::{LegalActions, Pending, PlayerAction};
use crate::combat::{self, AttackerInfo, BlockerInfo};
use crate::event::{Cause, GameEvent, LossReason};
use crate::object::Status;
use crate::sba;
use crate::state::{CardLookup, GameState, SetupError, StateError};
use crate::turn::{Phase, Step};
use crate::win::{EndReason, GameResult};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{GamePreset, HouseRules};
use baylee_core::types::TypeSet;

/// Engine API errors.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The game already ended.
    #[error("game is over")]
    GameOver,
    /// The action does not match the pending request.
    #[error("action does not match the pending request")]
    MismatchedAction,
    /// The action is not legal right now.
    #[error("illegal action: {0}")]
    IllegalAction(&'static str),
    /// Setup failed.
    #[error("setup: {0}")]
    Setup(#[from] SetupError),
    /// Casting/playing failed.
    #[error("casting: {0}")]
    Cast(#[from] CastFailure),
    /// Zone machinery failure.
    #[error("state: {0}")]
    State(#[from] StateError),
}

/// Which combat declaration has already happened this step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CombatDeclared {
    None,
    Attackers,
    Blockers,
}

/// A deterministic, self-contained game of Magic.
pub struct Engine {
    state: GameState,
    pending: Pending,
    house_rules: HouseRules,
    /// Consecutive priority passes since the last non-pass action.
    passes: u8,
    /// Who holds priority in the current APNAP round (`None` = no round).
    priority_holder: Option<PlayerId>,
    /// All players passed with a non-empty stack: resolve the top.
    resolve_next: bool,
    /// Mulligan progress per seat.
    mulligans: Vec<u8>,
    /// Seat currently mulliganing.
    mulligan_player: usize,
    /// Combat declaration progress.
    combat_declared: CombatDeclared,
    /// `true` while the current pending request is unanswered — the
    /// progression machine must never overwrite a fresh pending.
    awaiting_answer: bool,
}

impl Engine {
    /// Creates a game from a preset. The first pending request is the
    /// first seat's mulligan decision.
    ///
    /// # Errors
    /// [`EngineError::Setup`] for invalid presets or unknown cards.
    pub fn new(preset: &GamePreset, lookup: &impl CardLookup) -> Result<Self, EngineError> {
        let state = GameState::from_preset(preset, lookup)?;
        let mut engine = Self {
            mulligans: vec![0; state.players.len()],
            mulligan_player: 0,
            house_rules: preset.house_rules.clone(),
            state,
            pending: Pending::Mulligan {
                player: PlayerId::new(0),
                taken: 0,
                next_is_free: preset.house_rules.mulligan_free_first,
            },
            passes: 0,
            priority_holder: None,
            resolve_next: false,
            combat_declared: CombatDeclared::None,
            awaiting_answer: true,
        };
        engine.state.turn_start_timestamp = engine.state.timestamp;
        Ok(engine)
    }

    /// The current pending request.
    #[must_use]
    pub fn pending(&self) -> &Pending {
        &self.pending
    }

    /// Read-only state access (tests, debugging, dev mode).
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// Mutable state access for dev-mode commands (dev games only).
    #[must_use]
    pub fn state_mut_dev(&mut self) -> &mut GameState {
        &mut self.state
    }

    /// The journal.
    #[must_use]
    pub fn journal(&self) -> &crate::event::Journal {
        &self.state.journal
    }

    /// Determinism hash.
    #[must_use]
    pub fn snapshot_hash(&self) -> u64 {
        self.state.snapshot_hash()
    }

    /// Applies a player's action and advances automatically until the next
    /// decision point.
    ///
    /// # Errors
    /// [`EngineError`] on mismatched/illegal actions.
    pub fn apply(&mut self, player: PlayerId, action: PlayerAction) -> Result<(), EngineError> {
        if matches!(self.pending, Pending::GameOver(_)) {
            return Err(EngineError::GameOver);
        }
        // Concession is always legal for any seated player (CR 104.3a).
        if let PlayerAction::Concede = action {
            sba::eliminate_player(&mut self.state, player, LossReason::Conceded);
            self.awaiting_answer = false;
            self.run_until_choice();
            return Ok(());
        }
        self.awaiting_answer = false;
        self.apply_inner(player, action)?;
        self.run_until_choice();
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // flat match over the choice taxonomy — splitting would obscure, not clarify
    fn apply_inner(&mut self, player: PlayerId, action: PlayerAction) -> Result<(), EngineError> {
        match (&self.pending, action) {
            (Pending::Mulligan { player: p, .. }, PlayerAction::MulliganKeep) if *p == player => {
                let taken = self.mulligans[player.get() as usize];
                let bottom = self.mulligan_bottom_count(taken);
                if bottom > 0 {
                    self.pending = Pending::MulliganBottom {
                        player,
                        count: bottom,
                    };
                    self.awaiting_answer = true;
                } else {
                    self.advance_mulligan();
                }
                Ok(())
            }
            (
                Pending::Mulligan {
                    player: p, taken, ..
                },
                PlayerAction::MulliganTake,
            ) if *p == player => {
                // Hand goes back, reshuffle, draw 7 (CR 103.5).
                let hand: Vec<_> = self.state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                self.state.shuffle_library(player);
                self.state.draw_cards(player, 7);
                let taken = *taken + 1;
                self.mulligans[player.get() as usize] = taken;
                self.pending = Pending::Mulligan {
                    player,
                    taken,
                    next_is_free: false,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            (
                Pending::MulliganBottom { player: p, count },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != *count as usize {
                    return Err(EngineError::IllegalAction(
                        "must bottom exactly the required number of cards",
                    ));
                }
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                for card in objects {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                self.advance_mulligan();
                Ok(())
            }
            (Pending::Priority { player: p, .. }, PlayerAction::PassPriority) if *p == player => {
                self.passes += 1;
                Ok(())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::PlayLand { card })
                if *p == player =>
            {
                if !legal.lands.contains(&card) {
                    return Err(EngineError::IllegalAction("land not playable now"));
                }
                casting::play_land(&mut self.state, player, card)?;
                self.after_action(player);
                Ok(())
            }
            (Pending::Priority { player: p, legal }, PlayerAction::CastSpell { card })
                if *p == player =>
            {
                if !legal.castable.contains(&card) {
                    return Err(EngineError::IllegalAction("spell not castable now"));
                }
                casting::cast_spell(&mut self.state, player, card)?;
                self.after_action(player);
                Ok(())
            }
            (
                Pending::Priority { player: p, legal },
                PlayerAction::ActivateManaAbility { source },
            ) if *p == player => {
                if !legal.mana_abilities.contains(&source) {
                    return Err(EngineError::IllegalAction("mana ability not activatable"));
                }
                casting::activate_mana(&mut self.state, player, source)?;
                self.after_action(player);
                Ok(())
            }
            (
                Pending::ChooseAttackers { player: p },
                PlayerAction::DeclareAttackers { attackers },
            ) if *p == player => self.declare_attackers(player, attackers),
            (
                Pending::ChooseBlockers { player: p, .. },
                PlayerAction::DeclareBlockers { blockers },
            ) if *p == player => self.declare_blockers(player, blockers),
            (
                Pending::DiscardChoice { player: p, count },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != *count as usize {
                    return Err(EngineError::IllegalAction(
                        "must discard exactly the required number",
                    ));
                }
                for card in &objects {
                    if !self.in_hand(player, *card) {
                        return Err(EngineError::IllegalAction("card not in hand"));
                    }
                }
                for card in objects {
                    self.state.journal.record(GameEvent::Discarded {
                        object: card,
                        player,
                    });
                    self.state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::TurnBased,
                    )?;
                }
                self.end_cleanup();
                Ok(())
            }
            (
                Pending::LegendChoice { player: p, options },
                PlayerAction::ChooseObjects { objects },
            ) if *p == player => {
                if objects.len() != 1 || !options.contains(&objects[0]) {
                    return Err(EngineError::IllegalAction(
                        "choose exactly one legendary permanent to keep",
                    ));
                }
                let options = options.clone();
                sba::apply_legend_choice(&mut self.state, player, objects[0], &options);
                Ok(())
            }
            _ => Err(EngineError::MismatchedAction),
        }
    }

    fn in_hand(&self, player: PlayerId, card: ObjectId) -> bool {
        self.state
            .object(card)
            .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player))
    }

    fn after_action(&mut self, player: PlayerId) {
        // After any non-pass action, priority returns to the acting player
        // (CR 117.3c) and the pass counter resets.
        self.passes = 0;
        self.priority_holder = Some(player);
        self.pending = Pending::Priority {
            player,
            legal: Box::new(self.compute_legal(player)),
        };
        self.awaiting_answer = true;
    }

    fn mulligan_bottom_count(&self, taken: u8) -> u8 {
        taken.saturating_sub(u8::from(self.house_rules.mulligan_free_first))
    }

    fn advance_mulligan(&mut self) {
        self.mulligan_player += 1;
        if self.mulligan_player >= self.state.players.len() {
            self.begin_turn(true);
        } else {
            let player = PlayerId::new(self.mulligan_player as u8);
            self.pending = Pending::Mulligan {
                player,
                taken: 0,
                next_is_free: self.house_rules.mulligan_free_first,
            };
            self.awaiting_answer = true;
        }
    }

    /// The automatic progression machine: SBAs, stack resolution, and
    /// step/turn transitions until a decision is required.
    fn run_until_choice(&mut self) {
        if self.awaiting_answer {
            return;
        }
        loop {
            // 1. Game over?
            if let Some(result) = self.game_result() {
                self.pending = Pending::GameOver(result);
                self.awaiting_answer = true;
                self.state.journal.record(GameEvent::GameWon {
                    player: result.winner,
                });
                return;
            }
            // 2. State-based actions (fixpoint).
            let outcome = sba::run(&mut self.state);
            if let Some((player, options)) = outcome.legend_choice {
                self.pending = Pending::LegendChoice { player, options };
                self.awaiting_answer = true;
                return;
            }
            if outcome.changed {
                continue;
            }
            // 3. Resolve the top of the stack after all passed.
            if self.resolve_next {
                self.resolve_next = false;
                self.passes = 0;
                self.priority_holder = None;
                casting::resolve_top(&mut self.state);
                continue;
            }
            // 4. Progress the step machine.
            if self.progress_step() {
                return; // a pending choice was set
            }
        }
    }

    /// One step-machine transition. Returns `true` when a pending choice
    /// was produced (loop must stop).
    fn progress_step(&mut self) -> bool {
        match self.state.turn.step {
            Step::Untap => {
                self.untap_step();
                false
            }
            Step::Cleanup => self.cleanup_step(),
            Step::DeclareAttackers if self.combat_declared != CombatDeclared::Attackers => {
                self.pending = Pending::ChooseAttackers {
                    player: self.state.turn.active,
                };
                self.awaiting_answer = true;
                true
            }
            Step::DeclareBlockers if self.combat_declared != CombatDeclared::Blockers => {
                let active = self.state.turn.active;
                let defending = self.next_alive_after(active);
                self.pending = Pending::ChooseBlockers {
                    player: defending,
                    attacker: active,
                };
                self.awaiting_answer = true;
                true
            }
            _ => self.priority_round(),
        }
    }

    /// Runs the APNAP priority round for steps that grant priority.
    /// Returns `true` when a pending choice was produced.
    fn priority_round(&mut self) -> bool {
        if self.priority_holder.is_none() && self.passes == 0 {
            // Open a new round with the active player (CR 117.3a).
            let active = self.state.turn.active;
            self.priority_holder = Some(active);
            self.pending = Pending::Priority {
                player: active,
                legal: Box::new(self.compute_legal(active)),
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

    fn alive_players(&self) -> Vec<PlayerId> {
        self.state
            .players
            .iter()
            .filter(|p| !p.has_lost)
            .map(|p| p.id)
            .collect()
    }

    fn next_alive_after(&self, player: PlayerId) -> PlayerId {
        let n = self.state.players.len() as u8;
        let start = player.get();
        for offset in 1..=n {
            let candidate = PlayerId::new((start + offset) % n);
            if !self.state.players[candidate.get() as usize].has_lost {
                return candidate;
            }
        }
        player
    }

    fn game_result(&self) -> Option<GameResult> {
        let alive = self.alive_players();
        match alive.len() {
            0 => Some(GameResult {
                winner: None,
                reason: EndReason::Draw,
            }),
            1 => Some(GameResult {
                winner: Some(alive[0]),
                reason: EndReason::LastPlayerStanding,
            }),
            _ => None,
        }
    }

    fn compute_legal(&self, player: PlayerId) -> LegalActions {
        let mut legal = LegalActions {
            can_pass: true,
            ..LegalActions::default()
        };
        let main_phase = matches!(self.state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        let sorcery_timing =
            main_phase && self.state.turn.active == player && self.state.zones.stack_is_empty();
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            if obj.characteristics().types.contains(TypeSet::LAND)
                && sorcery_timing
                && self.state.players[player.get() as usize].lands_played_this_turn == 0
            {
                legal.lands.push(card);
            }
            if casting::can_cast(&self.state, player, card).is_ok() {
                legal.castable.push(card);
            }
        }
        for &id in self.state.zones.list(ZoneLocation::Battlefield) {
            if casting::can_activate_mana(&self.state, player, id) {
                legal.mana_abilities.push(id);
            }
        }
        legal
    }

    // ------------------------------------------------------------ combat

    fn declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, PlayerId)>,
    ) -> Result<(), EngineError> {
        let mut seen = Vec::with_capacity(attackers.len());
        for (creature, defending) in &attackers {
            if !combat::can_attack(&self.state, player, *creature) {
                return Err(EngineError::IllegalAction("creature cannot attack"));
            }
            if *defending == player || self.state.players[defending.get() as usize].has_lost {
                return Err(EngineError::IllegalAction("invalid defending player"));
            }
            if seen.contains(creature) {
                return Err(EngineError::IllegalAction("duplicate attacker"));
            }
            seen.push(*creature);
        }
        for (creature, defending) in attackers {
            let vigilance = self.state.object(creature).is_some_and(|o| {
                o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::VIGILANCE)
            });
            if !vigilance {
                if let Some(obj) = self.state.object_mut(creature) {
                    obj.status.insert(Status::TAPPED);
                }
                self.state.journal.record(GameEvent::ObjectTapped {
                    object: creature,
                    cause: Cause::TurnBased,
                });
            }
            self.state.combat.attackers.push(AttackerInfo {
                creature,
                defending,
            });
            self.state.journal.record(GameEvent::BecameAttacker {
                object: creature,
                defending,
            });
        }
        self.combat_declared = CombatDeclared::Attackers;
        self.passes = 0;
        self.priority_holder = None;
        Ok(())
    }

    fn declare_blockers(
        &mut self,
        defending: PlayerId,
        blockers: Vec<(ObjectId, ObjectId)>,
    ) -> Result<(), EngineError> {
        let mut seen = Vec::with_capacity(blockers.len());
        for (blocker, attacker) in &blockers {
            if !self
                .state
                .combat
                .attackers
                .iter()
                .any(|a| a.creature == *attacker)
            {
                return Err(EngineError::IllegalAction("no such attacker"));
            }
            if !combat::can_block(&self.state, defending, *blocker, *attacker) {
                return Err(EngineError::IllegalAction("creature cannot block"));
            }
            if seen.contains(blocker) {
                return Err(EngineError::IllegalAction("duplicate blocker"));
            }
            seen.push(*blocker);
        }
        // Menace: needs two blockers per attacker (CR 702.110).
        for attacker in &self.state.combat.attackers {
            let has_menace = self.state.object(attacker.creature).is_some_and(|o| {
                o.characteristics()
                    .keywords
                    .contains(baylee_cards_dsl::KeywordSet::MENACE)
            });
            if has_menace {
                let count = blockers
                    .iter()
                    .filter(|(_, a)| *a == attacker.creature)
                    .count();
                if count == 1 {
                    return Err(EngineError::IllegalAction("menace requires two blockers"));
                }
            }
        }
        for (blocker, attacker) in blockers {
            self.state
                .combat
                .blockers
                .push(BlockerInfo { blocker, attacker });
            self.state.journal.record(GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            });
        }
        self.combat_declared = CombatDeclared::Blockers;
        self.passes = 0;
        self.priority_holder = None;
        Ok(())
    }

    // --------------------------------------------------------- turn steps

    fn begin_turn(&mut self, first_turn: bool) {
        if !first_turn {
            let next = self.next_alive_after(self.state.turn.active);
            self.state.turn.active = next;
            self.state.turn.number += 1;
        }
        self.state.turn_start_timestamp = self.state.timestamp;
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

    fn advance_step(&mut self) {
        let (phase, step) = (self.state.turn.phase, self.state.turn.step);
        let (next_phase, next_step) = match (phase, step) {
            (_, Step::Untap) => (Phase::Beginning, Step::Upkeep),
            (_, Step::Upkeep) => (Phase::Beginning, Step::Draw),
            (_, Step::Draw) => {
                // Turn-based action: draw (first player skips on turn 1 in
                // two-player games, CR 103.8).
                let skip = self.state.turn.number == 1
                    && self.state.players.len() == 2
                    && self.state.turn.active.get() == 0;
                if !skip {
                    let active = self.state.turn.active;
                    self.state.draw_cards(active, 1);
                }
                (Phase::FirstMain, Step::Main)
            }
            (Phase::FirstMain, Step::Main) => (Phase::Combat, Step::CombatBegin),
            (Phase::SecondMain, Step::Main) => (Phase::Ending, Step::End),
            (_, Step::CombatBegin) => (Phase::Combat, Step::DeclareAttackers),
            (_, Step::DeclareAttackers) => (Phase::Combat, Step::DeclareBlockers),
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
                self.combat_declared = CombatDeclared::None;
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

    fn any_first_or_double_striker(&self) -> bool {
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

    fn untap_step(&mut self) {
        let active = self.state.turn.active;
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield).clone();
        for id in battlefield {
            let tapped = self
                .state
                .object(id)
                .is_some_and(|o| o.controller == active && o.status.contains(Status::TAPPED));
            if tapped {
                if let Some(obj) = self.state.object_mut(id) {
                    obj.status.remove(Status::TAPPED);
                }
                self.state.journal.record(GameEvent::ObjectUntapped {
                    object: id,
                    cause: Cause::TurnBased,
                });
            }
        }
        self.advance_step();
    }

    fn cleanup_step(&mut self) -> bool {
        // Clear damage (CR 514.2) and check hand size.
        for obj in self.state.arena.iter_mut_all() {
            obj.damage = 0;
        }
        let active = self.state.turn.active;
        let max_hand = 7i32 + i32::from(self.state.players[active.get() as usize].hand_modifier);
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

    fn end_cleanup(&mut self) {
        self.state.combat = crate::combat::CombatState::default();
        self.combat_declared = CombatDeclared::None;
        self.begin_turn(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn card_index(oracle_id: &str) -> CardIndex {
        baylee_cards::by_oracle_id(oracle_id)
            .expect("card exists")
            .index
    }

    fn forest() -> CardIndex {
        card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
    }

    fn creature() -> CardIndex {
        // Ondu Cleric — a 1/1 with no S2-relevant abilities.
        card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
    }

    fn preset_2p(seed: u64, deck: &[CardIndex]) -> GamePreset {
        let entries: Vec<DeckEntry> = deck
            .iter()
            .cycle()
            .take(60)
            .map(|c| DeckEntry {
                card: *c,
                print: PrintRef::new(0),
            })
            .collect();
        GamePreset {
            format: FormatId::Freeform,
            seed,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: (0..2)
                .map(|_| SeatSpec {
                    controller: SeatController::Ai(AIProfile::default()),
                    deck: entries.clone(),
                    starting_life: None,
                    starting_hand: None,
                    starting_battlefield: vec![],
                    emblems: vec![],
                    team: None,
                })
                .collect(),
        }
    }

    fn keep_all(engine: &mut Engine) {
        for _ in 0..2 {
            match engine.pending().clone() {
                Pending::Mulligan { player, .. } => {
                    engine.apply(player, PlayerAction::MulliganKeep).unwrap();
                }
                Pending::MulliganBottom { player, .. } => {
                    let hand: Vec<_> = engine
                        .state()
                        .zones
                        .list(ZoneLocation::Hand(player))
                        .clone();
                    engine
                        .apply(
                            player,
                            PlayerAction::ChooseObjects {
                                objects: hand[..1].to_vec(),
                            },
                        )
                        .unwrap();
                }
                other => panic!("expected mulligan, got {other:?}"),
            }
        }
    }

    fn pass_all(engine: &mut Engine) {
        for _ in 0..4 {
            match engine.pending().clone() {
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::ChooseAttackers { player } => {
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                        .unwrap();
                }
                Pending::ChooseBlockers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                        .unwrap();
                }
                _ => return,
            }
        }
    }

    #[test]
    fn full_turn_cycle_works() {
        let mut engine = Engine::new(&preset_2p(42, &[forest()]), &RegistryLookup).unwrap();
        keep_all(&mut engine);
        // Turn 1: untap → upkeep → draw (skipped for P1) → main.
        assert!(matches!(engine.pending(), Pending::Priority { player, .. } if player.get() == 0));
        pass_all(&mut engine);
        // After enough passing, turn 2 begins for player 2.
        let mut guard = 0;
        while !(engine.state().turn.number == 2 && engine.state().turn.active.get() == 1) {
            match engine.pending().clone() {
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::ChooseAttackers { player } => {
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                        .unwrap();
                }
                Pending::ChooseBlockers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                        .unwrap();
                }
                other => panic!("unexpected pending: {other:?}"),
            }
            guard += 1;
            assert!(guard < 50, "no progress: {:?}", engine.pending());
        }
        assert_eq!(engine.state().turn.number, 2);
    }

    #[test]
    fn determinism_through_engine() {
        let mut a = Engine::new(&preset_2p(7, &[forest()]), &RegistryLookup).unwrap();
        let mut b = Engine::new(&preset_2p(7, &[forest()]), &RegistryLookup).unwrap();
        keep_all(&mut a);
        keep_all(&mut b);
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
        for _ in 0..20 {
            let pending = a.pending().clone();
            let Pending::Priority { player, .. } = pending else {
                break;
            };
            a.apply(player, PlayerAction::PassPriority).unwrap();
            b.apply(player, PlayerAction::PassPriority).unwrap();
            assert_eq!(a.snapshot_hash(), b.snapshot_hash());
        }
    }

    #[test]
    fn land_play_and_mana_and_cast() {
        // Deck with forests + cheap creatures; scripted to find them.
        let mut engine =
            Engine::new(&preset_2p(11, &[forest(), creature()]), &RegistryLookup).unwrap();
        keep_all(&mut engine);
        // Player 1 main phase: play a land if possible, else pass.
        let p0 = PlayerId::new(0);
        let mut played_land = false;
        let mut cast_creature = false;
        for _ in 0..200 {
            let pending = engine.pending().clone();
            match pending {
                Pending::Priority { player, legal } => {
                    if player == p0 && !played_land && !legal.lands.is_empty() {
                        engine
                            .apply(
                                player,
                                PlayerAction::PlayLand {
                                    card: legal.lands[0],
                                },
                            )
                            .unwrap();
                        played_land = true;
                    } else if player == p0
                        && played_land
                        && !legal.mana_abilities.is_empty()
                        && !cast_creature
                    {
                        engine
                            .apply(
                                player,
                                PlayerAction::ActivateManaAbility {
                                    source: legal.mana_abilities[0],
                                },
                            )
                            .unwrap();
                    } else if player == p0
                        && played_land
                        && !legal.castable.is_empty()
                        && !cast_creature
                    {
                        let card = legal.castable[0];
                        let is_creature = engine
                            .state()
                            .object(card)
                            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE));
                        engine
                            .apply(player, PlayerAction::CastSpell { card })
                            .unwrap();
                        if is_creature {
                            cast_creature = true;
                        }
                    } else {
                        engine.apply(player, PlayerAction::PassPriority).unwrap();
                    }
                }
                Pending::GameOver(_) => break,
                _ => {
                    // combat declarations: declare nothing
                    break;
                }
            }
            if cast_creature {
                break;
            }
        }
        assert!(
            played_land,
            "expected to play a land; pending: {:?}",
            engine.pending()
        );
    }

    #[test]
    fn combat_kills_and_wins() {
        // Player 0 starts with 20 creatures on the battlefield via preset.
        let mut preset = preset_2p(3, &[forest()]);
        preset.seats[0].starting_battlefield = (0..20)
            .map(|_| DeckEntry {
                card: creature(),
                print: PrintRef::new(0),
            })
            .collect();
        preset.seats[1].starting_life = Some(5);
        let mut engine = Engine::new(&preset, &RegistryLookup).unwrap();
        keep_all(&mut engine);
        // Walk to combat and attack with everything.
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let mut guard = 0;
        loop {
            match engine.pending().clone() {
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::ChooseAttackers { player } => {
                    let attackers: Vec<(ObjectId, PlayerId)> = engine
                        .state()
                        .zones
                        .list(ZoneLocation::Battlefield)
                        .iter()
                        .copied()
                        .filter(|id| combat::can_attack(engine.state(), player, *id))
                        .map(|id| (id, p1))
                        .collect();
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers })
                        .unwrap();
                }
                Pending::ChooseBlockers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                        .unwrap();
                }
                Pending::GameOver(result) => {
                    assert_eq!(result.winner, Some(p0));
                    return;
                }
                other => panic!("unexpected pending: {other:?}"),
            }
            guard += 1;
            assert!(guard < 200, "game did not end");
        }
    }
}
