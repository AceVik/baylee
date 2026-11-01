//! Validate the shared acceptance holds against repository source.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context;
pub(crate) use baylee_cards_codegen::coverage_holds::CoverageHolds;

/// Read and validate holds against current source, before codegen writes anything.
pub(crate) fn load(root: &Path) -> anyhow::Result<CoverageHolds> {
    load_completing_stubs(root, &BTreeSet::new())
}

/// Include only explicitly selected stubs that must fully render before any write.
/// Their existing holds keep canonical tests unaccepted during that transition.
pub(crate) fn load_completing_stubs(
    root: &Path,
    completing: &BTreeSet<String>,
) -> anyhow::Result<CoverageHolds> {
    let path = root.join("data/card-test-holds.tsv");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let holds = CoverageHolds::parse(&text)?;
    let cards = root.join("crates/baylee-cards/src/cards");
    let mut eligible = completing.clone();
    for path in crate::card_files(&cards)?.values() {
        let source = std::fs::read_to_string(path)?;
        let Some(coverage) = crate::files::knob(&source, "coverage") else {
            continue;
        };
        if crate::stubgen::is_machine_owned(&source)
            && (coverage.starts_with("Coverage::Generated")
                || coverage.starts_with("Coverage::Implemented"))
            && let Some(value) = crate::files::knob(&source, "oracle_id")
            && let Some(id) = crate::files::quoted_value(value, "\"")
        {
            eligible.insert(id);
        }
    }
    holds.validate_ids(&eligible)?;
    Ok(holds)
}
