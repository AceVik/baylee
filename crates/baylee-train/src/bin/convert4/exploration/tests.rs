use super::*;
use baylee_train::convert4::AuxTargets4;
use baylee_train::features4::{EncodedState4, StepProvenance4, TeacherTarget4};
use baylee_train_ops::exploration::{NetBehavior, RuleProposal, StepExploration};
use std::sync::Arc;

const EPSILON: f64 = 0.25;

fn run() -> RunExploration {
    RunExploration::new(EPSILON, "mass,react", &["mass", "react"])
}

fn provenance(source: SourceKind4, behavior_logprob: f32) -> StepProvenance4 {
    StepProvenance4 {
        source,
        temperature: 0.7,
        thinking_steps: 2,
        behavior_logprob,
        has_logprob: source == SourceKind4::Net,
    }
}

/// A game of `rules.len()` explored learner decisions between house steps,
/// drawn from `seed`'s stream as the collector draws them.
fn game(seed: u64, rules: &[RuleProposal]) -> (Vec<RolloutStepSidecar4>, GameExploration) {
    let mut stream = Rng::new(seed);
    let mut sidecar = Vec::new();
    let mut played_total = 0;
    for (draw, &rule) in rules.iter().enumerate() {
        sidecar.push(RolloutStepSidecar4 {
            provenance: provenance(SourceKind4::House, 0.0),
            ..RolloutStepSidecar4::default()
        });
        let coin = stream.unit();
        let played = rule == RuleProposal::Differs && coin < EPSILON;
        played_total += u32::from(played);
        let logprob = -0.4 - 0.1 * f32::from(u8::try_from(draw).unwrap());
        sidecar.push(RolloutStepSidecar4 {
            provenance: provenance(SourceKind4::Net, logprob),
            free_mask: Vec::new(),
            pick_sequence: Vec::new(),
            exploration: Some(StepExploration {
                draw: u32::try_from(draw).unwrap(),
                coin,
                rule,
                played,
                proposer: if played { "rule:mass" } else { NET }.to_owned(),
                mixture_logprob: exploration::mixture_logprob(
                    exploration::net_probability(NetBehavior::Sampled(logprob), played),
                    EPSILON,
                    rule,
                    played,
                ),
            }),
        });
    }
    let stream = GameExploration {
        seed,
        draws: u32::try_from(rules.len()).unwrap(),
        played: played_total,
    };
    (sidecar, stream)
}

fn explored(sidecar: &mut [RolloutStepSidecar4], at: usize) -> &mut StepExploration {
    sidecar[2 * at + 1].exploration.as_mut().unwrap()
}

/// A change to a recorded game.
type Tamper = dyn Fn(&mut Vec<RolloutStepSidecar4>, &mut GameExploration);

fn rules() -> Vec<RuleProposal> {
    let mut rules = vec![RuleProposal::Differs; 24];
    rules[3] = RuleProposal::Same;
    rules[7] = RuleProposal::Absent;
    rules
}

#[test]
fn a_game_replays_its_seeded_stream_and_formula() {
    let (sidecar, stream) = game(77, &rules());
    assert!(stream.played > 0 && stream.played < stream.draws);
    verify_game(&sidecar, &stream, &run()).unwrap();
}

#[test]
fn a_tampered_game_is_refused() {
    let (sidecar, stream) = game(77, &rules());
    let mut edits: Vec<(&str, Box<Tamper>)> = vec![
        ("another seed", Box::new(|_, game| game.seed += 1)),
        ("another coin", Box::new(|s, _| explored(s, 2).coin += 1e-9)),
        (
            "μ moved",
            Box::new(|s, _| {
                let step = explored(s, 4);
                step.mixture_logprob = step.mixture_logprob.map(|mixture| mixture - 1e-3);
            }),
        ),
        (
            "μ left out beside π",
            Box::new(|s, _| explored(s, 4).mixture_logprob = None),
        ),
        (
            "π moved",
            Box::new(|s, _| s[3].provenance.behavior_logprob -= 1e-3),
        ),
        (
            "played flipped",
            Box::new(|s, _| {
                let step = explored(s, 5);
                step.played = !step.played;
            }),
        ),
        (
            "a draw left out",
            Box::new(|s, _| {
                s.remove(1);
            }),
        ),
        (
            "a record on a house step",
            Box::new(|s, _| {
                s[0].exploration = s[1].exploration.clone();
            }),
        ),
        ("totals", Box::new(|_, game| game.played += 1)),
    ];
    for (why, edit) in edits.drain(..) {
        let (mut sidecar, mut stream) = (sidecar.clone(), stream.clone());
        edit(&mut sidecar, &mut stream);
        assert!(verify_game(&sidecar, &stream, &run()).is_err(), "{why}");
    }
}

/// A blocker answer the attackers' counts moved carries no behaviour
/// probability: its record has no μ, and the game still checks. A μ made up
/// for it, zero or a number's point mass, is refused.
#[test]
fn a_step_without_a_behaviour_probability_records_no_mixture() {
    let (mut sidecar, stream) = game(77, &rules());
    // A decision the layer answered otherwise and the coin left to the net.
    let at = (0..rules().len())
        .find(|&at| {
            let step = explored(&mut sidecar, at);
            step.rule == RuleProposal::Differs && !step.played
        })
        .unwrap();
    sidecar[2 * at + 1].provenance.has_logprob = false;
    sidecar[2 * at + 1].provenance.behavior_logprob = 0.0;
    explored(&mut sidecar, at).mixture_logprob = None;
    verify_game(&sidecar, &stream, &run()).unwrap();
    let mut made_up = sidecar.clone();
    explored(&mut made_up, at).mixture_logprob = Some(0.0);
    assert!(verify_game(&made_up, &stream, &run()).is_err(), "μ = 1");
    let mut sampled = sidecar.clone();
    sampled[2 * at + 1].provenance.behavior_logprob = -0.5;
    assert!(
        verify_game(&sampled, &stream, &run()).is_err(),
        "no μ beside a log-probability"
    );
    // A number's point mass in its place passes the record's own check, as
    // a number's does; admission, which reads the head, refuses it there.
    // Its frame says it has none: no ln μ, a zero word, no log-probability.
    let table = proposer_table([&run()]);
    let mut step = frame(true, Some(explored(&mut sidecar, at).clone()));
    step.provenance = sidecar[2 * at + 1].provenance;
    let (row, bytes) = written(&step, &table).unwrap();
    assert_eq!(row["behavior_mixture"]["mixture_logprob"], Value::Null);
    assert_eq!(row["provenance"]["has_logprob"], json!(false));
    let first = header(&bytes);
    let flags = u32::from_le_bytes(first[24..28].try_into().unwrap());
    assert_ne!(flags & frame::NO_PROBABILITY, 0);
    assert_eq!(flags & 0b100, 0);
    assert_eq!(&first[40..44], &[0; 4]);
}

#[test]
fn a_rule_proposal_is_never_a_supervised_target() {
    let on_policy = Modes {
        on_policy: true,
        shadow_bc: false,
        llm_bc: false,
        live_bc: false,
        self_imitation: false,
        search_targets: false,
        bundle_distill: false,
    };
    refuse_supervised(&on_policy).unwrap();
    for supervised in [
        Modes {
            shadow_bc: true,
            ..on_policy
        },
        Modes {
            llm_bc: true,
            ..on_policy
        },
        Modes {
            live_bc: true,
            ..on_policy
        },
        Modes {
            self_imitation: true,
            ..on_policy
        },
        Modes {
            search_targets: true,
            ..on_policy
        },
        Modes {
            bundle_distill: true,
            ..on_policy
        },
        Modes {
            on_policy: false,
            ..on_policy
        },
    ] {
        assert!(refuse_supervised(&supervised).is_err());
    }
    // In the on-policy dataset itself no decision weighs anything supervised.
    let mut step = frame(true, None);
    step.loss_weight = 1.0;
    apply(&mut step);
    assert_eq!(step.loss_weight, 0.0);
    let mut passive = frame(false, None);
    passive.loss_weight = 0.5;
    apply(&mut passive);
    assert_eq!(
        passive.loss_weight, 0.5,
        "only decisions carry policy weight"
    );
    // A run that declares nothing carries nothing.
    let (sidecar, _) = game(5, &rules());
    assert!(refuse_undeclared(&json!({}), Some(&sidecar)).is_err());
    assert!(refuse_undeclared(&json!({"exploration": {"seed": 1}}), None).is_err());
    refuse_undeclared(&json!({}), None).unwrap();
}

#[test]
fn the_proposer_table_unites_runs_in_order() {
    let other = RunExploration::new(EPSILON, "react,aim", &["react", "aim"]);
    assert_eq!(
        proposer_table([&run(), &other]),
        ["net", "rules", "rule:mass", "rule:react", "rule:aim"]
    );
}

fn frame(is_decision: bool, mixture: Option<StepExploration>) -> Step4 {
    Step4 {
        game_id: 1,
        seat: 0,
        step_cursor: 0,
        turn: 1,
        is_decision,
        loss_weight: 0.0,
        is_terminal: true,
        state: EncodedState4::default(),
        head: 0,
        pending_kind: 0,
        cand_feat: vec![vec![0.0; 4]; 2],
        cand_entity_idx: vec![-1; 2],
        cand_rel_seat_idx: vec![-1; 2],
        cand_card_id: vec![0; 2],
        cand_face_id: vec![255; 2],
        cand_ability_idx: vec![-1; 2],
        cand_rule_refs: vec![Vec::new(); 2],
        pairs: Vec::new(),
        target: TeacherTarget4 {
            single_idx: 1,
            ..TeacherTarget4::default()
        },
        outcome: 1.0,
        aux: AuxTargets4::default(),
        provenance: provenance(SourceKind4::Net, -0.9),
        shadow_label: None,
        llm_label: None,
        seat_input: None,
        distill: None,
        cand_classes: None,
        behavior_mixture: mixture,
        search_root: None,
    }
}

/// Writes one frame through the converter's writer and reads it back
/// through the native reader admission uses.
fn written(step: &Step4, proposers: &[String]) -> anyhow::Result<(Value, Vec<u8>)> {
    let temp = std::env::temp_dir().join(format!("baylee-explore-{}", uuid::Uuid::now_v7()));
    let mut writer = crate::output::GameWriter::new(
        &temp,
        false,
        1,
        Arc::new(std::collections::BTreeMap::new()),
    )?
    .with_proposers(Arc::new(proposers.to_vec()));
    writer.push(step)?;
    let name = writer.finish()?;
    let path = temp.join("train").join(name);
    let mut rows = Vec::new();
    baylee_train_ops::trajectory::read_shard(&path, |row| {
        rows.push(row);
        Ok(())
    })?;
    let bytes = std::fs::read(path)?;
    std::fs::remove_dir_all(&temp)?;
    Ok((rows.remove(0), bytes))
}

/// The decompressed BST4 frame of a shard holding one record.
fn header(bytes: &[u8]) -> Vec<u8> {
    use std::io::Read as _;
    // One record: its 32-byte index row sits before the 24-byte footer.
    let row = &bytes[bytes.len() - 56..bytes.len() - 24];
    let length = usize::try_from(u32::from_le_bytes(row[24..28].try_into().unwrap())).unwrap();
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&bytes[8..8 + length])
        .read_to_end(&mut raw)
        .unwrap();
    raw
}

/// The converter writes ln μ beside π, and a frame without exploration is
/// the frame it always was: no flag, a zero reserved word.
#[test]
fn ln_mu_rides_beside_pi_and_a_plain_frame_is_unchanged() {
    let table = proposer_table([&run()]);
    let mixture = StepExploration {
        draw: 0,
        coin: 0.1,
        rule: RuleProposal::Differs,
        played: true,
        proposer: "rule:mass".to_owned(),
        mixture_logprob: exploration::mixture_logprob(
            exploration::net_probability(NetBehavior::Sampled(-0.9), true),
            EPSILON,
            RuleProposal::Differs,
            true,
        ),
    };
    let (row, _) = written(&frame(true, Some(mixture.clone())), &table).unwrap();
    assert_eq!(row["provenance"]["behavior_logprob"], json!(-0.9_f32));
    assert_eq!(
        row["behavior_mixture"],
        json!({"mixture_logprob": mixture.mixture_logprob, "rule": "differs",
            "played": true, "proposer": 2})
    );
    let (plain, bytes) = written(&frame(true, None), &[]).unwrap();
    assert!(plain.get("behavior_mixture").is_none());
    let (with, _) = written(&frame(true, None), &table).unwrap();
    assert_eq!(plain, with, "a table changes no plain frame");
    // The header of the plain frame: decision, terminal and log-probability
    // flags only, and a zero reserved word.
    let first = header(&bytes);
    assert_eq!(&first[24..28], &0b111_u32.to_le_bytes());
    assert_eq!(&first[40..44], &[0; 4]);
    // A proposer outside the table is refused.
    let mut stray = mixture;
    stray.proposer = "rule:gift".to_owned();
    assert!(written(&frame(true, Some(stray)), &table).is_err());
}
