//! Frozen shop catalogs. Adding a family never changes an existing offer stream.
use super::catalog::{Definition, Family};
use crate::Refusal;

const FIRST: [Family; 4] = [
    Family::Heartwood,
    Family::Satchel,
    Family::Compass,
    Family::Grove,
];

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
