//! Shared research metadata only; engine termination proofs live in baylee-train.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Immutable native research target descriptor.
pub fn contract() -> Value {
    json!({"name":"failure_terminal_research_v1","predicate":"repeated_view_two_replayed_announcement_rollbacks_v1",
        "utility_mapping":"natural_1_0_half_learner_loop_0_unknown_null_v1",
        "bootstrap":false,"fallback_recovery_actions_executed":0,
        "admission":"unadmitted_native_collection_only"})
}

/// Structural framing/preprocessing identity. Old BST4 caches cannot consume it.
pub const FRAME_CONTRACT: &str = "failure_terminal_research_observations_v1";

/// Separate research return. Unknown attempts have no return or MTG outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchReturn {
    /// Versioned research ending, never an implicit engine result.
    pub termination_kind: String,
    /// Declared learner utility, absent for unknown attempts.
    pub training_utility: Option<f32>,
    /// Actual natural outcome only; failure terminals retain null.
    pub mtg_outcome: Option<Value>,
    /// Version 1 uses no bootstrapping.
    pub bootstrap: bool,
}
