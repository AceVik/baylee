//! `--search` through the evaluator's own game loop, with a stand-in for the
//! net. Engine-backed plumbing without an ONNX session: NOT evidence about
//! playing strength.
use std::collections::BTreeMap;

use super::*;
use crate::{Args, CoverageMatrix4, collector, play_one_table, search_config, search_refusal};
use baylee_ai::HeuristicAgent;
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_core::types::TypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_train::convert4::RolloutStepSidecar4;
use baylee_train::features4::{
    HeadKind4, PendingKind4, SourceKind4, StepProvenance4, TeacherTarget4,
};
use baylee_train::netplay4::search::{
    KnownDecks, SearchPlayer4, SearchedAnswer4, Selection4, ViewContext4, search_decision,
};
use baylee_train::netplay4::{RootDecision4, rank_single_choice};
use clap::Parser as _;

/// What the stand-in changes while it plays.
#[derive(Clone, Debug, Default, PartialEq)]
struct Memory {
    observed: BTreeMap<u8, f32>,
    observations: u64,
    notes: Vec<PlayerAction>,
    sampling: u64,
}

/// The steady house as a net: `passive` passes at every priority instead,
/// and `critic` is its value. When it `lends`, its rollout players are
/// copies of it.
struct Fixture {
    memory: Memory,
    coverage: CoverageMatrix4,
    passive: bool,
    critic: fn(&PlayerView) -> f32,
    lends: bool,
}

impl Fixture {
    fn new(passive: bool, critic: fn(&PlayerView) -> f32) -> Self {
        Self {
            memory: Memory::default(),
            coverage: CoverageMatrix4::default(),
            passive,
            critic,
            lends: false,
        }
    }

    fn decision(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4 {
        let state = baylee_train::features7::encode_state7(view, Some(pending), None, teams);
        let menu = baylee_train::features7::encode_candidates7_grouped(&state, view, pending);
        let policy = menu.head == HeadKind4::SingleChoice && menu.candidates.len() > 1
            || menu.head != HeadKind4::SingleChoice;
        let action = if self.passive && matches!(pending, Pending::Priority { .. }) {
            PlayerAction::PassPriority
        } else {
            HeuristicAgent::new(AIProfile::STEADY)
                .with_teams(teams.to_vec())
                .with_seed(self.memory.sampling)
                .act(view, pending)
        };
        let source = if policy {
            SourceKind4::Net
        } else {
            SourceKind4::Forced
        };
        self.coverage.record(PendingKind4::of(pending), source);
        RootDecision4 {
            answer: Answer4 {
                action,
                value: 0.5,
                target: TeacherTarget4::default(),
                provenance: StepProvenance4 {
                    source,
                    temperature: 1.0,
                    thinking_steps: 2,
                    has_logprob: policy,
                    behavior_logprob: if policy { -0.125 } else { 0.0 },
                },
            },
            ranking: (menu.head == HeadKind4::SingleChoice && menu.candidates.len() > 1).then(
                || rank_single_choice(&menu, &vec![0.0; menu.candidates.len()], view, pending),
            ),
            notes_answer: policy,
        }
    }
}

impl SearchPlayer4 for Fixture {
    type Snapshot = (Memory, CoverageMatrix4);
    fn snapshot(&self) -> Self::Snapshot {
        (self.memory.clone(), self.coverage.clone())
    }
    fn restore(&mut self, snapshot: &Self::Snapshot) {
        self.memory.clone_from(&snapshot.0);
        self.coverage.clone_from(&snapshot.1);
    }
    fn view_context(&self) -> ViewContext4 {
        ViewContext4::Choice
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        self.memory.observations += 1;
        self.memory
            .observed
            .insert(view.seat.get(), (self.critic)(view));
        Ok(())
    }
    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        self.memory.observed.get(&seat.get()).copied()
    }
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4 {
        self.decision(view, pending, teams)
    }
    fn conclude(&mut self, _: &PlayerView, decision: &RootDecision4, action: &PlayerAction) {
        if decision.notes_answer {
            self.memory.notes.push(action.clone());
        }
    }
    fn answer(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Answer4 {
        let decision = self.decision(view, pending, teams);
        SearchPlayer4::conclude(self, view, &decision, &decision.answer.action);
        decision.answer
    }
    fn reseed_sampling(&mut self, seed: u64) {
        self.memory.sampling = seed;
    }
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4) {
        let row = &mut self.coverage.counts[PendingKind4::of(pending) as usize];
        row[from as usize] -= 1;
        row[SourceKind4::Search as usize] += 1;
    }
    fn lends_rollout_players(&self) -> bool {
        self.lends
    }
    fn rollout_player(&self, snapshot: &Self::Snapshot) -> Option<Self> {
        self.lends.then(|| Self {
            memory: snapshot.0.clone(),
            coverage: snapshot.1.clone(),
            ..*self
        })
    }
}

impl EvaluationPlayer for Fixture {
    fn reseed(&mut self, _: u64) {}
    fn init_game(&mut self, _: &baylee_core::preset::GamePreset) {
        self.memory = Memory::default();
    }
    fn uses_context(&self) -> bool {
        true
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        SearchPlayer4::observe(self, view)
    }
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<Answer4>> {
        Ok(Some(SearchPlayer4::answer(self, view, pending, teams)))
    }
    fn coverage(&self) -> &CoverageMatrix4 {
        &self.coverage
    }
    fn search(
        &mut self,
        table: &Session,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        config: SearchConfig4,
        seed: u64,
    ) -> Option<anyhow::Result<SearchedAnswer4>> {
        Some(search_decision(
            self,
            &KnownDecks(table),
            view,
            pending,
            teams,
            config,
            seed,
        ))
    }
}

fn flat(_: &PlayerView) -> f32 {
    0.5
}

/// Likes the seat's lands.
fn lands(view: &PlayerView) -> f32 {
    let mine = view
        .battlefield
        .iter()
        .filter(|o| o.controller == view.seat && o.types.contains(TypeSet::LAND))
        .count();
    (0.2 + 0.05 * f32::from(u16::try_from(mine).unwrap_or(u16::MAX))).min(1.0)
}

type Played = (f32, u64, Vec<u8>, Vec<RolloutStepSidecar4>);

/// One recorded deal of Victory against Victory, the fixture in seat 0.
fn play(player: &mut Fixture, search: Option<&mut GameSearch>) -> Played {
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let outcome = play_one_table(
        "lookahead-test",
        0,
        0,
        12,
        34,
        &[&deck, &deck],
        &[AIProfile::STEADY; 2],
        &[],
        &[None; 2],
        Some(player),
        true,
        5000,
        true,
        None,
        None,
        search,
    )
    .unwrap();
    assert!(!outcome.1, "the fixture finishes its game");
    (outcome.0, outcome.4, outcome.7, outcome.9)
}

fn config(k: usize, p: usize, h: usize) -> SearchConfig4 {
    SearchConfig4::new(k, p, h).unwrap()
}

/// What the search's own machinery costs without a model: forks, house
/// rollouts and the stand-in's answers, per searched decision. Prints;
/// run optimised (`--profile ci-release`) with `--ignored --nocapture`.
#[test]
#[ignore = "a measurement, not a check"]
#[allow(clippy::cast_precision_loss, reason = "Report statistics")]
fn measure_the_search_machinery_without_a_model() {
    for (k, p, h) in [(1, 1, 0), (4, 2, 0), (4, 2, 8), (4, 2, 32), (8, 4, 32)] {
        let mut player = Fixture::new(false, flat);
        let mut search = GameSearch::new(config(k, p, h), 0);
        let started = Instant::now();
        play(&mut player, Some(&mut search));
        let seconds = started.elapsed().as_secs_f64();
        let tally = &search.tally;
        let searched_ms: f64 = tally.ms.iter().sum();
        println!(
            "K,P,H={k},{p},{h}: {} decisions in {seconds:.2}s ({:.1}/s), {} searched, \
             {:.2} ms per searched decision, {} rollouts, {:.1} steps per rollout, {:?}",
            tally.decisions,
            tally.decisions as f64 / seconds,
            tally.searched,
            searched_ms / tally.searched.max(1) as f64,
            tally.rollouts,
            tally.rollout_steps as f64 / tally.rollouts.max(1) as f64,
            tally.leaves,
        );
    }
}

#[test]
fn one_candidate_records_the_game_the_policy_plays_and_it_converts() {
    let mut plain = Fixture::new(false, flat);
    let mut searched = Fixture::new(false, flat);
    let mut search = GameSearch::new(config(1, 2, 6), 0);
    let without = play(&mut plain, None);
    let with = play(&mut searched, Some(&mut search));

    assert_eq!(without.0, with.0, "the same outcome");
    assert_eq!(with.1, 0, "no answer refused");
    assert_eq!(
        without.2, with.2,
        "the record is the policy's game, byte for byte"
    );
    assert_eq!(plain.memory, searched.memory, "every world undone");
    let tally = &search.tally;
    assert!(tally.searched > 5, "{tally:?}");
    assert_eq!(tally.changed, 0);
    assert_eq!(tally.rounds, 2 * tally.searched);
    assert!(tally.skipped.get("not_priority").copied().unwrap_or(0) > 0);
    assert_eq!(
        tally.decisions,
        tally.searched + tally.skipped.values().sum::<u64>() + tally.refused.values().sum::<u64>()
    );

    assert_eq!(without.3.len(), with.3.len());
    let searched_steps: Vec<_> = with
        .3
        .iter()
        .filter(|entry| entry.provenance.source == SourceKind4::Search)
        .collect();
    assert_eq!(searched_steps.len() as u64, tally.searched);
    assert!(
        searched_steps
            .iter()
            .all(|entry| !entry.provenance.has_logprob && entry.provenance.behavior_logprob == 0.0)
    );
    assert_eq!(search.events.len() as u64, tally.searched);
    crate::diagnostics::bind_fallback_events(&mut search.events, &with.2)
        .expect("every searched answer is the recorded input at its offset");
    assert!(
        search
            .events
            .iter()
            .all(|event| event["record_input_n"].is_u64()
                && event["chosen"] == 0
                && event["candidates"][0] == event["action"])
    );
    let search_coverage: u64 = searched
        .coverage
        .counts
        .iter()
        .map(|row| row[SourceKind4::Search as usize])
        .sum();
    assert_eq!(search_coverage, tally.searched);

    // The record replays through the converter with the search's provenance.
    let mut converted = Vec::new();
    baylee_train::convert4::stream_game4(&with.2, 1, true, Some(&with.3), |step| {
        converted.push(step);
        Ok(())
    })
    .expect("a searched record converts");
    let searched_frames: Vec<_> = converted
        .iter()
        .filter(|step| step.is_decision && step.provenance.source == SourceKind4::Search)
        .collect();
    assert!(!searched_frames.is_empty());
    assert!(
        searched_frames
            .iter()
            .all(|step| !step.provenance.has_logprob)
    );
}

/// A passive fixture's game, searched at `K, 1, 0` under `selection` with a
/// critic that likes lands, and its tally.
fn passive_land_game(selection: Selection4) -> SearchTally {
    let mut player = Fixture::new(true, lands);
    let config = config(4, 1, 0).with_selection(selection).unwrap();
    let mut search = GameSearch::new(config, 0);
    let played = play(&mut player, Some(&mut search));
    assert_eq!(played.1, 0, "no searched answer refused");
    assert_eq!(search.tally.rollout_steps, 0, "H = 0");
    assert_eq!(search.tally.leaves.immediate, search.tally.rollouts);
    search.tally
}

#[test]
fn a_critic_that_likes_lands_changes_answers_and_the_game_still_plays() {
    let tally = passive_land_game(Selection4::Mean);
    assert!(tally.changed > 0, "{tally:?}");
    assert_eq!(tally.changes.values().sum::<u64>(), tally.changed);
    assert!(
        tally
            .changes
            .keys()
            .all(|key| key.starts_with("PassPriority->")),
        "the fixture's own answer is always a pass: {:?}",
        tally.changes
    );
    assert!(tally.changes.contains_key("PassPriority->PlayLand"));
    assert_eq!(
        tally.searched_by_kind.get("PassPriority"),
        Some(&tally.searched)
    );

    let paired = passive_land_game(Selection4::Paired {
        margin: 0.01,
        z: 0.0,
    });
    assert!(paired.changes.contains_key("PassPriority->PlayLand"));
    let strict = passive_land_game(Selection4::Paired {
        margin: 0.06,
        z: 0.0,
    });
    assert_eq!(strict.changed, 0, "a land is worth 0.05 here");
    assert_eq!(
        passive_land_game(Selection4::Paired {
            margin: 0.05,
            z: 1.0
        })
        .changed,
        0,
        "one world clears no standard error"
    );
}

/// A game whose searches run their worlds at once, on rollout players of
/// the fixture's own, records the game they record one by one: the same
/// bytes, the same counts, every searched decision counted as parallel.
#[test]
fn parallel_rollouts_record_the_serial_game() {
    let run = |lends: bool| {
        let mut player = Fixture::new(false, lands);
        player.lends = lends;
        let config = config(3, 2, 4).with_threads(4).unwrap();
        let mut search = GameSearch::new(config, 6);
        let played = play(&mut player, Some(&mut search));
        (played, search.tally, player.memory)
    };
    let (serial, serial_tally, serial_memory) = run(false);
    let (parallel, mut parallel_tally, parallel_memory) = run(true);
    assert_eq!(serial_tally.parallel, 0);
    assert!(parallel_tally.searched > 0);
    assert_eq!(parallel_tally.parallel, parallel_tally.searched);
    parallel_tally.parallel = 0;
    parallel_tally.ms.clone_from(&serial_tally.ms);
    assert_eq!(parallel_tally, serial_tally);
    assert_eq!(parallel.0, serial.0, "the same outcome");
    assert_eq!(parallel.2, serial.2, "the same record, byte for byte");
    assert_eq!(parallel_memory, serial_memory);
}

#[test]
fn the_budget_caps_searched_decisions_per_game() {
    let mut player = Fixture::new(false, flat);
    let mut search = GameSearch::new(config(2, 1, 2), 3);
    play(&mut player, Some(&mut search));
    assert_eq!(search.tally.searched, 3);
    assert!(search.tally.skipped.get("budget").copied().unwrap_or(0) > 0);
    let report = search.tally.report();
    assert_eq!(report["searched"], 3);
    assert!(report["searched_ms"]["max"].as_f64().is_some());
}

#[test]
fn search_is_refused_where_it_cannot_run() {
    let args = |extra: &[&str]| {
        let mut argv = vec!["eval4", "--model", "m", "--out", "o"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv).unwrap()
    };
    let refusal = |extra: &[&str]| {
        let args = args(extra);
        let contract = collector::Contract::new(&args, None).unwrap();
        search_refusal(&args, &contract, None)
    };
    assert_eq!(refusal(&[]), None);
    assert_eq!(refusal(&["--search", "4,2,0"]), None);
    assert!(refusal(&["--search", "4,2,0", "--legacy-v3"]).is_some());
    assert!(
        Args::try_parse_from(["eval4", "--model", "m", "--out", "o", "--search", "0,2,0"]).is_err()
    );
    assert!(
        Args::try_parse_from(["eval4", "--model", "m", "--out", "o", "--search", "4,2"]).is_err()
    );

    let selected = |extra: &[&str]| search_config(&args(extra));
    assert_eq!(selected(&[]).unwrap(), None);
    assert_eq!(
        selected(&["--search", "4,2,3"])
            .unwrap()
            .unwrap()
            .selection(),
        Selection4::Prior { c: 1.0 },
        "the policy's prior at weight 1 unless told otherwise"
    );
    assert_eq!(
        selected(&["--search", "4,2,3", "--search-select", "paired"])
            .unwrap()
            .unwrap()
            .selection(),
        Selection4::Paired {
            margin: 0.05,
            z: 1.0
        },
        "paired keeps its margin and one standard error"
    );
    let prior = selected(&[
        "--search",
        "4,2,3",
        "--search-select",
        "prior",
        "--search-prior-c",
        "2",
    ])
    .unwrap()
    .unwrap();
    assert_eq!(prior.selection(), Selection4::Prior { c: 2.0 });
    assert_eq!(prior.metadata()["selection"]["c"], 2.0);
    assert!(
        selected(&["--search", "4,2,3", "--search-margin=-0.1"]).is_err(),
        "a wrong value is refused whichever rule is chosen"
    );
    assert!(selected(&["--search", "4,2,3", "--search-prior-c=-1"]).is_err());

    let threads = |extra: &[&str]| selected(extra).unwrap().unwrap().threads();
    assert_eq!(
        threads(&["--search", "4,2,3", "--search-threads", "8"]),
        1,
        "without shared batches the rollouts run one by one"
    );
    assert_eq!(
        threads(&["--search", "4,2,3", "--inference-batch", "64"]),
        SearchConfig4::DEFAULT_THREADS
    );
    assert_eq!(
        threads(&[
            "--search",
            "4,2,3",
            "--inference-batch",
            "64",
            "--search-threads",
            "8"
        ]),
        8
    );
    assert!(selected(&["--search", "4,2,3", "--search-threads", "0"]).is_err());
}
