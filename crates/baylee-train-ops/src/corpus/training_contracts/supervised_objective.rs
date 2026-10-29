//! Saved objective compatibility, independent of tensor/optimizer restoration.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn legacy_defaults() -> Value {
    json!({"version":4,"outcome_weighting":true,"normalization":"legacy_timestep","policy_head_weights":[1.0,1.0,1.0,1.0],"detach_value_input":false,"detach_aux_input":false,"value_weight":0.5,"auxiliary_weight":0.25})
}

/// Loss4 historically allowed dataclass defaults in a partial saved config.
/// Loss5 always requires its complete explicit, unambiguous objective contract.
pub(super) fn saved_loss(raw: &Value) -> Result<Value> {
    let fields = raw
        .as_object()
        .context("saved loss contract must be an object")?;
    let mut normalized = legacy_defaults();
    let version = raw
        .get("version")
        .unwrap_or(&normalized["version"])
        .as_f64();
    if version == Some(5.0) {
        normalized["version"] = json!(5);
        normalized["categorical_weighting"] = json!("symmetric_v1");
        ensure!(
            fields.keys().collect::<BTreeSet<_>>()
                == normalized
                    .as_object()
                    .context("objective defaults")?
                    .keys()
                    .collect(),
            "Incomplete supervised loss5 configuration: explicit categorical_weighting required; use --init for a new learning experiment"
        );
    }
    for (key, value) in fields {
        ensure!(
            normalized.get(key).is_some(),
            "unknown supervised loss field {key}"
        );
        normalized[key] = value.clone();
    }
    super::loss(&normalized)?;
    // Numeric JSON spelling (1 versus1.0) never changes the objective.
    normalized["version"] = json!(if version == Some(5.0) { 5 } else { 4 });
    normalized["policy_head_weights"] = json!([1.0, 1.0, 1.0, 1.0]);
    normalized["value_weight"] = json!(0.5);
    normalized["auxiliary_weight"] = json!(0.25);
    Ok(normalized)
}

pub(super) fn resume(input: &Value) -> Result<Value> {
    let expected = saved_loss(&super::loss(&input["contract"])?)?;
    let saved = input["saved"]
        .as_object()
        .context("resume metadata must be an object")?;
    let selection = input["selection"]
        .as_str()
        .context("checkpoint selection missing")?;
    ensure!(
        matches!(selection, "policy" | "combined"),
        "unknown checkpoint selection"
    );
    match saved.get("supervised_loss_config").filter(|v| !v.is_null()) {
        Some(recorded) => ensure!(
            saved_loss(recorded)? == expected,
            "Resume supervised loss configuration differs"
        ),
        None => ensure!(
            expected["version"] == 4,
            "Explicit supervised loss configuration required for version5"
        ),
    }
    let saved_version = match saved.get("training_loss_version").and_then(Value::as_f64) {
        Some(4.0) => Some(4),
        Some(5.0) => Some(5),
        _ => None,
    };
    ensure!(
        saved_version == expected["version"].as_u64(),
        "Resume loss semantics differ; use --init for a new learning experiment"
    );
    ensure!(
        saved
            .get("checkpoint_selection")
            .unwrap_or(&json!("combined"))
            == selection,
        "Resume checkpoint selection differs from the saved scoring protocol"
    );
    Ok(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loss4_canonical_and_legacy_resume_stay_unchanged() -> Result<()> {
        let legacy = legacy_defaults();
        assert_eq!(super::super::loss(&legacy)?, legacy);
        assert_eq!(legacy.as_object().context("legacy")?.len(), 8);
        for recorded in [Value::Null, json!({"version":4}), legacy.clone()] {
            assert_eq!(
                resume(
                    &json!({"contract":legacy,"selection":"combined","saved":{"training_loss_version":4,"supervised_loss_config":recorded}})
                )?,
                legacy
            );
        }
        let mut changed = legacy;
        changed["categorical_weighting"] = json!("symmetric_v1");
        assert!(super::super::loss(&changed).is_err());
        Ok(())
    }

    #[test]
    fn loss5_requires_explicit_symmetric_marker_and_exact_resume() -> Result<()> {
        let mut config = legacy_defaults();
        config["version"] = json!(5);
        assert!(super::super::loss(&config).is_err());
        assert!(saved_loss(&config).is_err());
        config["categorical_weighting"] = json!("symmetric_v1");
        let input = json!({"contract":config,"selection":"policy","saved":{"training_loss_version":5,"checkpoint_selection":"policy","supervised_loss_config":config}});
        assert_eq!(resume(&input)?, config);
        for marker in [Value::Null, json!("legacy_no_cast"), json!(true)] {
            let mut bad = input.clone();
            bad["saved"]["supervised_loss_config"]["categorical_weighting"] = marker;
            assert!(resume(&bad).is_err());
        }
        for key in ["categorical_weighting", "detach_aux_input"] {
            let mut bad = input.clone();
            bad["saved"]["supervised_loss_config"]
                .as_object_mut()
                .context("saved")?
                .remove(key);
            assert!(resume(&bad).is_err());
        }
        let mut bad = input;
        bad["saved"]["checkpoint_selection"] = json!("combined");
        assert!(resume(&bad).is_err());
        Ok(())
    }
}
