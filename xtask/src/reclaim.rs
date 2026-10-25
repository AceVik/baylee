//! Bounded generation of explicit card selections with pinned identities.

use crate::{Path, PathBuf, fs};

mod identity;
mod planning;

use planning::{plan, plan_with_mode};
#[cfg(test)]
mod tests;

struct CardPlan {
    path: PathBuf,
    resolved_path: PathBuf,
    original: String,
    generated: String,
    implemented: bool,
}

struct Plan {
    cards: Vec<CardPlan>,
    mode: Mode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Reclaim,
    CompleteStub,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Self::Reclaim => "reclaim",
            Self::CompleteStub => "complete-stub",
        }
    }

    fn completed(self) -> &'static str {
        match self {
            Self::Reclaim => "reclaimed",
            Self::CompleteStub => "completed",
        }
    }
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

/// Complete only explicitly named machine-owned stubs whose scripts are fully read.
pub(crate) fn complete_stubs(
    root: &Path,
    cache: &Path,
    scripts: &Path,
    names: &[String],
    check: bool,
) -> anyhow::Result<()> {
    plan_with_mode(root, cache, scripts, names, Mode::CompleteStub)?.apply(check)
}

impl Plan {
    /// Recheck every source before the first write, including sources later in the plan.
    fn apply(&self, check: bool) -> anyhow::Result<()> {
        for card in &self.cards {
            anyhow::ensure!(
                fs::canonicalize(&card.path)? == card.resolved_path,
                "{} source path changed after planning; no cards written",
                card.path.display()
            );
            anyhow::ensure!(
                fs::read_to_string(&card.path)? == card.original,
                "{} source changed after {} planning; no cards written",
                card.path.display(),
                self.mode.label()
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
                "{} selected cards would be {}; no cards written",
                changed.len(),
                self.mode.completed()
            );
        } else {
            for card in &changed {
                fs::write(&card.path, &card.generated)?;
                println!(
                    "{} {} ({})",
                    self.mode.completed(),
                    card.path.display(),
                    if card.implemented {
                        "Implemented"
                    } else {
                        "Generated"
                    }
                );
            }
        }
        println!(
            "{}: {} selected, {} changed; tables unchanged",
            self.mode.label(),
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
