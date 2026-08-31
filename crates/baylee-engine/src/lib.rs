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

/// Triggered abilities: collection and APNAP ordering.
pub mod trigger;

/// Evaluation of DSL data: filters, amounts, target options.
pub mod eval;

/// Effect resolution: the op interpreter with choice continuations.
pub mod resolve;

/// Characteristic projection: layers 1–7, dependency, cached recompute.
pub mod layers;

/// `ContinuousEffect`, `EffectTable`, modifiers, durations.
pub mod effects;

/// Endless-loop detection for decision-free segments (house rule).
pub mod loops;

// NOTE: the roadmap items (pipeline formalization, replacement registry,
// legality precompute, cost model, unusual casting, copy machinery, format
// modifiers, automation, dev mode, loop detection, scripted modifiers)
// live in docs/mechanics-roadmap.md. They deliberately have no empty
// module stubs here — a stub with a doc comment reads as shipped code.
