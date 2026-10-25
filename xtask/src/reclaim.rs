//! Selective return of fully read, tested card definitions to generator ownership.

use crate::{Path, PathBuf, fs};

mod identity;
mod planning;

use planning::plan;
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
