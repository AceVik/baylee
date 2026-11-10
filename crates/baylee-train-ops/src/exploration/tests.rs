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
            net_probability(true, logprob, played),
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
        let got = mixture_logprob(net_probability(true, logprob, played), 0.25, rule, played);
        assert_eq!(
            got.to_bits(),
            (mixture.ln() as f32).to_bits(),
            "{rule:?} {played}"
        );
    }
    // A number has no density: π is the point mass at the net's answer.
    let scalar =
        |rule, played| mixture_logprob(net_probability(false, 0.0, played), 0.25, rule, played);
    assert_eq!(scalar(RuleProposal::Same, false), 0.0);
    assert_eq!(scalar(RuleProposal::Differs, false), 0.75_f64.ln() as f32);
    assert_eq!(scalar(RuleProposal::Differs, true), 0.25_f64.ln() as f32);
    assert_eq!(scalar(RuleProposal::Absent, false), 0.0);
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
            f64::from(mixture_logprob(p, epsilon, rule, false)).exp()
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
    tampered.mixture_logprob += 1e-4;
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
            mixture_logprob: -0.75,
            rule,
            played,
            proposer,
        };
        let flags = 0b101 | mixture.flags();
        assert_eq!(
            FrameMixture::decode(flags, mixture.word()).unwrap(),
            Some(mixture)
        );
    }
    // A frame without exploration keeps its flags and a zero word.
    assert_eq!(FrameMixture::decode(0b111, 0).unwrap(), None);
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
