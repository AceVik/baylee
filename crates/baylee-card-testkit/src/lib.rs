//! What every card test crate shares: the test kit, the doors' vocabulary and the
//! kits the card tests lean on. `docs/cards.md` says where a test goes.
#![cfg(feature = "enabled")]
#![allow(clippy::wildcard_imports, unused_imports, dead_code, missing_docs)]
// Test helpers: a panic is how they fail, and nothing outside the tests calls them.
#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::return_self_not_must_use
)]

// The tests look cards up, so their binary holds the pool.
#[cfg(test)]
use baylee_cards_pool as _;

pub use baylee_cards_dsl::{AbilityDef, ActivationTiming, Cost, CostPart};
pub use baylee_core::ids::{NameRef, ObjectId, PlayerId};
pub use baylee_core::preset::{GamePreset, HouseRules};
pub use baylee_core::types::TypeSet;
pub use baylee_engine::casting::{self, CastFailure};
pub use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
pub use baylee_engine::combat::{self, AttackerInfo};
pub use baylee_engine::engine::decision::DecisionContext;
pub use baylee_engine::engine::progress::QueuedTrigger;
pub use baylee_engine::engine::*;
pub use baylee_engine::eval;
pub use baylee_engine::event::{Cause, GameEvent};
pub use baylee_engine::object::{AbilityLoc, GameObject, ObjectKind, Status};
pub use baylee_engine::resolve::{self, Resolution};
pub use baylee_engine::sba;
pub use baylee_engine::state::{CardLookup, GameState, SetupError, StateError};
pub use baylee_engine::trigger;
pub use baylee_engine::turn::{Phase, Step};
pub use baylee_engine::win::{EndReason, GameResult};
pub use baylee_engine::zone::{Zone, ZoneLocation, ZonePosition};
pub use smallvec::SmallVec;
pub use std::collections::VecDeque;

#[path = "../../baylee-engine/src/engine/card_tests/mod.rs"]
pub mod card_tests;
#[path = "../../baylee-engine/src/engine/synthetic.rs"]
pub mod synthetic;
#[path = "../../baylee-engine/src/engine/testkit/mod.rs"]
pub mod testkit;
