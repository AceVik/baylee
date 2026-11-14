//! Strict, bounded comparison of an adapter's results with every reference case.
use super::{cases::Suite, execution::Variant, recipes::Output};
use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
};

const MAX_JSON_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(super) enum Outcome {
    Fusion(Option<Output>),
    Resale(u32),
    Execution(Option<Variant>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    pub id: String,
    pub result: Outcome,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observations {
    pub schema: u32,
    pub contract_revision: u32,
    /// Caller-supplied provenance, never interpreted as authenticated evidence.
    pub implementation: String,
    pub cases: Vec<Observation>,
}

pub(super) fn read(path: &Path) -> Result<Observations> {
    ensure!(
        path.metadata()?.is_file(),
        "observations must be a regular file"
    );
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_JSON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    decode(&bytes)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Observations> {
    ensure!(
        !bytes.is_empty() && bytes.len() <= MAX_JSON_BYTES,
        "observation size limit"
    );
    // Serde's errors may quote an unknown field/enum value supplied by the caller.
    serde_json::from_slice(bytes).map_err(|_| anyhow!("invalid observation JSON"))
}

pub(super) fn compare(suite: &Suite, observations: &Observations) -> Result<usize> {
    suite.check()?;
    super::schema(observations.schema, observations.contract_revision)?;
    ensure!(
        super::bounded_label(&observations.implementation),
        "invalid implementation label"
    );
    let expected: BTreeMap<_, _> = suite
        .fusions
        .iter()
        .map(|case| (case.id.as_str(), Outcome::Fusion(case.expected.clone())))
        .chain(
            suite
                .resales
                .iter()
                .map(|case| (case.id.as_str(), Outcome::Resale(case.expected))),
        )
        .chain(
            suite
                .executions
                .iter()
                .map(|case| (case.id.as_str(), Outcome::Execution(case.expected))),
        )
        .collect();
    ensure!(
        observations.cases.len() == expected.len(),
        "observation case count differs"
    );
    let mut seen = BTreeSet::new();
    for case in &observations.cases {
        ensure!(seen.insert(&case.id), "duplicate observation case");
        let Some(want) = expected.get(case.id.as_str()) else {
            anyhow::bail!("unknown observation case");
        };
        ensure!(
            case.result == *want,
            "observation result differs from reference"
        );
    }
    Ok(expected.len())
}
