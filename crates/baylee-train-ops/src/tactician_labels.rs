//! The tactical layer's answers kept as labels (`docs/trained-ai.md` §"The
//! tactician, stage S1") are read only where they were made for:
//!
//! - [`IMITATION_CONTRACT`] (`eval4 --tactician-imitate`, `--tactician-relabel`):
//!   the tactician's records, read only by its own trainer;
//! - [`DISTILL_CONTRACT`] (`eval4 --distill-labels`): the whole net's
//!   distillation labels beside the game records, read only by `convert4
//!   --bundle-distill` ([`admit_distillation`]).
//!
//! A run that keeps either names its contract in its `run.json`. Every reader
//! of a run that turns it into training data of another kind (convert4, the
//! research and self-imitation readers, the corpus indexes) calls
//! [`refuse_imitation`] on it first.

use anyhow::{Result, ensure};
use serde_json::Value;

/// The record and run contract of the tactician's imitation labels.
pub const IMITATION_CONTRACT: &str = "tactician_imitation_v1";

/// The run and dataset contract of the whole net's distillation labels.
pub const DISTILL_CONTRACT: &str = "bundle_distillation_v1";

/// Refuses a run whose `run.json` keeps the layer's answers as labels.
///
/// # Errors
/// When any value in `run`, at any depth, is [`IMITATION_CONTRACT`] or
/// [`DISTILL_CONTRACT`].
pub fn refuse_imitation(run: &Value) -> Result<()> {
    ensure!(
        !names(run, IMITATION_CONTRACT),
        "this run keeps {IMITATION_CONTRACT} records: the tactical layer's answers as labels, \
         which train the tactician and nothing else"
    );
    ensure!(
        !names(run, DISTILL_CONTRACT),
        "this run keeps {DISTILL_CONTRACT} labels: the tactical layer's answers, read only by \
         convert4 --bundle-distill"
    );
    Ok(())
}

/// Admits a run to `convert4 --bundle-distill`: its `run.json` must declare
/// the distillation contract (`distill.contract`). The tactician's imitation
/// contract it may also name is for its own records, elsewhere.
///
/// # Errors
/// A run that declares no [`DISTILL_CONTRACT`].
pub fn admit_distillation(run: &Value) -> Result<()> {
    ensure!(
        run["distill"]["contract"] == DISTILL_CONTRACT,
        "convert4 --bundle-distill reads only runs recorded with eval4 --distill-labels \
         ({DISTILL_CONTRACT})"
    );
    Ok(())
}

/// Whether any string in `value`, at any depth, is `contract`.
fn names(value: &Value, contract: &str) -> bool {
    match value {
        Value::String(text) => text == contract,
        Value::Array(items) => items.iter().any(|item| names(item, contract)),
        Value::Object(map) => map.values().any(|item| names(item, contract)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_run_naming_either_contract_anywhere_is_refused() {
        refuse_imitation(&json!({"tactician": {"record_contract": "tactician_records_v1"}}))
            .unwrap();
        for run in [
            json!({"labels": "tactician_imitation_v1"}),
            json!({"tactician": {"record_contract": "tactician_imitation_v1"}}),
            json!({"runs": [{"contracts": ["x", "tactician_imitation_v1"]}]}),
            json!({"distill": {"contract": "bundle_distillation_v1"}}),
        ] {
            assert!(refuse_imitation(&run).is_err());
        }
    }

    #[test]
    fn only_a_declared_distillation_run_is_admitted_to_its_mode() {
        admit_distillation(&json!({"distill": {"contract": "bundle_distillation_v1"},
            "labels": "tactician_imitation_v1"}))
        .unwrap();
        for run in [
            json!({"labels": "tactician_imitation_v1"}),
            json!({"distill": {"contract": "search_distillation_v1"}}),
            json!({}),
        ] {
            assert!(admit_distillation(&run).is_err());
        }
    }
}
