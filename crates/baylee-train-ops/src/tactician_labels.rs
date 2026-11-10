//! The tactical layer's answers kept as labels for the tactician
//! (`eval4 --tactician-imitate`, `docs/trained-ai.md` §"The tactician, stage
//! S1") teach the tactician and nothing else.
//!
//! A run that keeps them names [`IMITATION_CONTRACT`] in its `run.json`: the
//! record directory's own, and `--record-out`'s when the games are recorded
//! too. Every reader of a run that turns it into training data of another
//! kind (convert4, the research and self-imitation readers, the corpus
//! indexes) calls [`refuse_imitation`] on it first.

use anyhow::{Result, ensure};
use serde_json::Value;

/// The record and run contract of an imitating collection.
pub const IMITATION_CONTRACT: &str = "tactician_imitation_v1";

/// Refuses a run whose `run.json` keeps imitation labels.
///
/// # Errors
/// When any value in `run`, at any depth, is [`IMITATION_CONTRACT`].
pub fn refuse_imitation(run: &Value) -> Result<()> {
    ensure!(
        !names_imitation(run),
        "this run keeps {IMITATION_CONTRACT} records: the tactical layer's answers as labels, \
         which train the tactician and nothing else"
    );
    Ok(())
}

/// Whether any string in `value`, at any depth, is [`IMITATION_CONTRACT`].
fn names_imitation(value: &Value) -> bool {
    match value {
        Value::String(text) => text == IMITATION_CONTRACT,
        Value::Array(items) => items.iter().any(names_imitation),
        Value::Object(map) => map.values().any(names_imitation),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_run_naming_the_contract_anywhere_is_refused() {
        refuse_imitation(&json!({"tactician": {"record_contract": "tactician_records_v1"}}))
            .unwrap();
        for run in [
            json!({"labels": "tactician_imitation_v1"}),
            json!({"tactician": {"record_contract": "tactician_imitation_v1"}}),
            json!({"runs": [{"contracts": ["x", "tactician_imitation_v1"]}]}),
        ] {
            assert!(refuse_imitation(&run).is_err());
        }
    }
}
