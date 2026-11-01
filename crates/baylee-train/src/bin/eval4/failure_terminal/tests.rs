use super::*;
use baylee_core::{
    ids::PrintRef,
    preset::{AIProfile, DeckEntry},
};
use baylee_train as train;
use baylee_train::features4::{HeadKind4, StepProvenance4};
#[allow(dead_code)]
#[path = "../../../../tests/support/research.rs"]
mod research_fixtures;

const OWNER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

#[test]
fn research_encoder_is_rejected_before_collection_when_replay_cannot_support_it() {
    for encoder in [0, 5, 6, 10] {
        assert!(validate_encoder(encoder).is_err());
    }
    for encoder in [7, 8, 9] {
        assert!(validate_encoder(encoder).is_ok());
    }
}

#[derive(Clone, Default)]
struct Samples {
    steps: Vec<RolloutStepSidecar4>,
    bindings: Vec<Value>,
}
impl std::ops::Deref for Samples {
    type Target = [RolloutStepSidecar4];
    fn deref(&self) -> &Self::Target {
        &self.steps
    }
}

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::all()
            .find(|card| card.name() == name)
            .unwrap()
            .index,
        print: PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn act(session: &mut Session, sidecar: &mut Samples, action: PlayerAction) {
    let seat = session.pending().asked().unwrap();
    let (pending, view) = session.view_for_with_context(seat).unwrap();
    let state = baylee_train::features7::encode_state7(&view, Some(&pending), None, &[None, None]);
    let menu = baylee_train::features7::encode_candidates7_grouped(&state, &view, &pending);
    let forced = match menu.head {
        HeadKind4::SingleChoice => menu.candidates.len() <= 1,
        HeadKind4::SubsetPick => menu.candidates.is_empty(),
        HeadKind4::PairPick => menu.pairs.is_empty(),
        HeadKind4::ScalarValue => menu.max_choices <= menu.min_choices,
    };
    let stochastic = !forced && menu.head != HeadKind4::ScalarValue;
    let step = RolloutStepSidecar4 {
        provenance: StepProvenance4 {
            source: if forced {
                SourceKind4::Forced
            } else {
                SourceKind4::Net
            },
            has_logprob: stochastic,
            temperature: 0.7,
            thinking_steps: if forced { 0 } else { 2 },
            behavior_logprob: if stochastic { -0.7 } else { 0.0 },
        },
        free_mask: vec![],
        pick_sequence: vec![],
    };
    let mut binding = sample_binding(session, seat, 7, &action, &step).unwrap();
    session.act_slice(seat, action, 0).unwrap();
    binding["post_snapshot_hash"] = json!(format!("{:016x}", session.snapshot_hash()));
    sidecar.steps.push(step);
    sidecar.bindings.push(binding);
}

fn fixture() -> (Session, Samples, ObjectId) {
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset =
        baylee_train::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.seats[0].starting_hand = Some(vec![entry("Fireball")]);
    preset.seats[0].starting_battlefield = vec![entry("Mountain")];
    preset.seats[1].starting_hand = Some(vec![entry("Forest")]);
    let mut session = Session::new_recorded(&preset, "failure-terminal-test").unwrap();
    for player in [OWNER, OTHER] {
        session.take_over(player);
    }
    let mut sidecar = Samples::default();
    for _ in 0..64 {
        if session.observe_view(OWNER).phase == baylee_view::Phase::FirstMain
            && matches!(session.pending(),Pending::Priority {player,..} if *player==OWNER)
        {
            break;
        }
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected opening {other:?}"),
        };
        act(&mut session, &mut sidecar, action);
    }
    let land = session
        .observe_view(OWNER)
        .battlefield
        .iter()
        .find(|card| card.name == "Mountain")
        .unwrap()
        .id;
    act(
        &mut session,
        &mut sidecar,
        PlayerAction::ActivateManaAbility { source: land },
    );
    let card = session
        .observe_view(OWNER)
        .hand
        .iter()
        .find(|card| card.name == "Fireball")
        .unwrap()
        .id;
    (session, sidecar, card)
}

fn cast(session: &mut Session, sidecar: &mut Samples, card: ObjectId) {
    act(session, sidecar, PlayerAction::CastSpell { card });
    while matches!(session.pending(), Pending::ChooseCastMode { .. }) {
        act(session, sidecar, PlayerAction::ChooseMode(0));
    }
    act(session, sidecar, PlayerAction::ChooseNumber(0));
    assert!(matches!(session.pending(), Pending::ChooseTargets { .. }));
}

fn outcome(
    mut session: Session,
    sidecar: &Samples,
    learner: usize,
    guard: bool,
    reason: &str,
) -> (Value, Vec<u8>) {
    let hash = boundary_view_hash(&session, OWNER, 7).unwrap();
    let snapshot = session.snapshot_hash();
    let bytes = session.take_record();
    let value = stop(
        &bytes,
        sidecar,
        &sidecar.bindings,
        7,
        learner,
        OWNER,
        guard,
        reason,
        hash,
        Some(&PlayerAction::PassPriority),
        snapshot,
    )
    .unwrap();
    (value, bytes)
}

#[test]
fn learner_repeated_unpaid_announcement_is_research_terminal_without_mtg_result() {
    let (mut session, mut sidecar, card) = fixture();
    for _ in 0..3 {
        let before = session.snapshot_hash();
        cast(&mut session, &mut sidecar, card);
        act(
            &mut session,
            &mut sidecar,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![OWNER, OTHER],
            },
        );
        assert_eq!(
            session.snapshot_hash(),
            before,
            "automatic rollback must restore the pre-announcement engine hash"
        );
        assert!(
            session
                .observe_view(OWNER)
                .hand
                .iter()
                .any(|object| object.id == card)
        );
    }
    let (value, bytes) = outcome(session, &sidecar, 0, true, "internal_repeated_view");
    assert_eq!(
        value["termination_kind"], "learner_announcement_loop",
        "{}",
        value["boundary"]["rollback_evidence"]
    );
    assert_eq!(value["training_utility"], 0.0);
    assert!(value["mtg_outcome"].is_null());
    assert_eq!(value["bootstrap"], false);
    assert_eq!(value["boundary"]["fallback_executed"], false);
    assert_eq!(
        value["boundary"]["rollback_evidence"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(
        !String::from_utf8(bytes)
            .unwrap()
            .contains("\"kind\":\"end\"")
    );
    assert!(
        sidecar
            .iter()
            .filter(|step| step.provenance.source == SourceKind4::Forced)
            .all(|step| !step.provenance.has_logprob)
    );
}

#[test]
fn opponent_rollbacks_and_non_guard_aborts_remain_unknown() {
    for (learner, guard, reason, expected) in [
        (1, true, "internal_repeated_view", "opponent_failure"),
        (0, false, "engine_refusal", "engine_refusal"),
        (0, false, "answer_cap", "answer_cap"),
    ] {
        let (mut session, mut sidecar, card) = fixture();
        for _ in 0..3 {
            cast(&mut session, &mut sidecar, card);
            act(
                &mut session,
                &mut sidecar,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![OWNER, OTHER],
                },
            );
        }
        let (value, _) = outcome(session, &sidecar, learner, guard, reason);
        assert_eq!(value["termination_kind"], "unknown");
        assert_eq!(value["unknown_reason"], expected);
        assert!(value["training_utility"].is_null());
        assert!(value["mtg_outcome"].is_null());
    }
}

#[test]
fn deliberate_cancellations_and_paid_completion_are_never_rollback_labels() {
    for cancel in [true, false] {
        let (mut session, mut sidecar, card) = fixture();
        for _ in 0..if cancel { 2 } else { 1 } {
            cast(&mut session, &mut sidecar, card);
            act(
                &mut session,
                &mut sidecar,
                if cancel {
                    PlayerAction::CancelCast
                } else {
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![OTHER],
                    }
                },
            );
        }
        let (value, _) = outcome(session, &sidecar, 0, true, "internal_repeated_view");
        assert_eq!(value["termination_kind"], "unknown");
        assert_eq!(value["boundary"]["rollback_evidence"], json!([]));
    }
}

#[test]
fn record_role_sidecar_and_boundary_tampering_are_rejected() {
    let (mut session, sidecar, _) = fixture();
    let hash = boundary_view_hash(&session, OWNER, 7).unwrap();
    let snapshot = session.snapshot_hash();
    let bytes = session.take_record();
    let check = |record: &[u8], samples: &[RolloutStepSidecar4], view, engine| {
        stop(
            record,
            samples,
            &sidecar.bindings,
            7,
            0,
            OWNER,
            true,
            "internal_repeated_view",
            view,
            None,
            engine,
        )
    };
    assert!(check(&bytes, &sidecar, hash, snapshot).is_ok());
    assert!(check(&bytes, &sidecar, hash ^ 1, snapshot).is_err());
    assert!(check(&bytes, &sidecar, hash, snapshot ^ 1).is_err());
    assert!(check(&bytes, &sidecar[..sidecar.len() - 1], hash, snapshot).is_err());
    let mut recovery = sidecar.clone();
    recovery.steps[0].provenance.source = SourceKind4::House;
    assert!(check(&bytes, &recovery, hash, snapshot).is_err());
    let mut changed_sample = sidecar.steps.clone();
    changed_sample[0].provenance.behavior_logprob = -3.0;
    assert!(check(&bytes, &changed_sample, hash, snapshot).is_err());
    let mut bindings = sidecar.bindings.clone();
    bindings[0]["seat"] = json!(1);
    assert!(
        stop(
            &bytes,
            &sidecar,
            &bindings,
            7,
            0,
            OWNER,
            true,
            "internal_repeated_view",
            hash,
            None,
            snapshot
        )
        .is_err()
    );
    let mut changed = bytes.clone();
    changed[0] = b'!';
    assert!(check(&changed, &sidecar, hash, snapshot).is_err());
    let mut lines: Vec<Value> = bytes
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    lines.last_mut().unwrap()["hash"] = json!("wrong");
    let mut tampered = Vec::new();
    for line in lines {
        serde_json::to_writer(&mut tampered, &line).unwrap();
        tampered.push(b'\n');
    }
    assert!(check(&tampered, &sidecar, hash, snapshot).is_err());
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "End-to-end no-recovery fixture with an explicit fake policy"
)]
fn collector_stops_before_applying_internal_house_proposal() {
    use crate::{CoverageMatrix4, EvaluationPlayer, play_one_table};
    struct Recovery(CoverageMatrix4, bool);
    impl EvaluationPlayer for Recovery {
        fn controls_seat(&self, _: usize, _: usize, _: &[Option<u8>]) -> bool {
            true
        }
        fn reseed(&mut self, _: u64) {}
        fn init_game(&mut self, _: &baylee_core::preset::GamePreset) {}
        fn uses_context(&self) -> bool {
            true
        }
        fn observe(&mut self, _: &baylee_view::PlayerView) -> anyhow::Result<()> {
            Ok(())
        }
        fn answer(
            &mut self,
            _: &baylee_view::PlayerView,
            _: &Pending,
            _: &[Option<u8>],
        ) -> anyhow::Result<Option<baylee_train::netplay4::Answer4>> {
            Ok(Some(baylee_train::netplay4::Answer4 {
                action: if self.1 {
                    PlayerAction::ChooseNumber(0)
                } else {
                    PlayerAction::MulliganKeep
                },
                value: 0.5,
                provenance: StepProvenance4 {
                    source: if self.1 {
                        SourceKind4::Net
                    } else {
                        SourceKind4::House
                    },
                    has_logprob: self.1,
                    temperature: 0.7,
                    thinking_steps: 2,
                    behavior_logprob: -0.7,
                },
                target: baylee_train::features4::TeacherTarget4::default(),
            }))
        }
        fn coverage(&self) -> &CoverageMatrix4 {
            &self.0
        }
    }
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let mut player = Recovery(CoverageMatrix4::default(), false);
    let result = play_one_table(
        "no-recovery",
        0,
        0,
        12,
        19,
        &[&deck, &deck],
        &[AIProfile::STEADY; 2],
        &[],
        &[None, None],
        Some(&mut player),
        true,
        5000,
        true,
        None,
        Some(7),
        None,
    )
    .unwrap();
    assert_eq!(result.3, 0);
    assert_eq!(result.8["termination_kind"], "unknown");
    assert!(result.9.is_empty());
    let lines: Vec<Value> = result
        .7
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert!(!lines.iter().any(|line| line["kind"] == "input"));
    for (refusal, cap, reason) in [(true, 5000, "engine_refusal"), (false, 0, "answer_cap")] {
        let mut player = Recovery(CoverageMatrix4::default(), refusal);
        let result = play_one_table(
            "unknown-stop",
            0,
            0,
            12,
            19,
            &[&deck, &deck],
            &[AIProfile::STEADY; 2],
            &[],
            &[None, None],
            Some(&mut player),
            true,
            cap,
            true,
            None,
            Some(7),
            None,
        )
        .unwrap();
        assert_eq!(result.3, 0);
        assert_eq!(result.8["termination_kind"], "unknown");
        assert_eq!(result.8["unknown_reason"], reason);
        assert!(result.8["training_utility"].is_null());
        assert!(result.9.is_empty());
    }
}

#[test]
fn scalar_net_frames_keep_their_absent_actor_probability() {
    let fixture = research_fixtures::natural_episode(0);
    baylee_train::failure_terminal::verify_episode(
        &fixture.record,
        &fixture.sidecar,
        &fixture.episode,
        7,
        0,
    )
    .unwrap();
    assert!(
        fixture
            .sidecar
            .iter()
            .any(|step| step.provenance.source == SourceKind4::Net
                && !step.provenance.has_logprob
                && step.provenance.behavior_logprob == 0.0)
    );
    let mut invalid = fixture.sidecar.clone();
    invalid[0].provenance.behavior_logprob = f32::NAN;
    assert!(
        baylee_train::failure_terminal::verify_episode(
            &fixture.record,
            &invalid,
            &fixture.episode,
            7,
            0,
        )
        .is_err()
    );
}
