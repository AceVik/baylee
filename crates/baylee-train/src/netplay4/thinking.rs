//! Adaptive thinking depth: the net says how sure it is of its answer, and a
//! decision it is unsure of is thought through again in the deeper graph.
//!
//! Only a model with the deliberation block states a confidence (output
//! `think_confidence`, contract [`THINKING_CONTRACT`] in its metadata; see
//! `docs/proposals/baylee-ai-thinking.md`). The threshold compares the net's
//! own output; no rule of the game is read here.

use anyhow::{Context as _, ensure};

/// The metadata contract of a graph that states a confidence.
pub const THINKING_CONTRACT: &str = "deliberation_v1";

/// Whether a model's metadata declares the confidence output.
#[must_use]
pub(super) fn states_confidence(meta: &serde_json::Value) -> bool {
    meta["thinking_outputs"]["contract"] == THINKING_CONTRACT
        && meta["thinking_outputs"]["names"]
            .as_array()
            .is_some_and(|names| names.iter().any(|name| name == "think_confidence"))
}

/// When a decision is thought through again, and how deep.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct AdaptiveThinking4 {
    /// Deliberation steps of the second pass.
    pub deep: u8,
    /// A confidence below this asks for the second pass.
    pub threshold: f32,
}

impl AdaptiveThinking4 {
    /// A second pass at `deep` steps for answers less sure than `threshold`.
    ///
    /// # Errors
    /// When `deep` is not deeper than `shallow` within `1..=8`, or the
    /// threshold is not a probability above zero.
    pub fn new(shallow: u8, deep: u8, threshold: f32) -> anyhow::Result<Self> {
        ensure!(
            (1..=8).contains(&shallow) && deep > shallow && deep <= 8,
            "the deeper pass must think longer than the first, within 1..=8 steps"
        );
        ensure!(
            threshold > 0.0 && threshold <= 1.0,
            "the confidence threshold is a probability above zero"
        );
        Ok(Self { deep, threshold })
    }

    /// Reads `DEEP,THRESHOLD` (for example `8,0.7`) for a first pass at
    /// `shallow` steps.
    ///
    /// # Errors
    /// When the text is not two numbers [`Self::new`] accepts.
    pub fn parse(text: &str, shallow: u8) -> anyhow::Result<Self> {
        let (deep, threshold) = text
            .split_once(',')
            .context("adaptive thinking is DEEP,THRESHOLD, for example 8,0.7")?;
        Self::new(
            shallow,
            deep.trim().parse().context("DEEP is a step count")?,
            threshold
                .trim()
                .parse()
                .context("THRESHOLD is a probability")?,
        )
    }

    /// Whether an answer of this confidence is thought through again. A
    /// confidence that is not a number is not trusted.
    #[must_use]
    pub fn escalates(&self, confidence: f32) -> bool {
        confidence.is_nan() || confidence < self.threshold
    }
}

/// How often a player thought deeper, over its decisions with a choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize)]
pub struct ThinkingUse4 {
    /// Decisions with two or more answers the first pass answered.
    pub decisions: u64,
    /// Of those, the ones thought through again.
    pub escalated: u64,
    /// Sum of the first pass's confidences, for their mean.
    pub confidence_sum: f64,
}

impl ThinkingUse4 {
    pub(super) fn record(&mut self, confidence: f32, escalated: bool) {
        self.decisions += 1;
        self.escalated += u64::from(escalated);
        if confidence.is_finite() {
            self.confidence_sum += f64::from(confidence);
        }
    }

    /// Adds another player's counts.
    pub fn add(&mut self, other: &Self) {
        self.decisions += other.decisions;
        self.escalated += other.escalated;
        self.confidence_sum += other.confidence_sum;
    }

    /// The counts and the shares they give, for a run's report.
    #[must_use]
    pub fn report(&self) -> serde_json::Value {
        #[allow(
            clippy::cast_precision_loss,
            reason = "decision counts of a run are far below 2^52"
        )]
        let n = self.decisions.max(1) as f64;
        #[allow(clippy::cast_precision_loss, reason = "as above")]
        let escalated = self.escalated as f64;
        serde_json::json!({
            "decisions": self.decisions,
            "escalated": self.escalated,
            "escalated_share": escalated / n,
            "mean_first_confidence": self.confidence_sum / n,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_pass_must_be_deeper_and_its_threshold_a_probability() {
        let adaptive = AdaptiveThinking4::parse("8, 0.7", 2).unwrap();
        assert_eq!(
            adaptive,
            AdaptiveThinking4 {
                deep: 8,
                threshold: 0.7
            }
        );
        for (text, shallow) in [
            ("2,0.7", 2),
            ("1,0.7", 2),
            ("9,0.7", 2),
            ("8,0", 2),
            ("8,1.5", 2),
            ("8,NaN", 2),
            ("8", 2),
            ("deep,0.5", 2),
            ("4,0.5", 0),
        ] {
            assert!(AdaptiveThinking4::parse(text, shallow).is_err(), "{text}");
        }
    }

    #[test]
    fn only_an_unsure_answer_is_thought_through_again() {
        let adaptive = AdaptiveThinking4::new(1, 4, 0.6).unwrap();
        assert!(adaptive.escalates(0.59));
        assert!(!adaptive.escalates(0.6));
        assert!(!adaptive.escalates(1.0));
        assert!(adaptive.escalates(f32::NAN));
        // A threshold of one asks again unless the net is certain.
        let always = AdaptiveThinking4::new(1, 4, 1.0).unwrap();
        assert!(always.escalates(0.999));
    }

    #[test]
    fn the_use_counts_decisions_and_second_passes() {
        let mut usage = ThinkingUse4::default();
        usage.record(0.9, false);
        usage.record(0.3, true);
        usage.record(f32::NAN, true);
        let mut total = ThinkingUse4::default();
        total.add(&usage);
        total.add(&usage);
        assert_eq!((total.decisions, total.escalated), (6, 4));
        let report = total.report();
        assert_eq!(report["escalated"], 4);
        assert!((report["escalated_share"].as_f64().unwrap() - 4.0 / 6.0).abs() < 1e-12);
        assert!((report["mean_first_confidence"].as_f64().unwrap() - 0.4).abs() < 1e-6);
    }

    #[test]
    fn only_the_declared_contract_states_a_confidence() {
        let declared = serde_json::json!({"thinking_outputs": {
            "contract": THINKING_CONTRACT, "names": ["think_confidence"]}});
        assert!(states_confidence(&declared));
        assert!(!states_confidence(&serde_json::json!({})));
        assert!(!states_confidence(
            &serde_json::json!({"thinking_outputs": {
            "contract": "deliberation_v0", "names": ["think_confidence"]}})
        ));
        assert!(!states_confidence(
            &serde_json::json!({"thinking_outputs": {
            "contract": THINKING_CONTRACT, "names": []}})
        ));
    }
}
