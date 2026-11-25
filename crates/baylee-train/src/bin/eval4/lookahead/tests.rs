//! `--search` through the evaluator's own game loop, with a stand-in for the
//! net. Engine-backed plumbing without an ONNX session: NOT evidence about
//! playing strength.
use std::collections::BTreeMap;

use super::*;
use crate::{
    Args, CoverageMatrix4, collector, play_one_table, search_config, search_metadata,
    search_refusal,
};
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
    KnownDecks, PriorDecks, ScriptFollow4, SearchPlayer4, SearchedAnswer4, Selection4,
    ViewContext4, follow_script, search_decision_avoiding,
};
use baylee_train::netplay4::{RootDecision4, rank_single_choice};
use clap::Parser as _;

/// What the stand-in changes while it plays.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Memory {
    observed: BTreeMap<u8, f32>,
    pub(crate) observations: u64,
    pub(crate) notes: Vec<PlayerAction>,
    sampling: u64,
}

/// What the stand-in does when it observes in a rollout.
#[derive(Clone)]
pub(crate) enum InRollouts {
    /// Observes, as at the table.
    Observes,
    /// Fails, as a model does.
    Fails,
    /// Panics.
    Panics,
    /// Waits until the test lets go of the other end, then observes.
    Waits(std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<()>>>),
}

/// The steady house as a net: `passive` passes at every priority instead,
/// and `critic` is its value. When it `lends`, its rollout players are
/// copies of it.
pub(crate) struct Fixture {
    pub(crate) memory: Memory,
    coverage: CoverageMatrix4,
    passive: bool,
    critic: fn(&PlayerView) -> f32,
    lends: bool,
    /// What it and its rollout players do at an observation.
    observing: InRollouts,
}

impl Fixture {
    pub(crate) fn new(passive: bool, critic: fn(&PlayerView) -> f32) -> Self {
        Self {
            memory: Memory::default(),
            coverage: CoverageMatrix4::default(),
            passive,
            critic,
            lends: false,
            observing: InRollouts::Observes,
        }
    }

    /// The same stand-in, lending rollout players.
    pub(crate) fn lending(self) -> Self {
        Self {
            lends: true,
            ..self
        }
    }

    /// The same stand-in, observing otherwise.
    pub(crate) fn observing(self, observing: InRollouts) -> Self {
        Self { observing, ..self }
    }

    /// The stand-in again, in its own state.
    pub(crate) fn twin(&self) -> Self {
        Self {
            memory: self.memory.clone(),
            coverage: self.coverage.clone(),
            observing: self.observing.clone(),
            ..*self
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
        match &self.observing {
            InRollouts::Observes => {}
            InRollouts::Fails => anyhow::bail!("the stand-in's model failed"),
            InRollouts::Panics => panic!("the stand-in panicked"),
            InRollouts::Waits(until) => {
                let _ = until.lock().unwrap().recv();
            }
        }
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
            ..self.twin()
        })
    }
    fn carry_table_notes(state: &mut Self::Snapshot, decided: &Self::Snapshot) {
        state.0.notes.clone_from(&decided.0.notes);
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
        taken_back: &[PlayerAction],
    ) -> Option<anyhow::Result<SearchedAnswer4>> {
        if config.fair() {
            let fair = PriorDecks::new(table, view);
            return Some(search_decision_avoiding(
                self, &fair, view, pending, teams, config, seed, taken_back,
            ));
        }
        Some(search_decision_avoiding(
            self,
            &KnownDecks(table),
            view,
            pending,
            teams,
            config,
            seed,
            taken_back,
        ))
    }
    fn follow_script(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        script: &mut baylee_train::netplay4::Script4,
    ) -> Option<ScriptFollow4> {
        Some(follow_script(self, view, pending, teams, script))
    }
}

pub(crate) fn flat(_: &PlayerView) -> f32 {
    0.5
}

/// Likes the seat's lands.
pub(crate) fn lands(view: &PlayerView) -> f32 {
    let mine = view
        .battlefield
        .iter()
        .filter(|o| o.controller == view.seat && o.types.contains(TypeSet::LAND))
        .count();
    (0.2 + 0.05 * f32::from(u16::try_from(mine).unwrap_or(u16::MAX))).min(1.0)
}

/// What a deal came to: the utility, the answers refused, the record and
/// its sidecar, the net's own answers and how many of them were timed.
pub(crate) struct Deal {
    pub(crate) utility: f32,
    pub(crate) refused: u64,
    pub(crate) record: Vec<u8>,
    pub(crate) sidecar: Vec<RolloutStepSidecar4>,
    pub(crate) net_answers: u64,
    pub(crate) timed: usize,
}

/// One recorded deal of Victory against Victory, `player` in seat 0.
pub(crate) fn play(player: &mut dyn EvaluationPlayer, search: Option<&mut GameSearch>) -> Deal {
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
        false,
        5000,
        true,
        None,
        None,
        search,
        crate::DecisionLogs::default(),
        None,
    )
    .unwrap();
    assert!(!outcome.1, "the fixture finishes its game");
    Deal {
        utility: outcome.0,
        refused: outcome.4,
        record: outcome.7,
        sidecar: outcome.9,
        net_answers: outcome.2,
        timed: outcome.6.len(),
    }
}

/// `--house-as-seat` through the evaluator's loop: the house chair is taken
/// over and answered from the socket's view (`house_seat`), and the deal
/// plays to its end with no answer refused.
#[test]
fn a_seated_house_plays_a_deal_to_its_end() {
    let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
    let mut player = Fixture::new(false, flat);
    let outcome = play_one_table(
        "seated-house-test",
        0,
        0,
        12,
        34,
        &[&deck, &deck],
        &[AIProfile::SHARP; 2],
        &[],
        &[None; 2],
        Some(&mut player),
        true,
        true,
        5000,
        false,
        None,
        None,
        None,
        crate::DecisionLogs::default(),
        None,
    )
    .unwrap();
    assert!(!outcome.1, "the deal ends before the answer cap");
    assert_eq!((outcome.3, outcome.4), (0, 0), "no fallback, no refusal");
}

pub(crate) fn config(k: usize, p: usize, h: usize) -> SearchConfig4 {
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

    assert_eq!(without.utility, with.utility, "the same outcome");
    assert_eq!(with.refused, 0, "no answer refused");
    assert_eq!(
        without.record, with.record,
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

    assert_eq!(without.sidecar.len(), with.sidecar.len());
    let searched_steps: Vec<_> = with
        .sidecar
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
    crate::diagnostics::bind_fallback_events(&mut search.events, &with.record)
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
    baylee_train::convert4::stream_game4(&with.record, 1, true, Some(&with.sidecar), |step| {
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
    assert_eq!(played.refused, 0, "no searched answer refused");
    assert_eq!(search.tally.rollout_steps, 0, "H = 0");
    // Nothing plays on at H = 0: every leaf is the candidate's own
    // position, scored at once, or one the candidate itself ended (the
    // game over, the cast taken back). Which of those a game meets is the
    // house opponent's to decide, and it moves with the house.
    let leaves = search.tally.leaves;
    assert_eq!(
        leaves.settled + leaves.horizon + leaves.turn_end,
        0,
        "{leaves:?}"
    );
    assert_eq!(
        leaves.immediate + leaves.terminal + leaves.stopped,
        search.tally.rollouts
    );
    assert!(leaves.immediate > 0);
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

/// Likes what the seat has on the battlefield, whatever it is.
fn permanents(view: &PlayerView) -> f32 {
    let mine = view
        .battlefield
        .iter()
        .filter(|o| o.controller == view.seat)
        .count();
    (0.1 + 0.05 * f32::from(u16::try_from(mine).unwrap_or(u16::MAX))).min(1.0)
}

/// The fair search through the evaluator's loop: a passive fixture whose
/// critic likes permanents is given lands and planned casts by the search,
/// a chosen planned cast is followed tap by tap until the spell is sent,
/// the table refuses none of it, and every world ends at its turn boundary
/// or with the game.
#[test]
fn a_fair_search_follows_its_planned_casts_through_the_game_loop() {
    let mut player = Fixture::new(true, permanents);
    let config = config(6, 2, 64)
        .with_fair(true)
        .with_macros(true)
        .with_neural_opponent(true)
        .with_turns(2)
        .unwrap()
        .with_selection(Selection4::Mean)
        .unwrap();
    let mut search = GameSearch::new(config, 40);
    let played = play(&mut player, Some(&mut search));
    let tally = &search.tally;
    assert_eq!(played.refused, 0, "no searched or scripted answer refused");
    assert!(tally.scripts > 0, "{tally:?}");
    assert!(tally.scripts_done > 0, "{tally:?}");
    assert!(tally.scripts_done + tally.scripts_broken <= tally.scripts);
    assert!(
        tally.changes.contains_key("PassPriority->PlannedCastSpell"),
        "{:?}",
        tally.changes
    );
    let leaves = tally.leaves;
    assert!(leaves.boundary > 0, "{leaves:?}");
    assert_eq!(leaves.settled + leaves.turn_end + leaves.immediate, 0);
    assert_eq!(config.contract(), "belief_lookahead_v5");
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
    assert_eq!(parallel.utility, serial.utility, "the same outcome");
    assert_eq!(
        parallel.record, serial.record,
        "the same record, byte for byte"
    );
    assert_eq!(parallel_memory, serial_memory);
}

/// A measuring search plays and records the unsearched game, and its time is
/// in none of the decisions timed: the searched ones are left out, whoever's
/// answer they played, so what is timed compares with a game without it.
#[test]
fn a_measuring_search_records_and_times_the_unsearched_game() {
    let mut plain = Fixture::new(true, lands);
    let without = play(&mut plain, None);
    assert_eq!(without.timed as u64, without.net_answers);

    let mut measured = Fixture::new(true, lands);
    let config = config(4, 1, 0).with_selection(Selection4::Measure).unwrap();
    let mut search = GameSearch::new(config, 0);
    let with = play(&mut measured, Some(&mut search));
    assert!(search.tally.searched > 5 && search.tally.changed == 0);
    assert!(
        search
            .events
            .iter()
            .any(|event| event["diffs"].as_array().unwrap().len() > 1)
    );
    assert_eq!(with.record, without.record, "byte for byte");
    let provenance = |deal: &Deal| -> Vec<_> {
        let steps = deal.sidecar.iter();
        steps.map(|step| step.provenance).collect()
    };
    assert_eq!(provenance(&with), provenance(&without));
    assert_eq!(with.net_answers, without.net_answers);
    assert_eq!(with.timed as u64 + search.tally.searched, with.net_answers);
    assert_eq!(search.tally.ms.len() as u64, search.tally.searched);
}

/// A refused answer marks the event of its own question alone, whoever's
/// answer it was; only a refused answer of the search's ends what the search
/// chose beside it.
#[test]
fn a_refused_answer_marks_its_own_event() {
    let mut search = GameSearch::new(config(2, 1, 0), 0);
    search.events = vec![
        json!({"decision_seq": 4, "accepted": true}),
        json!({"decision_seq": 7, "accepted": true}),
    ];
    let target = (PlayerId::new(0), PlayerAction::PassPriority);
    search.follow = Some(target.clone());
    search.refused(5, SourceKind4::Net);
    assert_eq!(search.events[1]["accepted"], true, "another question's");
    search.refused(7, SourceKind4::Net);
    assert_eq!(search.events[0]["accepted"], true);
    assert_eq!(search.events[1]["accepted"], false);
    assert_eq!(
        search.follow,
        Some(target.clone()),
        "not the search's answer"
    );
    assert_eq!(search.tally.follow_lost, 0);

    search.refused(7, SourceKind4::Search);
    assert_eq!((search.follow.take(), search.tally.follow_lost), (None, 1));
    // A refused target's own question has no event: its cast's stays.
    search.events[1]["accepted"] = json!(true);
    search.answered_follow = true;
    search.follow = Some(target);
    search.refused(9, SourceKind4::Search);
    assert_eq!(search.events[1]["accepted"], true);
    assert_eq!(search.tally.follow_lost, 1, "the cast's target was given");
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
    let neural = |extra: &[&str]| selected(extra).unwrap().unwrap().neural_opponent();
    assert!(
        !neural(&["--search", "4,2,3"])
            && neural(&["--search", "4,2,3", "--search-neural-opponent"])
    );
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

    let reproducible = |extra: &[&str]| {
        let args = args(extra);
        search_metadata(&args, search_config(&args).unwrap(), None, None).unwrap()["bitwise_reproducible"]
            .clone()
    };
    assert_eq!(reproducible(&["--search", "4,2,3"]), true);
    assert_eq!(
        reproducible(&["--search", "4,2,3", "--inference-batch", "64"]),
        false,
        "a batched search says it is not reproducible bit for bit"
    );
}

#[test]
fn served_calls_of_one_row_keep_a_search_reproducible() {
    let reproducible = |extra: &[&str]| {
        let mut argv = vec!["eval4", "--model", "m", "--out", "o", "--search", "4,2,3"];
        argv.extend_from_slice(extra);
        let args = Args::try_parse_from(argv).unwrap();
        search_metadata(&args, search_config(&args).unwrap(), None, None).unwrap()["bitwise_reproducible"]
            .clone()
    };
    assert_eq!(
        reproducible(&["--inference-batch", "1", "--inference-servers", "4"]),
        true,
        "calls of one row are the unbatched steps, whichever server runs them"
    );
    assert_eq!(
        reproducible(&["--inference-batch", "4", "--inference-servers", "4"]),
        false
    );
}

#[test]
fn the_stall_guard_is_asked_for_and_named() {
    let guarded = |extra: &[&str]| {
        let mut argv = vec!["eval4", "--model", "m", "--out", "o"];
        argv.extend_from_slice(extra);
        search_config(&Args::try_parse_from(argv).unwrap())
            .unwrap()
            .unwrap()
    };
    assert!(!guarded(&["--search", "4,2,3"]).stall_guard());
    let guard = guarded(&["--search", "4,2,3", "--search-stall-guard"]);
    assert!(guard.stall_guard());
    assert!(guard.metadata()["stall_guard"].is_string());
    assert!(
        guarded(&["--search", "4,2,3"]).metadata()["stall_guard"].is_null(),
        "a search without the guard says nothing of it"
    );
}
