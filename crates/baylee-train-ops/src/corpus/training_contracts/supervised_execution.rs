//! Admission of supervised objective execution/continuation modes.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
pub(super) fn validate(input: &Value) -> Result<Value> {
    let contract = super::loss(&input["contract"])?;
    let reference = input["reference_execution"]
        .as_bool()
        .context("reference execution flag must be boolean")?;
    let continuation = input["init_state"]
        .as_bool()
        .context("init-state presence flag must be boolean")?;
    let legacy = contract["version"].as_f64() == Some(4.0);
    ensure!(
        !reference || legacy,
        "Loss5 requires optimized execution (ordinary or deferred), not --reference-execution"
    );
    ensure!(
        !continuation || legacy,
        "Loss5 package optimizer continuation is unsupported; use --init or strict --resume"
    );
    Ok(contract)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn loss_version_execution_matrix_is_native_and_fail_closed() -> Result<()> {
        for version in [4, 5] {
            for reference in [false, true] {
                for continuation in [false, true] {
                    let mut input = json!({"contract":{"version":version,"outcome_weighting":true,"normalization":"legacy_timestep","policy_head_weights":[1,1,1,1],"detach_value_input":false,"detach_aux_input":false,"value_weight":0.5,"auxiliary_weight":0.25},"reference_execution":reference,"init_state":continuation});
                    if version == 5 {
                        input["contract"]["categorical_weighting"] = json!("symmetric_v1");
                    }
                    ensure!(
                        validate(&input).is_ok() == (version == 4 || (!reference && !continuation)),
                        "execution admission mismatch"
                    );
                }
            }
        }
        ensure!(validate(&json!({"contract":{}})).is_err());
        Ok(())
    }
}
