//! Export the public typed rules independently of recorded matches, or check
//! whether this build's catalog is a superset of the one a model was
//! exported with (`docs/trained-ai.md` §"Card catalog compatibility").
use baylee_train::cardstructure::{Forest, catalog, compat, digest};
use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    /// Write this build's catalog here.
    #[arg(long, required_unless_present = "check_model")]
    out: Option<PathBuf>,
    /// Compare a model's `card_structure.json` (or the model directory
    /// holding it) with this build's catalog, print the comparison, and fail
    /// unless every rule the model knows is unchanged (the strict superset;
    /// what drift would withdraw is printed either way).
    #[arg(long, conflicts_with = "out")]
    check_model: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let cards = baylee_cards::all()
        .map(|c| c.index.get())
        .collect::<Vec<_>>();
    if let Some(out) = &args.out {
        anyhow::ensure!(!out.exists(), "output already exists");
        let forest = catalog(&cards)?;
        fs::write(out, serde_json::to_vec(&forest)?)?;
        println!(
            "cards={} nodes={} roots={} sha256={}",
            cards.len(),
            forest.nodes.len(),
            forest.roots.len(),
            digest(&forest)?
        );
        return Ok(());
    }
    let path = args.check_model.expect("clap requires one mode");
    let path = if path.is_dir() {
        path.join("card_structure.json")
    } else {
        path
    };
    let model: Forest = serde_json::from_slice(&fs::read(&path)?)?;
    let compiled = catalog(&cards)?;
    anyhow::ensure!(
        compat::same_contract(&model, &compiled),
        "the structure contract differs; the catalogs cannot be compared"
    );
    let report = compat::superset(&model, &compiled);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "model_structure_sha256": digest(&model)?,
            "catalog_sha256": digest(&compiled)?,
            "model_dsl_source_sha256": model.dsl_source_sha256,
            "catalog_dsl_source_sha256": compiled.dsl_source_sha256,
            "model_roots": report.model_roots,
            "catalog_roots": report.catalog_roots,
            "new_roots": report.new_roots.len(),
            "new_tags": report.new_tags,
            "appended": compat::appended(&report, &model).len(),
            "withdrawn_under_drift": compat::withdrawn(&report, &model)
                .iter()
                .map(|owner| format!("{owner} {}", compat::owner_name(owner).unwrap_or_default()))
                .collect::<Vec<_>>(),
            "changed": report.changed,
            "missing": report.missing,
            "superset": report.holds(),
        }))?
    );
    anyhow::ensure!(
        report.holds(),
        "{} rules the model knows changed and {} are gone",
        report.changed.len(),
        report.missing.len()
    );
    Ok(())
}
