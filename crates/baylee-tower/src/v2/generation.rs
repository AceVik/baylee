//! Frozen shop catalogs. Adding a family never changes an existing offer stream.
use super::catalog::{Definition, Family};
use crate::Refusal;
use baylee_core::preset::AIProfile;

/// Independent equipment, six active slots and the Commander run shop.
/// Registered separately while its native executor is integrated.
pub const RUN: u16 = 4;

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

const TOKEN_WORKSHOP: [Family; 12] = [
    Family::Heartwood,
    Family::Satchel,
    Family::Compass,
    Family::Grove,
    Family::SpringChalice,
    Family::Hearthstone,
    Family::BloodAmber,
    Family::LifeCoil,
    Family::SeedCradle,
    Family::SwarmBanner,
    Family::HarvestWheel,
    Family::RootReservoir,
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
            generation: self.ai_generation,
            ..AIProfile::TOWER_EXPERT
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
            ai_generation: AIProfile::TOWER_EXPERT.generation,
        }),
        2 => Ok(Contract {
            revision: 3,
            ai_generation: AIProfile::PAY_LIFE_ACTIVATION_MANA,
        }),
        3 => Ok(Contract {
            revision: 4,
            ai_generation: AIProfile::OPTIONAL_TOKEN_SATURATION,
        }),
        RUN => Ok(Contract {
            revision: 5,
            ai_generation: AIProfile::OPTIONAL_TOKEN_SATURATION,
        }),
        _ => Err(Refusal::Invalid),
    }
}

/// Native rules and house policy of the compiled release.
pub const CURRENT: Contract = match contract(super::GENERATOR) {
    Ok(contract) => contract,
    Err(_) => panic!("unknown current Tower generation"),
};

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
        3 | RUN => Ok(&TOKEN_WORKSHOP),
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
