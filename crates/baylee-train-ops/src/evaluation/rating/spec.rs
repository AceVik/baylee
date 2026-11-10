//! The spec of a rating: the inputs, who played on each side of them, and the
//! fixed points of the scale. An `eval4` summary does not name its arms, so the
//! spec does.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What `rating4` reads, as JSON:
///
/// ```json
/// {"matches": [
///    {"file": "q6/eval-r1b-tac-1v1-test.json", "player": "r1+bundle", "opponent": "sharp-2026-11"},
///    {"ladder": "ladder/ds2a", "player": "r1+bundle", "opponent": "deepseek-low"}],
///  "anchors": {"random": 0, "sharp-2026-11": 1000},
///  "unfair": ["seer"]}
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The inputs, each one match between two named players.
    pub matches: Vec<MatchSpec>,
    /// Players held at a rating: they fix where the scale sits.
    pub anchors: BTreeMap<String, f64>,
    /// Players who see what a fair seat does not (`seer`). They are rated
    /// against the fair players held fixed, so they never move a fair rating
    /// and never link one to an anchor.
    #[serde(default)]
    pub unfair: BTreeSet<String>,
}

/// One input and the players on its two sides.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchSpec {
    /// An `eval4` summary (its `--out`): the player is the net's side, every
    /// game one of its `deal_details`.
    #[serde(default)]
    pub file: Option<PathBuf>,
    /// A `ladder4` root: the player is the net's chair, the opponent the
    /// profile's; clean games only.
    #[serde(default)]
    pub ladder: Option<PathBuf>,
    /// The name of the side under test.
    pub player: String,
    /// The name of the other side.
    pub opponent: String,
    /// The format the input has to have (`1v1`, `2v2`, `ffa4`): a check, never
    /// a conversion.
    #[serde(default)]
    pub format: Option<String>,
    /// A ladder root's games of this profile only.
    #[serde(default)]
    pub profile: Option<String>,
    /// A ladder root's games of this mode only.
    #[serde(default)]
    pub mode: Option<String>,
    /// Free text carried into the report.
    #[serde(default)]
    pub note: Option<String>,
}

impl MatchSpec {
    /// The input's path, whichever kind it is.
    pub fn source(&self) -> Option<&Path> {
        self.file.as_deref().or(self.ladder.as_deref())
    }
}

impl Spec {
    /// Reads a spec; relative paths are read from the spec's directory, and
    /// every input is named by its canonical path, so that one listed twice
    /// under two spellings is still seen.
    ///
    /// # Errors
    /// Rejects unreadable or malformed specs, missing inputs and any spec
    /// [`Spec::check`] refuses.
    pub fn read(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut spec: Self =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let base = path.parent().unwrap_or(Path::new("."));
        for entry in &mut spec.matches {
            for source in [&mut entry.file, &mut entry.ladder].into_iter().flatten() {
                let joined = base.join(&*source);
                *source = joined
                    .canonicalize()
                    .with_context(|| format!("input {}", joined.display()))?;
            }
        }
        spec.check()?;
        Ok(spec)
    }

    /// Refuses a spec that cannot make one table: no anchor, an unfair anchor,
    /// a match without exactly one input or with one name on both sides, and
    /// an input listed twice (its games would count twice).
    ///
    /// # Errors
    /// Names the first thing wrong.
    pub fn check(&self) -> Result<()> {
        ensure!(!self.matches.is_empty(), "a spec needs matches");
        ensure!(
            !self.anchors.is_empty(),
            "a scale needs at least one anchor"
        );
        for (name, rating) in &self.anchors {
            ensure!(rating.is_finite(), "anchor {name} needs a finite rating");
            ensure!(
                !self.unfair.contains(name),
                "{name} is unfair and cannot anchor the scale"
            );
        }
        let mut inputs = BTreeSet::new();
        for entry in &self.matches {
            let source = entry
                .source()
                .context("a match needs a `file` or a `ladder`")?;
            ensure!(
                entry.file.is_none() || entry.ladder.is_none(),
                "a match names one input, `file` or `ladder`: {}",
                source.display()
            );
            ensure!(
                entry.file.is_none() || (entry.profile.is_none() && entry.mode.is_none()),
                "`profile` and `mode` choose games of a ladder root, not of {}",
                source.display()
            );
            for name in [&entry.player, &entry.opponent] {
                ensure!(
                    !name.trim().is_empty() && name.trim() == name,
                    "a player's name is neither empty nor padded: {:?} in {}",
                    name,
                    source.display()
                );
            }
            ensure!(
                entry.player != entry.opponent,
                "{} plays {} against itself",
                source.display(),
                entry.player
            );
            ensure!(
                inputs.insert((source.to_path_buf(), &entry.profile, &entry.mode)),
                "{} is listed twice: its games would count twice",
                source.display()
            );
        }
        Ok(())
    }
}
