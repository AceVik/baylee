//! Frozen shop catalogs. Adding a family never changes an existing offer stream.
use super::catalog::{Definition, Family};
use crate::Refusal;
use baylee_core::preset::{AIProfile, HoldUp, Politics, Sight};

const FIRST: [Family; 4] = [
    Family::Heartwood,
    Family::Satchel,
    Family::Compass,
    Family::Grove,
];

const LIFE_WORKSHOP: [Family; 8] = [
    Family::Heartwood,
    Family::Satchel,
    Family::Compass,
    Family::Grove,
    Family::SpringChalice,
    Family::Hearthstone,
    Family::BloodAmber,
    Family::LifeCoil,
];

/// Rules and policy frozen alongside an offer stream, never read from live defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contract {
    /// Compiled native pack/economy revision.
    pub revision: u16,
    /// Deterministic house-policy generation.
    pub ai_generation: u16,
}

impl Contract {
    /// Expert's Tower settings with the saved policy generation.
    pub const fn expert(self) -> AIProfile {
        AIProfile {
            lookahead: 2,
            temperature_milli: 12,
            mulligan_skill: 2,
            politics: Politics::AttackLeader,
            hold_up: HoldUp::ThreatAware,
            sight: Sight::Fair,
            generation: self.ai_generation,
            foresight: 0,
        }
    }
}

/// Resolve an immutable generation without silently upgrading its executor.
/// # Errors
/// Unknown generations have no execution contract.
pub const fn contract(generation: u16) -> Result<Contract, Refusal> {
    match generation {
        1 => Ok(Contract {
            revision: 2,
            ai_generation: 19,
        }),
        2 => Ok(Contract {
            revision: 3,
            ai_generation: 19,
        }),
        _ => Err(Refusal::Invalid),
    }
}

/// The original stored shops predate an explicit generation field.
pub const fn original() -> u16 {
    1
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's skip_serializing_if requires a reference
pub(crate) fn is_original(generation: &u16) -> bool {
    *generation == original()
}

/// Exact equipment pool available to this immutable shop contract.
/// # Errors
/// An unknown generation is never upgraded or downgraded implicitly.
pub fn families(generation: u16) -> Result<&'static [Family], Refusal> {
    match generation {
        1 => Ok(&FIRST),
        2 => Ok(&LIFE_WORKSHOP),
        _ => Err(Refusal::Invalid),
    }
}

/// Check a frozen definition against the run's catalog generation.
/// # Errors
/// Unknown generation or a family introduced after this run began.
pub fn definition(generation: u16, definition: Definition) -> Result<(), Refusal> {
    if families(generation)?.contains(&definition.family) {
        Ok(())
    } else {
        Err(Refusal::Invalid)
    }
}
