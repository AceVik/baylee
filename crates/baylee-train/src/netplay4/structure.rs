use crate::cardstructure::{Forest, digest, refs::RuleRef};
use std::{collections::BTreeMap, path::Path};

pub(super) fn load(path: &Path, expected: &str) -> anyhow::Result<BTreeMap<String, u32>> {
    let forest: Forest = serde_json::from_slice(&std::fs::read(path)?)?;
    anyhow::ensure!(
        forest.version == 1 && digest(&forest)? == expected,
        "card structure manifest mismatch"
    );
    // Rule edits must not silently reuse embeddings learned for previous rules.
    let current = crate::cardstructure::catalog(
        &baylee_cards::all()
            .map(|c| c.index.get())
            .collect::<Vec<_>>(),
    )?;
    anyhow::ensure!(
        digest(&current)? == expected,
        "compiled card rules differ from exported model; export card_structure4, refresh_structure4.py, then export_onnx4.py"
    );
    Ok(forest.roots)
}

pub(super) fn pack(
    rows: &[Vec<RuleRef>],
    count: usize,
    roots: &BTreeMap<String, u32>,
) -> anyhow::Result<(usize, Vec<i64>)> {
    anyhow::ensure!(
        rows.len() <= count,
        "rule reference rows would be truncated"
    );
    let width = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let mut values = vec![0; count * width * 2];
    for (i, refs) in rows.iter().enumerate() {
        for (j, r) in refs.iter().enumerate() {
            let node = roots
                .get(&r.key)
                .ok_or_else(|| anyhow::anyhow!("unrepresented rule: {}", r.key))?;
            anyhow::ensure!((1..=7).contains(&r.role), "invalid rule relation");
            values[(i * width + j) * 2] = i64::from(*node);
            values[(i * width + j) * 2 + 1] = i64::from(r.role);
        }
    }
    Ok((width, values))
}
