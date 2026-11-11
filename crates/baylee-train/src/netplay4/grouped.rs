//! Grouped decode at priority (`--grouped-decode`, off unless asked).
//!
//! At the first tap of a cast the net often puts most of its probability on
//! acting, split across near-equal taps (one per land), while the single pass
//! stays the top answer: it passes although it wants to act. Here the menu is
//! read as two groups by the coarse classes ([`crate::coarse_class`], rule
//! `coarse_class_v1`): pass, and act (tap, cast, activate, land, other). The
//! group with the larger summed probability wins and the answer is taken
//! inside it. The probabilities are the policy's own: its softmax at the
//! sampling temperature, at temperature 0 the plain softmax. No rule of the
//! game is read beyond what kind of answer a candidate is.
//!
//! Only a priority question with both groups on its menu is touched.

use crate::coarse_class::PASS_CLASS;

/// The contract a run names when it decodes this way.
pub const GROUPED_DECODE_CONTRACT: &str = "grouped_decode_v1";

/// A logit that takes a candidate out of the answer (the masks' own value).
const OUT: f32 = -1.0e9;

/// What the grouping found at one priority question.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct Grouping4 {
    /// Summed probability of the pass group.
    pub pass_mass: f32,
    /// Summed probability of the act group.
    pub act_mass: f32,
    /// Whether the act group won.
    pub act_wins: bool,
    /// Whether the plain top answer lies in the losing group: the grouping
    /// changes the greedy answer.
    pub changed: bool,
}

/// The grouping of a priority menu's logits by its candidates' coarse
/// classes, or `None` when the menu lacks a group (nothing to decide).
/// A tie keeps the plain top answer's group.
#[must_use]
pub fn group(logits: &[f32], classes: &[u8], temperature: f32) -> Option<Grouping4> {
    if logits.len() != classes.len() || logits.is_empty() {
        return None;
    }
    let is_pass = |i: usize| classes[i] == PASS_CLASS;
    if (0..classes.len()).all(is_pass) || !(0..classes.len()).any(is_pass) {
        return None;
    }
    let tau = if temperature > 0.0 { temperature } else { 1.0 };
    let (top, best) =
        logits
            .iter()
            .copied()
            .enumerate()
            .fold(
                (0, f32::NEG_INFINITY),
                |acc, (i, l)| {
                    if l > acc.1 { (i, l) } else { acc }
                },
            );
    let (mut pass, mut act) = (0.0_f64, 0.0_f64);
    for (i, &logit) in logits.iter().enumerate() {
        let weight = f64::from((logit - best) / tau).exp();
        if is_pass(i) {
            pass += weight;
        } else {
            act += weight;
        }
    }
    let total = pass + act;
    let top_is_pass = is_pass(top);
    let act_wins = if (act - pass).abs() <= f64::EPSILON * total {
        !top_is_pass
    } else {
        act > pass
    };
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a probability in [0, 1] reported as f32"
    )]
    let share = |mass: f64| (mass / total) as f32;
    Some(Grouping4 {
        pass_mass: share(pass),
        act_mass: share(act),
        act_wins,
        changed: act_wins == top_is_pass,
    })
}

impl Grouping4 {
    /// Whether the grouping took the losing group out of the menu at this
    /// temperature ([`apply`]): the answer is then not the plain policy's
    /// draw, and its provenance claims no behaviour probability.
    #[must_use]
    pub fn decides(&self, temperature: f32) -> bool {
        self.changed || temperature > 0.0
    }
}

/// Takes the losing group out of `logits` where the grouping decides the
/// answer: when it changes the greedy answer, and always when the answer is
/// sampled (the group is decided by mass, the answer drawn inside it).
pub fn apply(logits: &mut [f32], classes: &[u8], grouping: &Grouping4, temperature: f32) {
    if !grouping.decides(temperature) {
        return;
    }
    for (logit, &class) in logits.iter_mut().zip(classes) {
        if (class == PASS_CLASS) == grouping.act_wins {
            *logit = OUT;
        }
    }
}

/// How often the grouping decided, over a player's priority questions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct GroupedUse4 {
    /// Priority questions with both groups on the menu.
    pub decisions: u64,
    /// Of those, the plain top answer was a pass and the act group won.
    pub pass_to_act: u64,
    /// Of those, the plain top answer was an act and the pass group won.
    pub act_to_pass: u64,
}

impl GroupedUse4 {
    pub(super) fn record(&mut self, grouping: &Grouping4) {
        self.decisions += 1;
        if grouping.changed {
            if grouping.act_wins {
                self.pass_to_act += 1;
            } else {
                self.act_to_pass += 1;
            }
        }
    }

    /// Adds another player's counts.
    pub fn add(&mut self, other: &Self) {
        self.decisions += other.decisions;
        self.pass_to_act += other.pass_to_act;
        self.act_to_pass += other.act_to_pass;
    }

    /// The counts with the contract they were taken under, for a report.
    #[must_use]
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({
            "contract": GROUPED_DECODE_CONTRACT,
            "classes": crate::coarse_class::COARSE_CLASS_RULE,
            "decisions": self.decisions,
            "changed": self.pass_to_act + self.act_to_pass,
            "pass_to_act": self.pass_to_act,
            "act_to_pass": self.act_to_pass,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argmax(logits: &[f32]) -> usize {
        (0..logits.len())
            .max_by(|&a, &b| logits[a].total_cmp(&logits[b]))
            .unwrap()
    }

    #[test]
    fn act_mass_split_across_taps_beats_a_top_pass() {
        // Pass is the top answer at 40 %; three taps share the other 60 %.
        let classes = [0, 1, 1, 1];
        let mut logits = [0.4_f32.ln(), 0.22_f32.ln(), 0.2_f32.ln(), 0.18_f32.ln()];
        assert_eq!(argmax(&logits), 0);
        let grouping = group(&logits, &classes, 0.0).unwrap();
        assert!((grouping.pass_mass - 0.4).abs() < 1e-5);
        assert!((grouping.act_mass - 0.6).abs() < 1e-5);
        assert!(grouping.act_wins && grouping.changed);
        apply(&mut logits, &classes, &grouping, 0.0);
        // The best candidate inside the winning group answers.
        assert_eq!(argmax(&logits), 1);
        let mut usage = GroupedUse4::default();
        usage.record(&grouping);
        assert_eq!(
            (usage.decisions, usage.pass_to_act, usage.act_to_pass),
            (1, 1, 0)
        );
        assert_eq!(usage.report()["changed"], 1);
        assert_eq!(usage.report()["contract"], GROUPED_DECODE_CONTRACT);
    }

    #[test]
    fn a_top_act_loses_to_the_heavier_pass_group_and_agreement_changes_nothing() {
        // Pass and cancel together outweigh the top cast.
        let classes = [0, 0, 2];
        let mut logits = [0.3_f32.ln(), 0.3_f32.ln(), 0.4_f32.ln()];
        let grouping = group(&logits, &classes, 0.0).unwrap();
        assert!(!grouping.act_wins && grouping.changed);
        apply(&mut logits, &classes, &grouping, 0.0);
        assert_eq!(classes[argmax(&logits)], PASS_CLASS);

        // The top answer's group wins: the greedy logits stay as they were.
        let classes = [0, 1, 1];
        let before = [0.7_f32.ln(), 0.2_f32.ln(), 0.1_f32.ln()];
        let mut logits = before;
        let grouping = group(&logits, &classes, 0.0).unwrap();
        assert!(!grouping.act_wins && !grouping.changed);
        apply(&mut logits, &classes, &grouping, 0.0);
        assert_eq!(logits, before);
        // Sampled, the answer is drawn inside the winning group.
        apply(&mut logits, &classes, &grouping, 0.7);
        assert!(logits[1] < -1.0e8 && logits[2] < -1.0e8);
        assert!((logits[0] - before[0]).abs() < f32::EPSILON);
    }

    #[test]
    fn the_masses_follow_the_temperature_and_a_tie_keeps_the_top_answer() {
        // At temperature 1 the two acts outweigh the pass; a cold
        // temperature sharpens toward the single best answer, the pass.
        let classes = [0, 1, 1];
        let logits = [0.45_f32.ln(), 0.3_f32.ln(), 0.25_f32.ln()];
        assert!(group(&logits, &classes, 1.0).unwrap().act_wins);
        assert!(!group(&logits, &classes, 0.4).unwrap().act_wins);
        let tie = group(&[0.0, 0.0], &[0, 1], 0.0).unwrap();
        assert!(!tie.act_wins && !tie.changed);
    }

    #[test]
    fn a_menu_of_one_group_is_left_alone() {
        assert_eq!(group(&[1.0, 2.0], &[1, 2], 0.0), None);
        assert_eq!(group(&[1.0, 2.0], &[0, 0], 0.0), None);
        assert_eq!(group(&[], &[], 0.0), None);
        assert_eq!(group(&[1.0], &[0, 1], 0.0), None);
    }
}
