//! Versioned value-head meaning, independent of the policy tensor ABI.
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const RESEARCH_OBJECTIVE: &str = "failure_terminal_utility_minus_frozen_source_value_uncentered_v1";
const RESEARCH_TARGET: &str = "failure_terminal_research_v1";

/// What the exported value head estimates. Neither variant asserts calibration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CriticSemantics4 {
    /// Side win/draw utility, compatible with the live draw rule's outcome scale.
    #[serde(rename = "mtg_side_outcome_utility_v1")]
    MtgSideOutcomeUtilityV1 {},
    /// Separate utility that also assigns targets to learner failures.
    #[serde(rename = "failure_terminal_research_utility_v1")]
    FailureTerminalResearchUtilityV1 {
        /// Exact supported learning objective; validated on metadata ingestion.
        objective: String,
        /// Exact failure/unknown/outcome classification contract.
        target_contract: String,
    },
}

pub(super) fn read_semantics(meta: &Value) -> anyhow::Result<Option<CriticSemantics4>> {
    let Some(value) = meta.get("critic_semantics") else {
        anyhow::ensure!(
            meta.get("research_ppo_contract").is_none()
                && meta.get("research_ppo_progress").is_none(),
            "research model metadata requires explicit critic_semantics"
        );
        // Preserve archival identities byte-for-byte. Absence is legacy provenance,
        // not evidence that an arbitrary checkpoint was trained on MTG outcomes.
        return Ok(None);
    };
    let semantics: CriticSemantics4 =
        serde_json::from_value(value.clone()).context("invalid critic_semantics")?;
    match &semantics {
        CriticSemantics4::MtgSideOutcomeUtilityV1 {} => anyhow::ensure!(
            meta.get("research_ppo_contract").is_none()
                && meta.get("research_ppo_progress").is_none(),
            "MTG outcome critic conflicts with research checkpoint metadata"
        ),
        CriticSemantics4::FailureTerminalResearchUtilityV1 {
            objective,
            target_contract,
        } => {
            anyhow::ensure!(
                objective == RESEARCH_OBJECTIVE && target_contract == RESEARCH_TARGET,
                "unsupported research critic objective or target contract"
            );
            if let Some(contract) = meta.get("research_ppo_contract") {
                anyhow::ensure!(
                    contract["version"] == 1
                        && contract["objective"] == *objective
                        && contract["target_contract"] == *target_contract,
                    "research critic conflicts with checkpoint metadata"
                );
            }
        }
    }
    Ok(Some(semantics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn research() -> Value {
        json!({"kind":"failure_terminal_research_utility_v1",
            "objective": RESEARCH_OBJECTIVE, "target_contract": RESEARCH_TARGET})
    }

    #[test]
    fn explicit_semantics_roundtrip_and_legacy_absence_remains_absent() {
        assert_eq!(read_semantics(&json!({})).unwrap(), None);
        for marker in [json!({"kind":"mtg_side_outcome_utility_v1"}), research()] {
            let actual = read_semantics(&json!({"critic_semantics":marker})).unwrap();
            assert_eq!(serde_json::to_value(actual.unwrap()).unwrap(), marker);
        }
    }

    #[test]
    fn unknown_null_incomplete_and_conflicting_semantics_are_rejected() {
        let mut wrong_objective = research();
        wrong_objective["objective"] = json!("unknown");
        let mut wrong_target = research();
        wrong_target["target_contract"] = json!("ordinary_outcome");
        for marker in [
            Value::Null,
            json!("mtg_side_outcome_utility_v1"),
            json!({"kind":"unknown"}),
            json!({"kind":"failure_terminal_research_utility_v1"}),
            json!({"kind":"mtg_side_outcome_utility_v1", "objective":RESEARCH_OBJECTIVE}),
            wrong_objective,
            wrong_target,
        ] {
            assert!(
                read_semantics(&json!({"critic_semantics":marker})).is_err(),
                "{marker}"
            );
        }
        for marker in [None, Some(json!({"kind":"mtg_side_outcome_utility_v1"}))] {
            let mut meta = json!({"research_ppo_contract":null});
            if let Some(marker) = marker {
                meta["critic_semantics"] = marker;
            }
            assert!(read_semantics(&meta).is_err());
        }
        assert!(
            read_semantics(&json!({"critic_semantics":research(),
            "research_ppo_contract":{"version":1,"objective":"wrong",
                "target_contract":RESEARCH_TARGET}}))
            .is_err()
        );
    }
}
