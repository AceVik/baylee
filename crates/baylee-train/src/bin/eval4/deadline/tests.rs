//! The deadline seat with a stand-in for its net and for the twin its worlds
//! are played by: what is played, timed and counted, not playing strength.
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use baylee_engine::choice::Pending;
use baylee_train::netplay4::search::{Selection4, search_decision};

use super::*;
use crate::EvaluationPlayer;
use crate::lookahead::tests::{Fixture, InRollouts, config, lands};

const ME: PlayerId = PlayerId::new(0);
const SEED: u64 = 0x0b5e_55ed_5eed_0001;
/// A budget no search of these tests spends.
const AMPLE: Duration = Duration::from_secs(300);

/// A recorded table at a root of the deadline seat's search.
struct Table {
    session: Session,
    view: PlayerView,
    pending: Pending,
    net: Fixture,
}

/// The search of the deadline seat, at its smallest.
fn fair() -> SearchConfig4 {
    config(3, 2, 2)
        .with_fair(true)
        .with_macros(true)
        .with_neural_opponent(true)
        .with_turns(1)
        .unwrap()
        .with_selection(Selection4::Mean)
        .unwrap()
}

impl Table {
    /// The stand-in, which passes at every priority, at its first main phase
    /// with a land to play.
    fn new() -> Self {
        let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
        let steady = baylee_core::preset::AIProfile::STEADY;
        let preset = baylee_train::selfplay::table_for(12, &[&deck, &deck], &[steady; 2], &[]);
        let mut session = Session::new_recorded(&preset, "test").unwrap();
        session.describe("deadline".into(), vec!["0".into(), "1".into()]);
        assert!(session.take_over(ME));
        let mut net = Fixture::new(true, lands);
        for _ in 0..5000 {
            if session.needs_pump() {
                session.pump_at_most(1);
                continue;
            }
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            SearchPlayer4::observe(&mut net, &view).unwrap();
            let decision = SearchPlayer4::decide(&mut net, &view, &pending, &[]);
            let lands = matches!(
                &pending,
                Pending::Priority { legal, .. } if !legal.lands.is_empty()
            );
            if lands && skip_reason(&view, &pending, &decision, fair(), None).is_none() {
                return Self {
                    session,
                    view,
                    pending,
                    net,
                };
            }
            session.act_slice(ME, decision.answer.action, 0).unwrap();
        }
        panic!("no such root");
    }

    fn ask(&mut self, probe: &mut Probe<Fixture>, config: SearchConfig4) -> anyhow::Result<Timed> {
        let question = Question {
            table: &self.session,
            view: &self.view,
            pending: &self.pending,
            teams: &[],
            offered: Instant::now(),
        };
        probe.search(&mut self.net, question, config, SEED)
    }
}

/// The stand-in's twin with another way of observing.
fn observing(observing: InRollouts) -> impl Fn(&Fixture) -> Fixture {
    move |net| net.twin().observing(observing.clone())
}

fn question(probe: &Probe<Fixture>, at: usize) -> Value {
    probe.report()["questions"][at].clone()
}

/// A search ready in time is the search without a deadline, world for world,
/// played by the twin alone, and its answer is bound to the record.
#[test]
fn a_search_in_time_is_the_search_without_a_deadline() {
    let mut table = Table::new();
    let mut plain = table.net.twin();
    let before = table.net.memory.observations;
    let without = search_decision(
        &mut plain,
        &PriorDecks::new(&table.session, &table.view),
        &table.view,
        &table.pending,
        &[],
        fair(),
        SEED,
    )
    .unwrap();
    let mut probe = Probe::with_twin(AMPLE, Fixture::twin);
    let Timed::Answered(with) = table.ask(&mut probe, fair()).unwrap() else {
        panic!("the search is ready in time");
    };
    assert!(matches!(&with.outcome, SearchOutcome4::Searched(report) if report.changed));
    assert_eq!(with.outcome, without.outcome);
    assert_eq!(with.answer.action, without.answer.action);
    assert_eq!(with.answer.provenance, without.answer.provenance);
    assert_eq!(
        table.net.memory, plain.memory,
        "the seat ends where it would"
    );
    assert_eq!(
        table.net.memory.observations, before,
        "no world is observed by the seat"
    );
    assert_eq!(
        EvaluationPlayer::coverage(&table.net).counts,
        EvaluationPlayer::coverage(&plain).counts,
        "counted as searched once"
    );

    let sequence = table.session.decision_seq();
    table
        .session
        .act_slice(ME, with.answer.action.clone(), 0)
        .unwrap();
    probe.applied(sequence, true);
    probe.finish(&table.session.take_record()).unwrap();
    assert_eq!(probe.dispatched, 1);
    let timed = question(&probe, 0);
    assert_eq!(timed["status"], "ready");
    assert_eq!(timed["searched"], true);
    assert_eq!(timed["changed"], true);
    assert_eq!(timed["accepted"], true);
    assert!(timed["record_input_n"].is_u64());
    assert!(timed["worker_finish_ms"].is_f64() && timed["apply_ms"].is_f64());
}

/// A measuring search under the deadline plays the policy's own answer, and
/// nothing counts the decision as searched but the probe.
#[test]
fn a_measurement_under_the_deadline_keeps_the_policys_provenance() {
    let mut table = Table::new();
    let mut probe = Probe::with_twin(AMPLE, Fixture::twin);
    let measure = fair().with_selection(Selection4::Measure).unwrap();
    let Timed::Answered(measured) = table.ask(&mut probe, measure).unwrap() else {
        panic!("the search is ready in time");
    };
    assert!(matches!(&measured.outcome, SearchOutcome4::Searched(report) if !report.changed));
    assert_eq!(measured.answer.action, PlayerAction::PassPriority);
    assert_eq!(measured.answer.provenance.source, SourceKind4::Net);
    let searched: u64 = EvaluationPlayer::coverage(&table.net)
        .counts
        .iter()
        .map(|row| row[SourceKind4::Search as usize])
        .sum();
    assert_eq!(searched, 0);
    assert_eq!(question(&probe, 0)["searched"], true);
}

/// Whatever is not ready is the policy's answer, and says why.
#[test]
fn what_is_not_ready_in_time_is_the_policys_and_says_why() {
    let mut table = Table::new();
    let mut late = Probe::with_twin(Duration::ZERO, Fixture::twin);
    let Timed::Missed(answer, missed) = table.ask(&mut late, fair()).unwrap() else {
        panic!("no time at all");
    };
    assert_eq!(answer.action, PlayerAction::PassPriority);
    assert_eq!((missed, missed.key()), (Missed::OwnAnswer, "f_late"));
    assert_eq!(late.dispatched, 0);
    assert_eq!(question(&late, 0)["status"], "f_late");

    // A search that waits is answered without at the deadline, and the next
    // question finds its worker busy.
    let (release, waiting) = mpsc::channel();
    let waiting = InRollouts::Waits(Arc::new(Mutex::new(waiting)));
    let mut probe = Probe::with_twin(Duration::from_secs(5), observing(waiting));
    for (expected, status) in [(Missed::Search, "deadline"), (Missed::Busy, "busy")] {
        let Timed::Missed(answer, missed) = table.ask(&mut probe, fair()).unwrap() else {
            panic!("the search waits");
        };
        assert_eq!(answer.action, PlayerAction::PassPriority);
        assert_eq!((missed, missed.key()), (expected, status));
        // Both questions are the same one: the table did not move.
        probe.applied(table.session.decision_seq(), false);
    }
    assert_eq!(probe.dispatched, 1);
    drop(release);
    probe.finish(&table.session.take_record()).unwrap();
    let (searched, busy) = (question(&probe, 0), question(&probe, 1));
    assert!(searched["worker_finish_ms"].as_f64() > searched["reply_ms"].as_f64());
    assert!(searched["dispatch_ms"].is_f64());
    assert_eq!(busy["status"], "busy");
    assert!(busy["worker_finish_ms"].is_null() && busy["dispatch_ms"].is_null());
}

/// A search that fails ends the run, whether its model or its thread does.
#[test]
fn a_failed_search_ends_the_run() {
    for (failure, says) in [
        (InRollouts::Fails, "the stand-in's model failed"),
        (InRollouts::Panics, "panicked: the stand-in panicked"),
    ] {
        let mut table = Table::new();
        let mut probe = Probe::with_twin(AMPLE, observing(failure));
        let error = table.ask(&mut probe, fair()).err().unwrap();
        assert!(format!("{error:#}").contains(says), "{error:#}");
        probe.finish(&table.session.take_record()).unwrap();
    }
}

/// A refused answer is the house's to repair, as without a deadline: the
/// question says so and has no recorded input. An answer that never reached
/// the table, or that the record does not hold, is refused.
#[test]
fn a_refused_answer_is_counted_and_an_unbound_one_is_an_error() {
    let mut table = Table::new();
    let mut probe = Probe::with_twin(AMPLE, Fixture::twin);
    table.ask(&mut probe, fair()).unwrap();
    let sequence = table.session.decision_seq();
    assert!(
        table
            .session
            .act_slice(ME, PlayerAction::DeclareAttackers { attackers: vec![] }, 0)
            .is_err()
    );
    probe.applied(sequence, false);
    let house = table.session.house_action(ME).unwrap();
    table.session.act_slice(ME, house, 0).unwrap();
    probe.finish(&table.session.take_record()).unwrap();
    let refused = question(&probe, 0);
    assert_eq!(refused["accepted"], false);
    assert!(refused["record_input_n"].is_null());

    let mut table = Table::new();
    let mut probe = Probe::with_twin(AMPLE, Fixture::twin);
    table.ask(&mut probe, fair()).unwrap();
    let error = probe.finish(&table.session.take_record()).err().unwrap();
    assert!(error.to_string().contains("no accepted record input"));
}

/// The run names the export that answers at the seat, which is not the one
/// its worlds are played on.
#[test]
fn the_run_names_the_export_that_answers_and_its_time() {
    let identity = ModelIdentity {
        model_family: "v5",
        model_version: 5,
        encoder_version: 10,
        global_feat_width: None,
        critic_semantics: None,
        architecture: None,
        onnx_sha256: "a".repeat(64),
        depth_graph_sha256: BTreeMap::new(),
        metadata_sha256: "d".repeat(64),
        source_checkpoint_sha256: Some("e".repeat(64)),
        precision: None,
    };
    let said = metadata(&identity, true);
    assert_eq!(said["fp32_cpu_model"], json!(identity));
    assert_eq!(said["use_search"], true);
    assert_eq!(said["budget_ms"], 5_000.0);
    assert_eq!(said["cpu_intra_threads"], 1);
    assert_eq!(metadata(&identity, false)["use_search"], false);
}

/// The worlds prepared are the fair ones a search forks itself, as many
/// copies of each as it takes, and none past the deadline.
#[test]
fn the_worlds_prepared_are_the_fair_worlds() {
    let table = Table::new();
    let until = Instant::now() + AMPLE;
    let worlds = FairWorlds::prepare(&table.session, &table.view, fair(), SEED, until).unwrap();
    assert_eq!(worlds.decision_seq(), table.session.decision_seq());
    let prior = PriorDecks::new(&table.session, &table.view);
    for particle in 0..2 {
        let seed = world_seed(SEED, table.session.decision_seq(), particle);
        let fair = prior.fork(ME, seed).unwrap().snapshot_hash();
        // Three candidates a world, and three more tries of the first one.
        let copies = if particle == 0 { 6 } else { 3 };
        assert_eq!(worlds.by_seed[&seed].borrow().len(), copies);
        for _ in 0..copies {
            assert_eq!(worlds.fork(ME, seed).unwrap().snapshot_hash(), fair);
        }
    }
    assert_eq!(
        FairWorlds::prepare(&table.session, &table.view, fair(), SEED, Instant::now()).err(),
        Some(Unprepared::Late)
    );
    // Declarations are compared in their own number.
    let combat = fair().with_combat(5).unwrap();
    let worlds = FairWorlds::prepare(&table.session, &table.view, combat, SEED, until).unwrap();
    let copies: Vec<usize> = worlds.by_seed.values().map(|w| w.borrow().len()).collect();
    assert_eq!(copies.iter().sum::<usize>(), 15);
}
