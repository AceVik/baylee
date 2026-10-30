//! Native semantic encoder and recurrent training schedule contracts.
mod supervised_execution;
mod supervised_objective;
use super::bc_metadata::digest;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

fn encoder(value: &Value) -> Result<Value> {
    let version = value["version"].as_u64();
    ensure!(
        matches!(version, Some(7..=9)),
        "Unsupported encoder version"
    );
    ensure!(
        !matches!(version, Some(8 | 9))
            || (value["model_version"] == 5 && value["relational_revision"] == 2),
        "Encoder8/9 requires model5 relational revision2"
    );
    let width = value.get("global_feat_width").unwrap_or(&Value::Null);
    ensure!(
        if version == Some(9) {
            width == 82 && matches!(value["candidate_context_revision"].as_u64(), Some(1 | 2))
        } else {
            width.is_null() || width == 80
        },
        "Encoder/global feature width mismatch"
    );
    Ok(value["version"].clone())
}
fn initial(value: &Value) -> Result<Value> {
    let source = &value["source"];
    let target = &value["target"];
    if value["explicit_to9"] == true {
        ensure!(
            matches!(source.as_u64(), Some(7 | 8))
                && target == 9
                && value["explicit_from7"] != true,
            "Encoder9 warmstart requires explicit 7/8-to-9 weights-only transition"
        );
        return Ok(
            json!({"source_encoder_version":source,"target_encoder_version":9,"kind":"new_experiment_weights_only","new_global_columns":"zero_initialized_trainable","behavior_equivalence_claimed":false}),
        );
    }
    if value["explicit_from7"] == true {
        ensure!(
            source == 7 && target == 8,
            "Explicit Encoder7 warmstart requires a 7-to-8 transition"
        );
        return Ok(
            json!({"source_encoder_version":7,"target_encoder_version":8,"kind":"new_experiment_weights_only"}),
        );
    }
    if target == 7 && (source == 6 || source == 7) {
        return Ok(Value::Null);
    }
    ensure!(
        source == target,
        "Encoder mismatch: new Encoder8 experiment requires explicit --encoder-warmstart-from7"
    );
    Ok(Value::Null)
}
fn depth(value: &Value) -> Result<Value> {
    let mut contract = value.clone();
    let fields = contract
        .as_object_mut()
        .context("invalid schedule contract")?;
    fields.remove("sha256");
    ensure!(
        contract["deterministic_algorithms"] == true,
        "Exact resume requires deterministic algorithms"
    );
    let mode = contract["mode"].as_str();
    ensure!(
        contract["version"] == 1 && matches!(mode, Some("fixed" | "legacy_frame_cycle")),
        "Unsupported depth schedule version/mode"
    );
    ensure!(
        contract["phase_rule"] == "optimizer_step_plus_frame_mod_period",
        "Unsupported depth schedule phase rule"
    );
    let depths = contract["positive_k"]
        .as_array()
        .context("Decision depths must be positive K1..8; K0 is observation-only")?;
    ensure!(
        !depths.is_empty()
            && depths
                .iter()
                .all(|v| v.as_u64().is_some_and(|k| (1..=8).contains(&k))),
        "Decision depths must be positive K1..8; K0 is observation-only"
    );
    ensure!(
        mode != Some("fixed") || depths.len() == 1,
        "Fixed depth needs exactly one K"
    );
    ensure!(
        mode != Some("legacy_frame_cycle") || contract["positive_k"] == json!([1, 2, 4, 8]),
        "Legacy frame cycle is exactly K1/2/4/8"
    );
    ensure!(
        contract["validation_history"] == "complete_independent_reset_per_k",
        "Validation must reset independent complete histories for every K"
    );
    // Every reported depth costs one complete validation pass; a run may report
    // fewer, but always the K2 its snapshots are selected by.
    let reported = contract["report_k"]
        .as_array()
        .and_then(|depths| depths.iter().map(Value::as_u64).collect::<Option<Vec<_>>>());
    ensure!(
        contract["selection_k"] == 2
            && reported.is_some_and(|depths| {
                depths.contains(&2)
                    && depths.windows(2).all(|pair| pair[0] < pair[1])
                    && depths.iter().all(|k| [1, 2, 4, 8].contains(k))
            }),
        "Validation contract requires selection K2 and reports an increasing subset of K1/2/4/8 with K2"
    );
    contract["sha256"] = json!(digest(&contract)?);
    Ok(contract)
}
/// Validate a semantic contract passed by a thin tensor-training adapter.
///
/// # Errors
/// Rejects unsupported encoders, incompatible warmstarts and changed resume schedules.
pub fn validate(value: &Value) -> Result<Value> {
    match value["kind"].as_str() {
        Some("encoder") => encoder(value),
        Some("initial_encoder") => initial(value),
        Some("depth") => depth(&value["contract"]),
        Some("supervised_loss") => loss(&value["contract"]),
        Some("supervised_saved_loss") => supervised_objective::saved_loss(&value["contract"]),
        Some("supervised_resume") => supervised_objective::resume(value),
        Some("supervised_execution") => supervised_execution::validate(value),
        Some("depth_resume") => {
            let expected = depth(&value["contract"])?;
            ensure!(
                value["saved"]["depth_schedule"] == expected,
                "Resume depth/validation schedule differs or is missing; use --init for a new weights-only continuation"
            );
            Ok(expected)
        }
        _ => bail!("unsupported training contract"),
    }
}
fn loss(contract: &Value) -> Result<Value> {
    let version = contract["version"].as_f64();
    ensure!(
        matches!(version, Some(4.0 | 5.0)) && contract["outcome_weighting"].is_boolean(),
        "Invalid supervised loss version/outcome flag"
    );
    ensure!(
        if version == Some(5.0) {
            contract["categorical_weighting"] == "symmetric_v1"
        } else {
            contract.get("categorical_weighting").is_none()
        },
        "Loss5 requires explicit categorical_weighting=symmetric_v1; old loss5 is ambiguous, use --init for a new learning experiment. Loss4 contract stays unchanged"
    );
    ensure!(
        contract["detach_value_input"].is_boolean() && contract["detach_aux_input"].is_boolean(),
        "Detachment flags must be boolean"
    );
    let normalization = contract["normalization"].as_str();
    ensure!(
        matches!(normalization, Some("legacy_timestep" | "unroll_per_head")),
        "Unsupported supervised normalization"
    );
    ensure!(
        contract["policy_head_weights"].as_array().is_some_and(
            |weights| weights.len() == 4 && weights.iter().all(|w| w.as_f64() == Some(1.0))
        ) && contract["value_weight"].as_f64() == Some(0.5)
            && contract["auxiliary_weight"].as_f64() == Some(0.25),
        "This ablation fixes policy/value/aux weights to1/.5/.25"
    );
    ensure!(
        version != Some(4.0)
            || (contract["outcome_weighting"] == true
                && normalization == Some("legacy_timestep")
                && contract["detach_value_input"] == false
                && contract["detach_aux_input"] == false),
        "Loss4 cannot enable new ablations"
    );
    Ok(contract.clone())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoder9_requires_explicit_width_architecture_and_transfer() -> Result<()> {
        let good = json!({"kind":"encoder","version":9,"model_version":5,"relational_revision":2,"candidate_context_revision":2,"global_feat_width":82});
        assert_eq!(validate(&good)?, 9);
        for (key, value) in [
            ("global_feat_width", json!(80)),
            ("global_feat_width", Value::Null),
            ("model_version", json!(3)),
            ("relational_revision", json!(1)),
            ("candidate_context_revision", json!(3)),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(validate(&bad).is_err());
        }
        for source in [7, 8] {
            assert!(initial(&json!({"source":source,"target":9})).is_err());
            assert_eq!(
                initial(&json!({"source":source,"target":9,"explicit_to9":true}))?["target_encoder_version"],
                9
            );
        }
        Ok(())
    }
    #[test]
    fn schedule_resume_and_loss_ablation_guards_are_native() -> Result<()> {
        let schedule = json!({"version":1,"mode":"fixed","positive_k":[2],"phase_rule":"optimizer_step_plus_frame_mod_period","selection_k":2,"report_k":[1,2,4,8],"validation_history":"complete_independent_reset_per_k","deterministic_algorithms":true});
        let sealed = validate(&json!({"kind":"depth","contract":schedule}))?;
        assert_eq!(sealed["sha256"], digest(&schedule)?);
        validate(
            &json!({"kind":"depth_resume","contract":schedule,"saved":{"depth_schedule":sealed}}),
        )?;
        let mut bad = schedule.clone();
        bad["positive_k"] = json!([true]);
        assert!(depth(&bad).is_err());
        let mut fewer = schedule.clone();
        fewer["report_k"] = json!([2]);
        assert_ne!(depth(&fewer)?["sha256"], sealed["sha256"]);
        for report in [
            json!([1, 4]),
            json!([2, 1]),
            json!([2, 3]),
            json!([]),
            json!([2, 2]),
        ] {
            let mut bad = schedule.clone();
            bad["report_k"] = report;
            assert!(depth(&bad).is_err());
        }
        assert!(validate(&json!({"kind":"depth_resume","contract":schedule,"saved":{}})).is_err());
        let mut config = json!({"version":4,"outcome_weighting":true,"normalization":"legacy_timestep","policy_head_weights":[1.0,1.0,1.0,1.0],"detach_value_input":false,"detach_aux_input":false,"value_weight":0.5,"auxiliary_weight":0.25});
        loss(&config)?;
        config["detach_value_input"] = json!(true);
        assert!(loss(&config).is_err());
        config["version"] = json!(5);
        config["categorical_weighting"] = json!("symmetric_v1");
        loss(&config)?;
        config["outcome_weighting"] = json!(1);
        assert!(loss(&config).is_err());
        Ok(())
    }
    #[test]
    fn encoder_transition_is_explicit_and_booleans_are_not_versions() -> Result<()> {
        assert!(validate(&json!({"kind":"encoder","version":true})).is_err());
        assert!(
            validate(
                &json!({"kind":"encoder","version":8,"model_version":5,"relational_revision":1})
            )
            .is_err()
        );
        assert_eq!(
            validate(
                &json!({"kind":"encoder","version":8,"model_version":5,"relational_revision":2})
            )?,
            8
        );
        assert!(
            validate(
                &json!({"kind":"initial_encoder","source":7,"target":8,"explicit_from7":false})
            )
            .is_err()
        );
        assert_eq!(
            validate(
                &json!({"kind":"initial_encoder","source":7,"target":8,"explicit_from7":true})
            )?["kind"],
            "new_experiment_weights_only"
        );
        Ok(())
    }
}
