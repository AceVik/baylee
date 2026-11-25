//! Rule proposals as recorded exploration in on-policy collection
//! (`eval4 --explore-rules GROUPS --explore-epsilon EPS`, `docs/trained-ai.md`
//! §"Rule proposals as exploration"): the behaviour mixture's contract,
//! shared by the collector's records, the converter's frames and admission.
//!
//! At a learner decision the net samples its answer `N` from π (temperature
//! 0.7) and the tactical layer answers `R` on the same logits, read on copies.
//! Where they differ, `R` is played with probability ε, decided by a coin from
//! the mixture's own seeded stream. The executed action `a` then has
//!
//! ```text
//! μ(a) = (1 − ε)·π(a) + ε·1[a = R]
//! ```
//!
//! π stays the recorded behaviour probability that the replay must reproduce;
//! μ is recorded beside it. Where the layer has no answer the behaviour is the
//! net's, μ = π. Where the executed action carries no behaviour probability
//! (a blocker answer the attackers' stated counts moved,
//! `features4::sample_pairs_constrained`), π is unknown and so is μ: the
//! step records none, never a number in its place. `R` is an action taken,
//! never a label: the records are exploration records and outcomes are their
//! only labels.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};

/// The contract's name in `run.json`, `meta.json` and `collection.json`.
pub const CONTRACT: &str = "rule_proposal_mixture_v1";
/// The proposer of an answer the net sampled.
pub const NET: &str = "net";
/// The proposer of a rule answer no single group can be credited with.
pub const RULES: &str = "rules";
/// The decision kinds the tactical layer answers and the mixture explores:
/// priority, other single choices (targets, modes, mulligans), subsets
/// (attackers, several targets, cards), blocker pairs and numbers.
pub const KINDS: [&str; 5] = ["priority", "single", "subset", "pair", "scalar"];
/// How a game's mixture stream is seeded from the learner's policy seed.
pub const SEED_RULE: &str = "learner_policy_seed_xor_0x6578_706c_6f72_6531_v1";
/// The salt [`SEED_RULE`] names.
pub const SEED_SALT: u64 = 0x6578_706c_6f72_6531;

/// What the tactical layer answered at a decision, against the net's sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleProposal {
    /// The layer had no answer that decodes exactly: the behaviour is π alone.
    Absent,
    /// The layer answered what the net sampled.
    Same,
    /// The layer answered otherwise.
    Differs,
}

/// One explored learner decision as the collector records it, beside the
/// step's provenance (`rollout_sidecar[i].exploration`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepExploration {
    /// This decision's index in its game's mixture stream.
    pub draw: u32,
    /// The coin drawn from that stream, uniform in `[0, 1)`.
    pub coin: f64,
    /// What the layer answered against the net's sample.
    pub rule: RuleProposal,
    /// Whether the layer's answer was played in place of the net's sample.
    pub played: bool,
    /// Who chose the executed action: [`NET`], [`RULES`] or `rule:<group>`.
    pub proposer: String,
    /// `ln μ(a)` of the executed action; `None` (JSON `null`) where it
    /// carries no behaviour probability ([`NetBehavior::Unknown`]).
    pub mixture_logprob: Option<f32>,
}

impl StepExploration {
    /// Whether the executed action is the layer's answer, `1[a = R]`.
    #[must_use]
    pub fn rule_taken(&self) -> bool {
        rule_taken(self.rule, self.played)
    }
}

/// `1[a = R]` from what the layer answered and whether it was played.
#[must_use]
pub fn rule_taken(rule: RuleProposal, played: bool) -> bool {
    rule == RuleProposal::Same || played
}

/// One game's mixture stream, in `games.jsonl` (`exploration`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameExploration {
    /// The stream's seed ([`SEED_RULE`]).
    pub seed: u64,
    /// Coins drawn: one per explored decision.
    pub draws: u32,
    /// Decisions whose layer answer was played.
    pub played: u32,
}

/// The run's exploration, in `run.json` (`exploration`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunExploration {
    /// [`CONTRACT`].
    pub contract: String,
    /// `exploration`: these records are exploration records.
    pub records: String,
    /// `outcomes_only`: no answer in them is a label.
    pub labels: String,
    /// ε, in `(0, 1)`.
    pub epsilon: f64,
    /// The rule groups over the layer, as `--tactics-rules` reads them.
    pub rules: String,
    /// Every proposer a step may name: [`NET`], [`RULES`], then
    /// `rule:<group>` for each group on.
    pub proposers: Vec<String>,
    /// [`KINDS`].
    pub kinds: Vec<String>,
    /// [`SEED_RULE`].
    pub seed_rule: String,
}

impl RunExploration {
    /// The contract a run with these groups and ε declares.
    #[must_use]
    pub fn new(epsilon: f64, rules: &str, groups: &[&str]) -> Self {
        let mut proposers = vec![NET.to_owned(), RULES.to_owned()];
        proposers.extend(groups.iter().map(|group| format!("rule:{group}")));
        Self {
            contract: CONTRACT.to_owned(),
            records: "exploration".to_owned(),
            labels: "outcomes_only".to_owned(),
            epsilon,
            rules: rules.to_owned(),
            proposers,
            kinds: KINDS.iter().map(|&kind| kind.to_owned()).collect(),
            seed_rule: SEED_RULE.to_owned(),
        }
    }

    /// Checks what a reader relies on.
    ///
    /// # Errors
    /// Another contract, an ε outside `(0, 1)`, or a malformed proposer table.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.contract == CONTRACT
                && self.records == "exploration"
                && self.labels == "outcomes_only"
                && self.seed_rule == SEED_RULE
                && self.kinds.iter().map(String::as_str).eq(KINDS),
            "unknown exploration contract"
        );
        validate_epsilon(self.epsilon)?;
        validate_proposers(&self.proposers)
    }
}

/// ε must lie strictly between 0 and 1.
///
/// # Errors
/// Otherwise.
pub fn validate_epsilon(epsilon: f64) -> Result<()> {
    ensure!(
        epsilon.is_finite() && epsilon > 0.0 && epsilon < 1.0,
        "exploration ε must lie in (0, 1)"
    );
    Ok(())
}

/// A proposer table starts `net`, `rules`, and names each group once.
///
/// # Errors
/// Otherwise.
pub fn validate_proposers(proposers: &[String]) -> Result<()> {
    ensure!(
        proposers.len() >= 2
            && proposers.len() <= 256
            && proposers[0] == NET
            && proposers[1] == RULES,
        "a proposer table starts with net and rules"
    );
    for (at, name) in proposers.iter().enumerate().skip(2) {
        let group = name.strip_prefix("rule:").unwrap_or_default();
        ensure!(
            !group.is_empty()
                && group.bytes().all(|b| b.is_ascii_lowercase())
                && !proposers[..at].contains(name),
            "malformed or repeated proposer {name}"
        );
    }
    Ok(())
}

/// What a step's provenance says of π, the net's probability of the executed
/// action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NetBehavior {
    /// A stochastic head's recorded log-probability.
    Sampled(f32),
    /// A number: no density; π is the point mass at the net's own answer,
    /// which a played layer answer leaves.
    PointMass,
    /// No behaviour probability (`has_logprob = false`): a subset, a pair or
    /// a single choice `injective_categorical_menu_v1` does not certify. π is
    /// unknown, and so is μ.
    Unknown,
}

impl NetBehavior {
    /// π as a recorded step's provenance and its record say: a step whose
    /// record carries `ln μ` is a sampled head where the provenance has a
    /// log-probability and a number where it has none; a step recorded
    /// without `ln μ` must carry no behaviour probability at all.
    ///
    /// # Errors
    /// A step recorded without `ln μ` whose provenance has a probability.
    pub fn recorded(
        step: &StepExploration,
        has_logprob: bool,
        behavior_logprob: f32,
    ) -> Result<Self> {
        Ok(match (step.mixture_logprob, has_logprob) {
            (Some(_), true) => Self::Sampled(behavior_logprob),
            (Some(_), false) => Self::PointMass,
            (None, false) if behavior_logprob == 0.0 => Self::Unknown,
            (None, _) => bail!("a step without ln μ carries a behaviour probability"),
        })
    }
}

/// π of the executed action: the recorded probability of a stochastic head,
/// and for a deterministic one (a number) the point mass at the net's
/// answer, which a played layer answer leaves; `None` where the step carries
/// no behaviour probability.
#[must_use]
pub fn net_probability(behavior: NetBehavior, played: bool) -> Option<f64> {
    match behavior {
        NetBehavior::Sampled(logprob) => Some(f64::from(logprob).exp()),
        NetBehavior::PointMass => Some(if played { 0.0 } else { 1.0 }),
        NetBehavior::Unknown => None,
    }
}

/// `ln μ(a)`, with μ = (1 − ε)·π(a) + ε·1[a = R], or π where the layer had no
/// answer; `None` where π is unknown. Computed once here, in plain f64
/// arithmetic, so that the collector, the converter and admission agree to
/// the bit; a sum that rounds above one is a probability of one.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the frame stores ln μ as f32, beside the f32 behaviour log-probability"
)]
pub fn mixture_logprob(
    net_probability: Option<f64>,
    epsilon: f64,
    rule: RuleProposal,
    played: bool,
) -> Option<f32> {
    let net_probability = net_probability?;
    let mixture = match rule {
        RuleProposal::Absent => net_probability,
        RuleProposal::Same | RuleProposal::Differs => {
            let taken = if rule_taken(rule, played) {
                epsilon
            } else {
                0.0
            };
            (1.0 - epsilon) * net_probability + taken
        }
    };
    Some(mixture.min(1.0).ln() as f32)
}

/// Checks one recorded step against ε: the coin decides, the proposer says
/// who played, and `ln μ` follows from π, or is absent exactly where the
/// step carries no behaviour probability ([`NetBehavior::recorded`]).
///
/// # Errors
/// Any disagreement.
pub fn check_step(
    step: &StepExploration,
    epsilon: f64,
    has_logprob: bool,
    behavior_logprob: f32,
    proposers: &[String],
) -> Result<()> {
    ensure!(
        step.coin.is_finite() && (0.0..1.0).contains(&step.coin),
        "exploration coin outside [0, 1)"
    );
    ensure!(
        step.played == (step.rule == RuleProposal::Differs && step.coin < epsilon),
        "a layer answer is played exactly when it differs and the coin falls under ε"
    );
    ensure!(
        (step.proposer == NET) != step.played && proposers.contains(&step.proposer),
        "proposer {} disagrees with who played",
        step.proposer
    );
    let behavior = NetBehavior::recorded(step, has_logprob, behavior_logprob)?;
    let expected = mixture_logprob(
        net_probability(behavior, step.played),
        epsilon,
        step.rule,
        step.played,
    );
    ensure!(
        expected.map(f32::to_bits) == step.mixture_logprob.map(f32::to_bits),
        "recorded ln μ {:?} differs from (1 − ε)·π + ε·1[a = R] = {expected:?}",
        step.mixture_logprob
    );
    Ok(())
}

/// How a converted BST4 frame carries the mixture: bits of the flags word
/// beyond the decision, terminal and log-probability bits, and `ln μ` as f32
/// bits in the header's reserved word, which frames without a mixture keep
/// at zero. A dataset without exploration is byte-identical to before.
pub mod frame {
    use super::RuleProposal;
    use anyhow::{Result, ensure};

    /// The reserved word holds `ln μ`.
    pub const MIXTURE: u32 = 1 << 3;
    /// The layer answered what the net sampled.
    pub const RULE_SAME: u32 = 1 << 4;
    /// The layer answered otherwise.
    pub const RULE_DIFFERS: u32 = 1 << 5;
    /// The layer's answer was played.
    pub const RULE_PLAYED: u32 = 1 << 6;
    /// The proposer's index in `meta.json`'s `behavior_mixture.proposers`.
    pub const PROPOSER_SHIFT: u32 = 8;
    /// Beside [`MIXTURE`]: the executed action carries no behaviour
    /// probability ([`super::NetBehavior::Unknown`]), so the frame has no
    /// `ln μ`; its reserved word stays zero and its log-probability bit clear.
    pub const NO_PROBABILITY: u32 = 1 << 16;
    /// Every bit the mixture may set.
    pub const BITS: u32 = MIXTURE
        | RULE_SAME
        | RULE_DIFFERS
        | RULE_PLAYED
        | (0xff << PROPOSER_SHIFT)
        | NO_PROBABILITY;
    /// The frame's own log-probability bit (`has_logprob`).
    const LOGPROB: u32 = 1 << 2;
    /// Bundle distillation's `N ≠ R` (`convert4 --bundle-distill`): the net's
    /// own answer differs from the frame's label.
    pub const DISTILL_DIFFERS: u32 = 1 << 7;
    /// The bits a frame has without exploration: decision, terminal,
    /// log-probability and bundle distillation's `N ≠ R`.
    pub const BASE_BITS: u32 = 0b111 | DISTILL_DIFFERS;
    /// The reserved header word's byte offset.
    pub const WORD_OFFSET: usize = 40;

    /// One frame's mixture.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct FrameMixture {
        /// `ln μ` of the executed action; `None` where it carries no
        /// behaviour probability ([`NO_PROBABILITY`]).
        pub mixture_logprob: Option<f32>,
        /// What the layer answered.
        pub rule: RuleProposal,
        /// Whether its answer was played.
        pub played: bool,
        /// The proposer's index in the dataset's table.
        pub proposer: u8,
    }

    impl FrameMixture {
        /// The flag bits to add to the frame's flags.
        #[must_use]
        pub fn flags(&self) -> u32 {
            let rule = match self.rule {
                RuleProposal::Absent => 0,
                RuleProposal::Same => RULE_SAME,
                RuleProposal::Differs => RULE_DIFFERS,
            };
            MIXTURE
                | rule
                | if self.played { RULE_PLAYED } else { 0 }
                | if self.mixture_logprob.is_none() {
                    NO_PROBABILITY
                } else {
                    0
                }
                | (u32::from(self.proposer) << PROPOSER_SHIFT)
        }

        /// The reserved word: `ln μ`, or zero without one.
        #[must_use]
        pub const fn word(&self) -> u32 {
            match self.mixture_logprob {
                Some(mixture_logprob) => mixture_logprob.to_bits(),
                None => 0,
            }
        }

        /// Reads a frame's flags and reserved word.
        ///
        /// # Errors
        /// A flag no version sets, a word without the mixture flag, both
        /// proposal bits, a nonfinite `ln μ`, or a frame without `ln μ`
        /// whose word or log-probability bit is set.
        pub fn decode(flags: u32, word: u32) -> Result<Option<Self>> {
            ensure!(
                flags & !(BASE_BITS | BITS) == 0,
                "unknown BST4 frame flags {flags:#x}"
            );
            if flags & MIXTURE == 0 {
                ensure!(
                    flags & BITS == 0 && word == 0,
                    "BST4 mixture fields without the mixture flag"
                );
                return Ok(None);
            }
            let rule = match (flags & RULE_SAME != 0, flags & RULE_DIFFERS != 0) {
                (false, false) => RuleProposal::Absent,
                (true, false) => RuleProposal::Same,
                (false, true) => RuleProposal::Differs,
                (true, true) => anyhow::bail!("a layer answer both same and different"),
            };
            let mixture_logprob = if flags & NO_PROBABILITY == 0 {
                let mixture_logprob = f32::from_bits(word);
                ensure!(
                    mixture_logprob.is_finite() && mixture_logprob <= 0.0,
                    "BST4 ln μ outside (−∞, 0]"
                );
                Some(mixture_logprob)
            } else {
                ensure!(
                    word == 0 && flags & LOGPROB == 0,
                    "a BST4 frame without ln μ carries a word or a behaviour probability"
                );
                None
            };
            Ok(Some(Self {
                mixture_logprob,
                rule,
                played: flags & RULE_PLAYED != 0,
                proposer: u8::try_from((flags >> PROPOSER_SHIFT) & 0xff)?,
            }))
        }
    }
}

#[cfg(test)]
mod tests;
