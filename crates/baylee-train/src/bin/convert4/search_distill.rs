//! Search distillation (`--search-targets`): the answers a lookahead search
//! played become the policy targets, so the net learns what its search
//! found. Only the net seat's searched decisions keep their weight (its
//! unsearched ones at `--search-unsearched-weight`); a house opponent's,
//! forced answers and every other step keep their frame for memory and
//! value but no policy weight.

use anyhow::{Context as _, ensure};
use baylee_train::convert4::Step4;
use baylee_train::features4::SourceKind4;
use serde_json::{Value, json};

const CONTRACT: &str = "search_distillation_v1";

/// The runs a search-distillation dataset is made of, and its weighting.
pub(super) struct Targets {
    weighting: Weighting,
    searches: Vec<Value>,
}

/// The policy weight each step keeps; copied into every conversion worker.
#[derive(Clone, Copy, Debug)]
pub(super) struct Weighting {
    unsearched_net_weight: f32,
}

impl Targets {
    /// Checks the weight of the net's unsearched decisions.
    pub(super) fn new(unsearched_net_weight: f32) -> anyhow::Result<Self> {
        ensure!(
            unsearched_net_weight.is_finite() && (0.0..=1.0).contains(&unsearched_net_weight),
            "--search-unsearched-weight {unsearched_net_weight} outside [0, 1]"
        );
        Ok(Self {
            weighting: Weighting {
                unsearched_net_weight,
            },
            searches: Vec::new(),
        })
    }

    /// The weighting the conversion workers apply.
    pub(super) fn weighting(&self) -> Weighting {
        self.weighting
    }

    /// Admits one run: it must have been played with a search.
    pub(super) fn admit(&mut self, run_json: &Value) -> anyhow::Result<()> {
        let search = run_json
            .get("search")
            .filter(|search| !search.is_null())
            .context("--search-targets converts searched runs only; this run has no search")?;
        ensure!(
            search.get("contract").and_then(Value::as_str).is_some(),
            "the run's search names no contract"
        );
        self.searches.push(search.clone());
        Ok(())
    }

    /// The dataset metadata naming these targets.
    pub(super) fn metadata(&self, games: usize) -> Value {
        json!({"version": 1, "contract": CONTRACT, "games": games,
            "targets": "the answers searched decisions played",
            "unsearched_net_weight": self.weighting.unsearched_net_weight,
            "searches": self.searches})
    }
}

impl Weighting {
    /// Keeps a step's policy weight for a searched decision, scales it for
    /// the net's unsearched ones and drops it for everything else.
    pub(super) fn apply(self, step: &mut Step4) {
        step.loss_weight *= self.weight(step.is_decision, step.provenance.source);
    }

    fn weight(self, is_decision: bool, source: SourceKind4) -> f32 {
        match (is_decision, source) {
            (true, SourceKind4::Search) => 1.0,
            (true, SourceKind4::Net) => self.unsearched_net_weight,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_searched_decisions_keep_full_weight() {
        let weighting = Targets::new(0.25).expect("weight in range").weighting();
        let weight = |source| weighting.weight(true, source);
        assert!((weight(SourceKind4::Search) - 1.0).abs() < f32::EPSILON);
        assert!((weight(SourceKind4::Net) - 0.25).abs() < f32::EPSILON);
        for source in [
            SourceKind4::House,
            SourceKind4::Forced,
            SourceKind4::External,
        ] {
            assert!(
                weight(source).abs() < f32::EPSILON,
                "{source:?} is no target"
            );
        }
    }

    #[test]
    fn an_observation_frame_is_no_target() {
        let weighting = Targets::new(1.0).expect("weight in range").weighting();
        for source in [SourceKind4::Search, SourceKind4::Net] {
            assert!(weighting.weight(false, source).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn a_run_without_search_is_refused() {
        let mut targets = Targets::new(0.0).expect("weight in range");
        assert!(targets.admit(&json!({"search": null})).is_err());
        assert!(targets.admit(&json!({})).is_err());
        assert!(targets.admit(&json!({"search": {}})).is_err());
        targets
            .admit(&json!({"search": {"contract": "belief_lookahead_v2"}}))
            .expect("a searched run");
        assert_eq!(
            targets.metadata(3)["searches"].as_array().map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn weights_outside_the_unit_interval_are_refused() {
        for weight in [-0.1, 1.5, f32::NAN] {
            assert!(Targets::new(weight).is_err(), "{weight}");
        }
    }
}
