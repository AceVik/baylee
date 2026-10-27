// The collector and converter must agree on these exact producer fields.
fn collector_policy_metadata(
    mode: PolicyMode4,
    temperature: f32,
    thinking_steps: u8,
    family: &str,
) -> Value {
    json!({"policy_mode":mode,"policy_contract":mode.contract(),"policy_provenance_version":2,
        "model_family":family,"temperature":temperature,"thinking_steps":thinking_steps,"sampling_version":2})
}

#[cfg(test)]
#[path = "../convert4/policy_provenance.rs"]
mod converter_policy_contract;

#[cfg(test)]
mod producer_tests {
    use super::*;
    #[test]
    fn actual_producer_fields_are_accepted_by_converter() {
        for mode in [PolicyMode4::Raw, PolicyMode4::Tactical] {
            let source = collector_policy_metadata(mode, 0.8, 2, "v5");
            assert!(converter_policy_contract::certified_run(&source));
            assert_eq!(source["temperature"], json!(0.8_f32));
            assert_eq!(source["thinking_steps"], 2);
            assert_eq!(source["sampling_version"], 2);
        }
        let invalid = collector_policy_metadata(PolicyMode4::Raw, 0.2, 2, "v5");
        assert!(!converter_policy_contract::certified_run(&invalid));
    }
}
