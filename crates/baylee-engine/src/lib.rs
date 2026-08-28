//! baylee-engine — the deterministic MTG rules kernel (no I/O, no async).
//!
//! Module layout is normative (see `docs/engine-internals.md`); modules are
//! implemented across M1 (state kernel, turn engine, ability runtime) and
//! M2 (layers, replacement chains, unusual casting, copy machinery).

#![warn(missing_docs)]

/// Dense, generational, O(1)-clone object storage.
pub mod arena;

/// `GameObject`, `ObjectKind`, `Characteristics`, counters, status.
pub mod object;

/// Zones and their ordered storage; library order IS the data.
pub mod zone;

/// `GameEvent`, event batches, and the journal.
pub mod event;

/// `GameState`: the complete, cloneable, hashable world.
pub mod state;

/// Phases, steps, and turn bookkeeping.
pub mod turn;

/// Seeded RNG (`ChaCha8`); every roll journaled.
pub mod rng;

/// The engine driver: turn/priority state machine (M1.S2).
pub mod engine;

/// The choice contract: `Pending` requests and `PlayerAction` answers.
pub mod choice;

/// Casting, land plays, intrinsic mana abilities, stack resolution.
pub mod casting;

/// Mana payment legality and auto-payment.
pub mod mana_pay;

/// Structured combat state machine.
pub mod combat;

/// State-based actions, CR 704 fixpoint.
pub mod sba;

/// Game results and win/lose evaluation.
pub mod win;

/// Propose → replacement rewrite → apply → collect triggers → journal (M1.S1).
pub mod pipeline {}

/// Replacement effect registry and application, CR 614–616 (M2.S6).
pub mod replacement {}

/// Triggered abilities: collection and APNAP ordering.
pub mod trigger;

/// Evaluation of DSL data: filters, amounts, target options.
pub mod eval;

/// Effect resolution: the op interpreter with choice continuations.
pub mod resolve;

/// Characteristic projection: layers 1–7, dependency, cached recompute (M2.S5).
pub mod layers {}

/// `ContinuousEffect`, `EffectTable`, modifiers, durations (M2.S5).
pub mod effects {}

/// Precomputed legal actions incl. filtered searches (M1.S3).
pub mod legality {}

/// `Cost`, `CostPart`, alternative/additional costs, cost modifiers (M1.S2).
pub mod cost {}

/// `CastPermission`, `PendingCast`, `DelayedTrigger`, `ExileRider` (M2.S7).
pub mod unusual {}

/// Copy objects/spells/tokens; copiable-values model (M2.S7).
pub mod copy {}

/// `FormatModifier` trait + commander/multiplayer modules (M2).
pub mod format {}

/// Server-side standing orders / reaction macros (M2+).
pub mod automation {}

/// Developer-mode commands through the same event pipeline (M1+).
pub mod dev {}

/// True endless-loop detection via snapshot-hash repetition (M1.S2).
pub mod loop_detect {}

/// Rhai `ScriptedModifier` for custom game modes (M2+).
pub mod script {}
