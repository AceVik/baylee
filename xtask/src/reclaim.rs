//! Selective return of fully read, tested card definitions to generator ownership.

use crate::{BTreeSet, Path, PathBuf, fs, ledger, scriptgen, scripts, stubgen};
use anyhow::Context;

mod identity;
#[cfg(test)]
mod tests;

struct CardPlan {
    path: PathBuf,
    original: String,
    generated: String,
}

struct Plan {
    cards: Vec<CardPlan>,
}

/// Reclaim only an explicit selection, or preview its complete source changes.
pub(crate) fn run(
    root: &Path,
    cache: &Path,
    scripts: &Path,
    names: &[String],
    check: bool,
) -> anyhow::Result<()> {
    plan(root, cache, scripts, names)?.apply(check)
}

/// Resolve and render the whole selection before any card can be written.
fn plan(root: &Path, cache: &Path, scripts: &Path, names: &[String]) -> anyhow::Result<Plan> {
    anyhow::ensure!(!names.is_empty(), "select at least one card to reclaim");
    let names: BTreeSet<_> = names.iter().collect();
    let ledger = ledger::IndexLedger::from_rows(&baylee_cards_index::ROWS)?;
    let cards_dir = root.join("crates/baylee-cards/src/cards");
    let files = crate::card_files(&cards_dir)?;
    let cats = crate::hold_back::compiled_catalogs();
    let cycles =
        crate::layout::LandCycles::parse(&fs::read_to_string(root.join("data/land-cycles.tsv"))?);
    let holds = crate::coverage_holds::load(root)?;
    let scripts_dir = crate::scripts_root(root, scripts);
    anyhow::ensure!(
        scripts_dir.is_dir(),
        "reclaim requires a local card-script corpus"
    );
    let lookup =
        scriptgen::ScriptLookup::new(scripts_dir.clone(), scripts::build_index(&scripts_dir)?);
    let tokens = crate::tokengen::TokenLookup::beside(&scripts_dir)?;
    let cache = root.join(cache);
    let mut cards = Vec::new();
    let mut selected = BTreeSet::new();
    for name in names {
        let row = ledger.entry_named(name)?;
        if !selected.insert(row.index) {
            continue;
        }
        let slug = crate::front_face_slug(&row.name);
        let path = files
            .get(&slug)
            .with_context(|| format!("no card file for {name}"))?;
        let original = fs::read_to_string(path)?;
        let printing = identity::pinned_printing(&cache, row, &original)?;
        let canonical = crate::layout::path_for(&printing, &slug, &cycles);
        anyhow::ensure!(
            *path == cards_dir.join(&canonical),
            "{} is not at its canonical path {canonical}; reclaim never moves cards",
            row.name
        );
        let tested = root
            .join("crates/baylee-engine/src/engine/card_tests")
            .join(&canonical)
            .is_file();
        anyhow::ensure!(
            holds.accepts_tests(&row.oracle_id, tested),
            "{} needs accepted canonical card tests",
            row.name
        );
        let script = lookup
            .script(&row.name)
            .with_context(|| format!("{} has no reference script", row.name))?;
        anyhow::ensure!(
            scriptgen::transcode(&script, &cats, tokens.as_ref()).is_some(),
            "{} is not fully read: {:?}",
            row.name,
            scriptgen::refusal_reason(&script, &cats, tokens.as_ref())
        );
        let (info, generated) = stubgen::render_stub(
            &printing,
            row,
            &ledger,
            &cats,
            Some(&lookup),
            tokens.as_ref(),
            &cycles,
            &|_| true,
        )?;
        anyhow::ensure!(
            info.path == canonical && info.index == row.index && info.oracle_id == row.oracle_id,
            "{} generated identity or path changed",
            row.name
        );
        anyhow::ensure!(
            generated.contains(stubgen::OWNED_MARKER)
                && !generated.contains(stubgen::STUB_MARKER)
                && crate::knob(&generated, "coverage")
                    .is_some_and(|v| v.starts_with("Coverage::Implemented")),
            "{} would lose accepted coverage",
            row.name
        );
        cards.push(CardPlan {
            path: path.clone(),
            original,
            generated,
        });
    }
    let sources: Vec<_> = cards.iter().map(|card| card.generated.clone()).collect();
    for (card, formatted) in cards.iter_mut().zip(crate::files::format_rust_many(
        &root.join("target"),
        &sources,
    )?) {
        syn::parse_file(&formatted).context("reclaimed source must be valid Rust syntax")?;
        card.generated = formatted;
    }
    Ok(Plan { cards })
}

impl Plan {
    /// Recheck every source before the first write, including sources later in the plan.
    fn apply(&self, check: bool) -> anyhow::Result<()> {
        for card in &self.cards {
            anyhow::ensure!(
                fs::read_to_string(&card.path)? == card.original,
                "{} source changed after reclaim planning; no cards written",
                card.path.display()
            );
        }
        let changed: Vec<_> = self
            .cards
            .iter()
            .filter(|card| card.original != card.generated)
            .collect();
        if check {
            for card in &changed {
                card.preview();
            }
            anyhow::ensure!(
                changed.is_empty(),
                "{} selected cards would be reclaimed; no cards written",
                changed.len()
            );
        } else {
            for card in &changed {
                fs::write(&card.path, &card.generated)?;
                println!("reclaimed {} (Implemented)", card.path.display());
            }
        }
        println!(
            "reclaim: {} selected, {} changed; tables unchanged",
            self.cards.len(),
            changed.len()
        );
        Ok(())
    }
}

impl CardPlan {
    /// A complete unified diff is reviewable without writing either version to disk.
    fn preview(&self) {
        let path = self.path.display();
        println!("--- {path}\n+++ {path} (generator-owned)");
        println!(
            "@@ -1,{} +1,{} @@",
            self.original.lines().count(),
            self.generated.lines().count()
        );
        for line in self.original.lines() {
            println!("-{line}");
        }
        for line in self.generated.lines() {
            println!("+{line}");
        }
    }
}
