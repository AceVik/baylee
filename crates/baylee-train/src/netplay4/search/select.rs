//! Choosing among the candidates once their worlds are played
//! ([`Selection4`]).
use super::{Ranked4, RootDecision4, Selection4};

/// What a selection made of the played worlds.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Choice {
    /// The chosen candidate's place in candidate order.
    pub(super) chosen: usize,
    pub(super) means: Vec<f64>,
    pub(super) diffs: Vec<f64>,
    pub(super) ses: Vec<Option<f64>>,
    pub(super) scores: Vec<f64>,
}

/// The first index with the highest score: ties go to the policy's answer,
/// then to candidate order.
pub(super) fn first_best(scores: &[f64]) -> usize {
    let mut chosen = 0;
    for (at, score) in scores.iter().enumerate().skip(1) {
        if *score > scores[chosen] {
            chosen = at;
        }
    }
    chosen
}

/// A count as a float, for means over rounds.
fn real(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Whether a mean improvement clears `max(margin, z · se)`. Without a
/// standard error only `z = 0` can clear it.
fn passes(diff: f64, se: Option<f64>, margin: f64, z: f64) -> bool {
    let threshold = match se {
        Some(se) => margin.max(z * se),
        None if z == 0.0 => margin,
        None => return false,
    };
    diff > threshold
}

/// Chooses among the candidates: `worlds[p][i]` is candidate `i`'s score in
/// world `p` (every world complete), `priors[i]` its prior.
pub(super) fn choose(selection: Selection4, worlds: &[Vec<f32>], priors: &[f64]) -> Choice {
    let k = worlds.first().map_or(0, Vec::len);
    let n = real(worlds.len());
    let column = |i: usize| worlds.iter().map(move |world| f64::from(world[i]));
    let means: Vec<f64> = (0..k).map(|i| column(i).sum::<f64>() / n).collect();
    let paired = |i: usize| {
        worlds
            .iter()
            .map(move |w| f64::from(w[i]) - f64::from(w[0]))
    };
    let diffs: Vec<f64> = (0..k).map(|i| paired(i).sum::<f64>() / n).collect();
    let ses: Vec<Option<f64>> = (0..k)
        .map(|i| {
            (i > 0 && worlds.len() >= 2).then(|| {
                let variance = paired(i).map(|d| (d - diffs[i]).powi(2)).sum::<f64>() / (n - 1.0);
                (variance / n).sqrt()
            })
        })
        .collect();
    let (chosen, scores) = match selection {
        Selection4::Mean => (first_best(&means), means.clone()),
        Selection4::Paired { margin, z } => {
            let mut chosen = 0;
            for i in 1..k {
                if passes(diffs[i], ses[i], margin, z) && (chosen == 0 || diffs[i] > diffs[chosen])
                {
                    chosen = i;
                }
            }
            (chosen, diffs.clone())
        }
        Selection4::Prior { c } => {
            let scores: Vec<f64> = (0..k)
                .map(|i| means[i] + c * priors.get(i).copied().unwrap_or(0.0) / (1.0 + n))
                .collect();
            (first_best(&scores), scores)
        }
    };
    Choice {
        chosen,
        means,
        diffs,
        ses,
        scores,
    }
}

/// Each candidate's prior: the policy's probability of it, the softmax of
/// the raw logits over every legal answer it ranked. An answer it did not
/// rank has prior `0`.
pub(super) fn priors(decision: &RootDecision4, candidates: &[Ranked4]) -> Vec<f64> {
    let ranking = decision.ranking.as_deref().unwrap_or_default();
    let max = ranking
        .iter()
        .map(|r| r.logit)
        .filter(|logit| logit.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    if !max.is_finite() {
        return vec![0.0; candidates.len()];
    }
    let weight = |logit: f32| {
        if logit.is_finite() {
            f64::from(logit - max).exp()
        } else {
            0.0
        }
    };
    let total: f64 = ranking.iter().map(|r| weight(r.logit)).sum();
    candidates.iter().map(|c| weight(c.logit) / total).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worlds(rows: &[&[f32]]) -> Vec<Vec<f32>> {
        rows.iter().map(|row| row.to_vec()).collect()
    }

    #[test]
    fn ties_go_to_the_policy_then_to_candidate_order() {
        assert_eq!(first_best(&[0.5, 0.5, 0.4]), 0);
        assert_eq!(first_best(&[0.4, 0.6, 0.6]), 1);
        assert_eq!(first_best(&[0.4]), 0);
    }

    #[test]
    fn the_mean_rule_takes_any_gain() {
        let played = worlds(&[&[0.50, 0.51], &[0.50, 0.51]]);
        let choice = choose(Selection4::Mean, &played, &[0.9, 0.1]);
        assert_eq!(choice.chosen, 1);
        assert!((choice.diffs[1] - 0.01).abs() < 1e-6);
    }

    #[test]
    fn a_paired_gain_must_clear_the_margin_and_its_noise() {
        let paired = |margin, z| Selection4::Paired { margin, z };
        // A consistent gain of 0.1 in every world: no noise at all.
        let steady = worlds(&[&[0.4, 0.5, 0.45], &[0.6, 0.7, 0.65], &[0.2, 0.3, 0.25]]);
        let choice = choose(paired(0.05, 2.0), &steady, &[]);
        assert_eq!(choice.chosen, 1);
        assert!(choice.ses[1].unwrap() < 1e-6, "{:?}", choice.ses);
        assert_eq!(
            choose(paired(0.12, 0.0), &steady, &[]).chosen,
            0,
            "below the margin"
        );
        // The same mean gain with the worlds disagreeing.
        let noisy = worlds(&[&[0.4, 0.9], &[0.6, 0.3], &[0.2, 0.3]]);
        let choice = choose(paired(0.05, 1.0), &noisy, &[]);
        assert!((choice.diffs[1] - 0.1).abs() < 1e-6);
        assert!(choice.ses[1].unwrap() > 0.1);
        assert_eq!(choice.chosen, 0, "the gain is within its noise");
        assert_eq!(choose(paired(0.05, 0.0), &noisy, &[]).chosen, 1);
        // One world has no standard error: only z = 0 can override.
        let one = worlds(&[&[0.2, 0.5]]);
        assert_eq!(choose(paired(0.05, 1.0), &one, &[]).chosen, 0);
        assert_eq!(choose(paired(0.05, 0.0), &one, &[]).chosen, 1);
        assert_eq!(
            choose(paired(0.0, 0.0), &worlds(&[&[0.5, 0.5]]), &[]).chosen,
            0
        );
    }

    #[test]
    fn the_prior_weighs_less_as_worlds_accumulate() {
        let prior = Selection4::Prior { c: 1.0 };
        // A gain of 0.1 against a policy that is sure of its answer.
        let two = worlds(&[&[0.5, 0.6], &[0.5, 0.6]]);
        assert_eq!(choose(prior, &two, &[0.9, 0.1]).chosen, 0, "0.1 < 0.8 / 3");
        let many = vec![vec![0.5, 0.6]; 15];
        assert_eq!(
            choose(prior, &many, &[0.9, 0.1]).chosen,
            1,
            "0.1 > 0.8 / 16"
        );
        assert_eq!(
            choose(Selection4::Prior { c: 0.0 }, &two, &[0.9, 0.1]).chosen,
            1
        );
    }

    #[test]
    fn priors_are_the_policys_probabilities() {
        use baylee_engine::choice::PlayerAction;
        let ranked = |index, logit| Ranked4 {
            index,
            action: PlayerAction::PassPriority,
            logit,
        };
        let decision = RootDecision4 {
            answer: crate::netplay4::Answer4 {
                action: PlayerAction::PassPriority,
                value: 0.5,
                provenance: crate::features4::StepProvenance4::default(),
                target: crate::features4::TeacherTarget4::default(),
            },
            ranking: Some(vec![ranked(0, 2.0), ranked(1, 0.0), ranked(2, 0.0)]),
            notes_answer: true,
        };
        let p = priors(&decision, &[ranked(0, 2.0), ranked(2, 0.0)]);
        let total = 2.0_f64.exp() + 2.0;
        assert!((p[0] - 2.0_f64.exp() / total).abs() < 1e-9);
        assert!((p[1] - 1.0 / total).abs() < 1e-9);
        assert_eq!(priors(&decision, &[ranked(9, f32::NEG_INFINITY)]), [0.0]);
    }
}
