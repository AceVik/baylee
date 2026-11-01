//! Fail-closed provenance admission for policy-gradient datasets.
//!
//! Replaying a historical record does not repair its behavior probabilities.
//! Only explicitly versioned collectors can certify the resulting dataset.
use serde_json::Value;

pub(super) fn certified_run(run: &Value) -> bool {
    if run["policy_provenance_version"].as_u64() != Some(2)
        || !matches!(run["model_family"].as_str(), Some("v4" | "v5"))
    {
        return false;
    }
    // A searched run's behavior is the search, which has no probability:
    // its unsearched decisions were taken on trajectories the policy alone
    // would not have reached.
    if !run["search"].is_null() {
        return false;
    }
    match (run["policy_mode"].as_str(), run["policy_contract"].as_str()) {
        (Some("raw"), Some("raw_neural_sampler2_v1")) => run["temperature"]
            .as_f64()
            .is_some_and(|temperature| temperature == 0.0 || temperature > 0.35),
        (Some("tactical"), Some("tactical_house_demonstrations_v1")) => true,
        _ => false,
    }
}

pub(super) fn certified_dataset(sources: &[Value]) -> bool {
    !sources.is_empty() && sources.iter().all(certified_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn raw() -> Value {
        json!({"model_family":"v4", "policy_provenance_version":2,
            "policy_mode":"raw", "policy_contract":"raw_neural_sampler2_v1",
            "temperature":0.7})
    }

    #[test]
    fn historical_and_mixed_sources_cannot_certify_ppo() {
        let mut prior = raw();
        prior["policy_provenance_version"] = json!(1);
        assert!(
            !certified_run(&prior),
            "version 1 predates exact raw decoder validation"
        );
        assert!(!certified_dataset(&[]));
        assert!(!certified_run(&json!({"model_family":"v4"})));
        assert!(!certified_dataset(&[raw(), json!({"model_family":"v4"})]));
        assert!(certified_dataset(&[raw()]));
    }

    #[test]
    fn version_alone_cannot_certify_a_different_sampler() {
        let mut source = raw();
        source["temperature"] = json!(0.3);
        assert!(!certified_run(&source));
        source["temperature"] = json!(0.0);
        assert!(certified_run(&source));
        source["policy_contract"] = json!("unknown");
        assert!(!certified_run(&source));
    }

    #[test]
    fn tactical_demonstrations_are_certified_without_neural_probabilities() {
        let mut source = raw();
        source["policy_mode"] = json!("tactical");
        assert!(!certified_run(&source));
        source["policy_contract"] = json!("tactical_house_demonstrations_v1");
        assert!(certified_run(&source));
        source["model_family"] = json!("v3");
        assert!(!certified_run(&source));
    }

    #[test]
    fn a_searched_run_certifies_no_policy_gradient_dataset() {
        for mode in ["raw", "tactical"] {
            let mut source = raw();
            if mode == "tactical" {
                source["policy_mode"] = json!("tactical");
                source["policy_contract"] = json!("tactical_house_demonstrations_v1");
            }
            assert!(certified_run(&source));
            source["search"] = json!({"contract": "belief_lookahead_v1"});
            assert!(!certified_run(&source), "{mode}");
            source["search"] = Value::Null;
            assert!(certified_run(&source), "an explicit null names no search");
        }
    }
}
