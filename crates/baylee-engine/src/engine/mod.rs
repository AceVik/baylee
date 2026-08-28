//! The engine driver: the complete turn/priority state machine.
//!
//! Public contract (see `docs/engine-internals.md`): the game only advances
//! through [`Engine::apply`] answering [`Engine::pending`]. Everything else
//! — SBAs, stack resolution, turn-based actions — is automatic.

use crate::casting::{self, CastFailure};
use crate::choice::{LegalActions, Pending, PlayerAction};
use crate::combat::{self, AttackerInfo, BlockerInfo};
use crate::eval;
use crate::event::{Cause, GameEvent, LossReason};
use crate::mana_pay;
use crate::object::{AbilityLoc, GameObject, ObjectKind, Status};
use crate::resolve::{self, Resolution};
use crate::sba;
use crate::state::{CardLookup, GameState, SetupError, StateError};
use crate::trigger;
use crate::turn::{Phase, Step};
use crate::win::{EndReason, GameResult};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{AbilityDef, ActivationTiming, Cost, CostPart};
use baylee_core::ids::{NameRef, ObjectId, PlayerId};
use baylee_core::preset::{GamePreset, HouseRules};
use baylee_core::types::TypeSet;
use smallvec::SmallVec;
use std::collections::VecDeque;

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
pub struct Engine<L: CardLookup> {
    lookup: L,
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
    /// A suspended effect resolution (choice continuation).
    resolution: Option<Resolution>,
    /// Journal sequence number up to which triggers were collected.
    trigger_scan_seq: u64,
    /// A cast/activation waiting for its target choice.
    pending_plan: Option<PlanKind>,
    /// Triggers collected but not yet stacked (target choices first).
    trigger_queue: VecDeque<trigger::PendingTrigger>,
}

/// What a `Pending::ChooseTargets` is targeting for.
#[derive(Clone, Copy, Debug)]
enum PlanKind {
    /// Casting a spell.
    CastSpell {
        /// The card being cast.
        card: ObjectId,
    },
    /// Activating an ability.
    ActivateAbility {
        /// Source permanent.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
    },
    /// Putting a triggered ability on the stack.
    Trigger {
        /// Source permanent.
        source: ObjectId,
        /// Ability index.
        ability_index: u32,
    },
}

impl<L: CardLookup> Engine<L> {
    /// Creates a game from a preset. The first pending request is the
    /// first seat's mulligan decision.
    ///
    /// # Errors
    /// [`EngineError::Setup`] for invalid presets or unknown cards.
    pub fn new(preset: &GamePreset, lookup: L) -> Result<Self, EngineError> {
        let state = GameState::from_preset(preset, &lookup)?;
        let trigger_scan_seq = state.journal.last_seq();
        let mut engine = Self {
            lookup,
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
            resolution: None,
            trigger_scan_seq,
            pending_plan: None,
            trigger_queue: VecDeque::new(),
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

    /// Determinism hash (state + suspended resolution + machine fields).
    #[must_use]
    pub fn snapshot_hash(&self) -> u64 {
        let base = self.state.snapshot_hash();
        let mut extra = self.trigger_scan_seq;
        if let Some(r) = &self.resolution {
            extra = extra
                .wrapping_mul(31)
                .wrapping_add(r.pc as u64)
                .wrapping_add(u64::from(r.on_stack.slot()))
                .wrapping_add(u64::from(r.controller.get()));
        }
        extra = extra.wrapping_mul(31).wrapping_add(u64::from(self.passes));
        base ^ extra.rotate_left(17)
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
}

mod abilities;
mod actions;
mod progress;

#[cfg(test)]
mod s3_tests;
#[cfg(test)]
mod tests;
