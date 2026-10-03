//! Exact, mandatory activation choices made while an instruction resolves.
use baylee_core::ids::DamageSourceRef;

/// Identity of one step in a resolving mana instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ManaChoiceId {
    /// The resolving spell or ability incarnation.
    pub source: DamageSourceRef,
    /// Accepted activations so far; old answers cannot select a later step.
    pub step: u64,
}

/// One legal mana ability of a still-present source incarnation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ManaAbilityChoice {
    /// Exact source being activated.
    pub source: DamageSourceRef,
    /// Printed/granted ability index, or `None` for intrinsic basic-land mana.
    pub ability_index: Option<u32>,
}
