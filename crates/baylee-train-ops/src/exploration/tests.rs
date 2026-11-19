use super::frame::FrameMixture;
use super::*;

fn proposers() -> Vec<String> {
    RunExploration::new(0.25, "mass,react", &["mass", "react"]).proposers
}

fn step(rule: RuleProposal, played: bool, coin: f64, logprob: f32) -> StepExploration {
    StepExploration {
        draw: 0,
        coin,
        rule,
        played,
        proposer: if played { "rule:mass" } else { NET }.to_owned(),
        mixture_logprob: mixture_logprob(
            net_probability(NetBehavior::Sampled(logprob), played),
            0.25,
            rule,
            played,
        ),
    }
}

#[test]
fn the_mixture_probability_is_the_formula_in_every_case() {
    let pi = 0.4_f64;
    let logprob = pi.ln() as f32;
    let p = f64::from(logprob).exp();
    let cases = [
        (RuleProposal::Same, false, 0.75 * p + 0.25),
        (RuleProposal::Differs, false, 0.75 * p),
        (RuleProposal::Differs, true, 0.75 * p + 0.25),
        (RuleProposal::Absent, false, p),
    ];
    for (rule, played, mixture) in cases {
        let got = mixture_logprob(
            net_probability(NetBehavior::Sampled(logprob), played),
            0.25,
            rule,
            played,
        );
        assert_eq!(
            got.map(f32::to_bits),
            Some((mixture.ln() as f32).to_bits()),
            "{rule:?} {played}"
        );
    }
    // A number has no density: π is the point mass at the net's answer.
    let scalar = |rule, played| {
        mixture_logprob(
            net_probability(NetBehavior::PointMass, played),
            0.25,
            rule,
            played,
        )
    };
    assert_eq!(scalar(RuleProposal::Same, false), Some(0.0));
    assert_eq!(
        scalar(RuleProposal::Differs, false),
        Some(0.75_f64.ln() as f32)
    );
    assert_eq!(
        scalar(RuleProposal::Differs, true),
        Some(0.25_f64.ln() as f32)
    );
    assert_eq!(scalar(RuleProposal::Absent, false), Some(0.0));
}

/// A step whose executed action carries no behaviour probability (a blocker
/// answer the attackers' counts moved) has no π, so no μ either: never zero,
/// never the point mass of a number, in every case of the mixture.
#[test]
fn without_a_behaviour_probability_there_is_no_mixture() {
    for (rule, played) in [
        (RuleProposal::Absent, false),
        (RuleProposal::Same, false),
        (RuleProposal::Differs, false),
        (RuleProposal::Differs, true),
    ] {
        assert_eq!(net_probability(NetBehavior::Unknown, played), None);
        assert_eq!(
            mixture_logprob(
                net_probability(NetBehavior::Unknown, played),
                0.25,
                rule,
                played
            ),
            None,
            "{rule:?} {played}"
        );
    }
    let table = proposers();
    let mut unknown = step(RuleProposal::Differs, false, 0.6, -1.2);
    unknown.mixture_logprob = None;
    check_step(&unknown, 0.25, false, 0.0, &table).unwrap();
    assert_eq!(
        NetBehavior::recorded(&unknown, false, 0.0).unwrap(),
        NetBehavior::Unknown
    );
    // A step recorded without μ whose row has a probability is refused.
    assert!(check_step(&unknown, 0.25, true, -1.2, &table).is_err());
    assert!(check_step(&unknown, 0.25, false, -1.2, &table).is_err());
    // A row without a probability whose record carries a μ that is not a
    // number's point mass is refused.
    let made_up = step(RuleProposal::Differs, false, 0.6, -1.2);
    assert!(check_step(&made_up, 0.25, false, 0.0, &table).is_err());
}

#[test]
fn the_mixture_sums_to_one_over_the_answers() {
    // Three answers with π = 0.5, 0.3, 0.2; the layer answers the second.
    let pi = [0.5_f64, 0.3, 0.2];
    let epsilon = 0.25;
    let total: f64 = pi
        .iter()
        .enumerate()
        .map(|(at, &p)| {
            let rule = if at == 1 {
                RuleProposal::Same
            } else {
                RuleProposal::Differs
            };
            f64::from(mixture_logprob(Some(p), epsilon, rule, false).unwrap()).exp()
        })
        .sum();
    assert!((total - 1.0).abs() < 1e-6, "{total}");
}

#[test]
fn a_recorded_step_is_checked_against_coin_proposer_and_formula() {
    let table = proposers();
    let logprob = -1.2_f32;
    for good in [
        step(RuleProposal::Differs, true, 0.1, logprob),
        step(RuleProposal::Differs, false, 0.6, logprob),
        step(RuleProposal::Same, false, 0.1, logprob),
        step(RuleProposal::Absent, false, 0.1, logprob),
    ] {
        check_step(&good, 0.25, true, logprob, &table).unwrap();
    }
    // A coin under ε at a different answer must play it, and over ε must not.
    assert!(
        check_step(
            &step(RuleProposal::Differs, false, 0.1, logprob),
            0.25,
            true,
            logprob,
            &table
        )
        .is_err()
    );
    assert!(
        check_step(
            &step(RuleProposal::Differs, true, 0.6, logprob),
            0.25,
            true,
            logprob,
            &table
        )
        .is_err()
    );
    let mut tampered = step(RuleProposal::Differs, true, 0.1, logprob);
    tampered.mixture_logprob = tampered.mixture_logprob.map(|mixture| mixture + 1e-4);
    assert!(check_step(&tampered, 0.25, true, logprob, &table).is_err());
    tampered.mixture_logprob = None;
    assert!(check_step(&tampered, 0.25, true, logprob, &table).is_err());
    let mut wrong = step(RuleProposal::Differs, true, 0.1, logprob);
    wrong.proposer = NET.to_owned();
    assert!(check_step(&wrong, 0.25, true, logprob, &table).is_err());
    wrong.proposer = "rule:gift".to_owned();
    assert!(
        check_step(&wrong, 0.25, true, logprob, &table).is_err(),
        "not in the table"
    );
    // π itself is part of the check.
    assert!(
        check_step(
            &step(RuleProposal::Differs, true, 0.1, logprob),
            0.25,
            true,
            -1.3,
            &table
        )
        .is_err()
    );
}

#[test]
fn the_run_contract_validates_its_table_and_epsilon() {
    let run = RunExploration::new(0.25, "mass,react", &["mass", "react"]);
    run.validate().unwrap();
    assert_eq!(run.proposers, ["net", "rules", "rule:mass", "rule:react"]);
    for epsilon in [0.0, 1.0, -0.1, f64::NAN] {
        let mut bad = run.clone();
        bad.epsilon = epsilon;
        assert!(bad.validate().is_err(), "{epsilon}");
    }
    let mut bad = run.clone();
    bad.labels = "rule_answers".to_owned();
    assert!(bad.validate().is_err());
    let mut bad = run;
    bad.proposers.push("rule:mass".to_owned());
    assert!(bad.validate().is_err());
}

#[test]
fn a_frame_round_trips_and_an_empty_frame_is_unchanged() {
    for (rule, played, proposer) in [
        (RuleProposal::Absent, false, 0),
        (RuleProposal::Same, false, 0),
        (RuleProposal::Differs, false, 0),
        (RuleProposal::Differs, true, 3),
    ] {
        let mixture = FrameMixture {
            mixture_logprob: Some(-0.75),
            rule,
            played,
            proposer,
        };
        let flags = 0b101 | mixture.flags();
        assert_eq!(
            FrameMixture::decode(flags, mixture.word()).unwrap(),
            Some(mixture)
        );
        // Without a behaviour probability: no ln μ, a zero word, and no
        // log-probability bit on the frame.
        let unknown = FrameMixture {
            mixture_logprob: None,
            ..mixture
        };
        assert_eq!(unknown.word(), 0);
        let flags = 0b001 | unknown.flags();
        assert_ne!(flags & frame::NO_PROBABILITY, 0);
        assert_eq!(FrameMixture::decode(flags, 0).unwrap(), Some(unknown));
        assert!(
            FrameMixture::decode(flags, (-0.75_f32).to_bits()).is_err(),
            "a word beside no probability"
        );
        assert!(
            FrameMixture::decode(flags | 0b100, 0).is_err(),
            "a log-probability beside no probability"
        );
    }
    assert!(
        FrameMixture::decode(frame::NO_PROBABILITY, 0).is_err(),
        "no probability without the mixture flag"
    );
    // A frame without exploration keeps its flags and a zero word.
    assert_eq!(FrameMixture::decode(0b111, 0).unwrap(), None);
    // A bundle-distillation frame's N ≠ R bit is no exploration either.
    assert_eq!(
        FrameMixture::decode(0b111 | frame::DISTILL_DIFFERS, 0).unwrap(),
        None
    );
    assert!(
        FrameMixture::decode(0b111, 7).is_err(),
        "a word without the flag"
    );
    assert!(FrameMixture::decode(1 << 20, 0).is_err(), "unknown flag");
    let both = frame::MIXTURE | frame::RULE_SAME | frame::RULE_DIFFERS;
    assert!(FrameMixture::decode(both, (-0.5_f32).to_bits()).is_err());
    assert!(
        FrameMixture::decode(frame::MIXTURE, 0.5_f32.to_bits()).is_err(),
        "μ above one"
    );
}
