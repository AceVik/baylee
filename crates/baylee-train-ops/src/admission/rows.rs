use super::{GameMixture, Seat};
use crate::exploration::{self, NetBehavior, RuleProposal};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub(super) struct Row {
    pub game_id: u32,
    pub seat: u8,
    step_cursor: u32,
    is_terminal: bool,
    is_decision: bool,
    outcome: f64,
    head: u8,
    cand_feat: Vec<serde::de::IgnoredAny>,
    pairs: Vec<serde::de::IgnoredAny>,
    target: Value,
    provenance: Provenance,
    #[serde(default)]
    loss_weight: Option<f64>,
    #[serde(default)]
    shadow_label: Option<serde::de::IgnoredAny>,
    #[serde(default)]
    llm_label: Option<serde::de::IgnoredAny>,
    /// A converted frame's behaviour mixture (`crate::exploration::frame`).
    #[serde(default)]
    behavior_mixture: Option<FrameMixture>,
}

#[derive(Deserialize)]
struct Provenance {
    source: String,
    temperature: f64,
    thinking_steps: u8,
    has_logprob: bool,
    behavior_logprob: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameMixture {
    /// `None` (`null`) where the executed action carries no behaviour
    /// probability (`crate::exploration::frame::NO_PROBABILITY`).
    mixture_logprob: Option<f64>,
    rule: RuleProposal,
    played: bool,
    proposer: u8,
}

/// The collection's exploring games and the size of its proposer table.
#[derive(Default)]
pub(super) struct Mixtures {
    pub games: BTreeMap<u32, GameMixture>,
    pub proposers: usize,
}

#[derive(Default)]
pub(super) struct History {
    pub frames: u32,
    pub terminal: bool,
    pub decisions: u64,
    /// Learner decisions with a behaviour mixture, and those the layer played.
    pub explored: u64,
    pub rule_played: u64,
    outcome: Option<f64>,
}

impl History {
    pub fn observe(&mut self, row: &Row, role: &Seat, mixtures: &Mixtures) -> Result<()> {
        ensure!(
            !self.terminal && row.step_cursor == self.frames,
            "missing, duplicated or post-terminal observation"
        );
        ensure!(
            row.outcome.is_finite()
                && (0.0..=1.0).contains(&row.outcome)
                && self
                    .outcome
                    .is_none_or(|v| v.to_bits() == row.outcome.to_bits()),
            "invalid or inconsistent terminal utility"
        );
        ensure!(
            row.shadow_label.is_none() && row.llm_label.is_none(),
            "teacher labels in on-policy history"
        );
        self.outcome = Some(row.outcome);
        self.frames = self
            .frames
            .checked_add(1)
            .context("observation count overflow")?;
        self.terminal = row.is_terminal;
        self.mixture(row, role, mixtures)?;
        let p = &row.provenance;
        ensure!(
            matches!(p.source.as_str(), "Net" | "House" | "Forced"),
            "search/external/unknown provenance in raw collection"
        );
        if !row.is_decision {
            ensure!(!p.has_logprob, "passive frame claims an action probability");
            return Ok(());
        }
        if role.role == "house" {
            ensure!(
                p.source != "Net" && !p.has_logprob,
                "neural action assigned to house seat"
            );
            return Ok(());
        }
        ensure!(
            p.source != "House",
            "neural actor fell back to house policy"
        );
        if p.source == "Forced" {
            ensure!(
                !p.has_logprob,
                "forced action claims a stochastic probability"
            );
            return Ok(());
        }
        if role.role == "learner" {
            ensure!(
                p.thinking_steps == 2 && (p.temperature - 0.7).abs() < 1e-6,
                "learner row depth/temperature mismatch"
            );
        }
        ensure!(
            p.temperature.is_finite() && p.behavior_logprob.is_finite(),
            "nonfinite policy provenance"
        );
        row.validate_target()?;
        if p.has_logprob {
            ensure!(
                row.head == 0
                    && row.cand_feat.len() > 1
                    && p.temperature > 0.35
                    && p.behavior_logprob <= 1e-6,
                "invalid sampled categorical action probability"
            );
        } else {
            ensure!(
                p.behavior_logprob == 0.0,
                "excluded neural action has a nonzero probability field"
            );
        }
        if role.role == "learner" && p.has_logprob {
            self.decisions += 1;
        }
        Ok(())
    }

    /// An exploring game's frames: every learner decision the net answered
    /// carries its mixture, with μ consistent with its π, ε and the recorded
    /// proposal, and without μ exactly where π is none; no other frame
    /// carries one; and no decision weighs anything supervised.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the frame holds both log-probabilities as f32"
    )]
    fn mixture(&mut self, row: &Row, role: &Seat, mixtures: &Mixtures) -> Result<()> {
        let p = &row.provenance;
        let explored = row.is_decision && role.role == "learner" && p.source == "Net";
        let Some(game) = mixtures.games.get(&row.game_id) else {
            ensure!(
                row.behavior_mixture.is_none(),
                "a behaviour mixture in a game that declares none"
            );
            return Ok(());
        };
        if row.is_decision {
            ensure!(
                row.loss_weight == Some(0.0),
                "an exploring game's decision carries a supervised weight"
            );
        }
        let Some(mixture) = &row.behavior_mixture else {
            ensure!(
                !explored,
                "an explored learner decision without its behaviour mixture"
            );
            return Ok(());
        };
        ensure!(
            explored,
            "a behaviour mixture on a frame the learner's net did not answer"
        );
        ensure!(
            !mixture.played || mixture.rule == RuleProposal::Differs,
            "a played layer answer that does not differ from the net's"
        );
        ensure!(
            (mixture.proposer == 0) != mixture.played
                && usize::from(mixture.proposer) < mixtures.proposers,
            "proposer {} disagrees with who played",
            mixture.proposer
        );
        // π as the row records it: its own log-probability for certified
        // categorical choices, a number's point mass, or none on excluded
        // categorical/subset/pair frames whose μ is none too.
        let behavior = match (p.has_logprob, row.head) {
            (true, _) => NetBehavior::Sampled(p.behavior_logprob as f32),
            (false, 3) => NetBehavior::PointMass,
            (false, 0..=2) if p.behavior_logprob == 0.0 => NetBehavior::Unknown,
            (false, _) => bail!("an explored decision without its behaviour probability"),
        };
        let expected = exploration::mixture_logprob(
            exploration::net_probability(behavior, mixture.played),
            game.epsilon,
            mixture.rule,
            mixture.played,
        );
        ensure!(
            expected.map(f32::to_bits)
                == mixture
                    .mixture_logprob
                    .map(|mixture_logprob| (mixture_logprob as f32).to_bits()),
            "ln μ {:?} is not (1 − ε)·π + ε·1[a = R] = {expected:?}",
            mixture.mixture_logprob
        );
        self.explored += 1;
        self.rule_played += u64::from(mixture.played);
        Ok(())
    }
}

impl Row {
    fn validate_target(&self) -> Result<()> {
        let row = self;
        match row.head {
            0 => {
                let size = row.cand_feat.len();
                let index = row.target["single_idx"]
                    .as_u64()
                    .context("missing categorical target")?;
                ensure!(
                    usize::try_from(index)? < size,
                    "categorical target outside menu"
                );
            }
            1 | 2 => {
                let size = if row.head == 1 {
                    row.cand_feat.len()
                } else {
                    row.pairs.len()
                };
                for name in ["multi_mask", "free_mask"] {
                    let mask = row.target[name]
                        .as_array()
                        .context("missing conditional action mask")?;
                    ensure!(
                        mask.len() == size,
                        "conditional action mask length mismatch"
                    );
                    for v in mask {
                        ensure!(
                            matches!(v.as_u64(), Some(0 | 1)),
                            "non-binary conditional action mask"
                        );
                    }
                }
            }
            3 => {}
            _ => anyhow::bail!("unknown neural policy head"),
        }
        Ok(())
    }
}
